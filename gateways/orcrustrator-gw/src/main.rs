use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
pub mod hot_reload;
use orcrustrator::{
    event::ChannelTransport,
    format::StructuredValue,
    method::HttpMethod,
    mutation::{CompensatingRollback, MutationEndpoint, MutationStep},
    relation::{RelationalEndpoint, Relationship},
    service::{Service, ServiceRegistry},
    source::Endpoint,
    CacheProvider, MemoryCache,
};
use shared::models::*;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppConfig {
    service: Arc<Service>,
    cache: Arc<MemoryCache>,
    transport: Arc<ChannelTransport>,
}

fn build_orcrustrator_service(
    reg_url: &str,
    lab_url: &str,
    pharm_url: &str,
    term_url: &str,
    bill_url: &str,
    notif_url: &str,
    cache: Arc<MemoryCache>,
    transport: Arc<ChannelTransport>,
) -> Service {
    let mut registry = ServiceRegistry::new();

    // 1. Read scenario: GET /patients/{id}/summary
    let root_ep = Endpoint::get("/api/v1/patients/{id}").base_url(reg_url);
    let mut read_ep = RelationalEndpoint::new("/patients/{id}/summary", "patient", root_ep);

    // Independent Join for Billing Account (fetched concurrently with root!)
    read_ep.add_relationship(Relationship::IndependentJoin {
        name: "billing_link".to_string(),
        endpoint: Endpoint::get("/api/v1/billing/accounts/{id}").base_url(bill_url),
        parent_match_key: "id".to_string(),
        child_match_key: "patient_id".to_string(),
        embed_as: "billing".to_string(),
        is_has_many: false,
    });

    // HasMany: Labs
    read_ep.add_relationship(Relationship::HasMany {
        name: "patient_labs".to_string(),
        endpoint: Endpoint::get("/api/v1/patients/{id}/labs").base_url(lab_url),
        foreign_param: "id".to_string(),
        parent_field: "id".to_string(),
        embed_as: "labs".to_string(),
    });

    // HasMany: Prescriptions
    read_ep.add_relationship(Relationship::HasMany {
        name: "patient_prescriptions".to_string(),
        endpoint: Endpoint::get("/api/v1/patients/{id}/prescriptions").base_url(pharm_url),
        foreign_param: "id".to_string(),
        parent_field: "id".to_string(),
        embed_as: "prescriptions".to_string(),
    });

    registry.register_read(read_ep);

    // 2. Write scenario: POST /visits
    let mut mut_ep = MutationEndpoint::new(HttpMethod::Post, "/visits");

    // Step 1: Create Encounter
    let step1 = MutationStep::new("create_encounter", Endpoint::post("/api/v1/encounters").base_url(reg_url))
        .map_body("patient_id", "patient_id")
        .map_body("provider_id", "provider_id")
        .map_body("department", "department");
    mut_ep.add_step(step1);

    // Rollback for Step 1
    let rb1 = CompensatingRollback::new("create_encounter", Endpoint::delete("/api/v1/encounters/{id}").base_url(reg_url))
        .map_path("id", "$steps.create_encounter.id");
    mut_ep.add_rollback(rb1);

    // Step 2: Reserve Stock
    let step2 = MutationStep::new("reserve_stock", Endpoint::post("/api/v1/pharmacy/reservations").base_url(pharm_url))
        .map_body("patient_id", "patient_id")
        .map_body("encounter_id", "$steps.create_encounter.id")
        .map_body("items", "medications");
    mut_ep.add_step(step2);

    // Rollback for Step 2
    let rb2 = CompensatingRollback::new("reserve_stock", Endpoint::delete("/api/v1/pharmacy/reservations/{id}").base_url(pharm_url))
        .map_path("id", "$steps.reserve_stock.id");
    mut_ep.add_rollback(rb2);

    // Step 3: Create Invoice
    let step3 = MutationStep::new("create_invoice", Endpoint::post("/api/v1/billing/invoices").base_url(bill_url))
        .map_body("patient_id", "patient_id")
        .map_body("encounter_id", "$steps.create_encounter.id")
        .map_body("items", "billing_items");
    mut_ep.add_step(step3);

    // Rollback for Step 3
    let rb3 = CompensatingRollback::new("create_invoice", Endpoint::post("/api/v1/billing/invoices/{id}/void").base_url(bill_url))
        .map_path("id", "$steps.create_invoice.id");
    mut_ep.add_rollback(rb3);

    // Step 4: Send Notification
    let step4 = MutationStep::new("send_notification", Endpoint::post("/api/v1/notifications").base_url(notif_url))
        .map_body("patient_id", "patient_id")
        .map_body("channel", "sms")
        .map_body("message", "Visit confirmed via Orcrustrator Saga Pipeline");
    mut_ep.add_step(step4);

    // Response mappings to match CreateVisitResponse
    mut_ep.map_response("encounter", "$steps.create_encounter");
    mut_ep.map_response("reservation", "$steps.reserve_stock");
    mut_ep.map_response("invoice", "$steps.create_invoice");
    mut_ep.map_response("notification", "$steps.send_notification");

    registry.register_mutation(mut_ep);

    Service::builder("http://localhost:8003")
        .registry(registry)
        .cache(cache)
        .transport(transport)
        .build()
}

// Read endpoint
async fn get_patient_summary(
    State(cfg): State<AppConfig>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, Response> {
    let path = format!("/patients/{}/summary", id);
    let structured = cfg
        .service
        .handle_request(HttpMethod::Get, &path, None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response())?;

    let json_val = serde_json::to_value(&structured)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response())?;

    Ok(Json(json_val))
}

// Write endpoint
async fn create_visit(
    State(cfg): State<AppConfig>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, Response> {
    let patient_id = payload.get("patient_id").and_then(|v| v.as_str()).map(|s| s.to_string());
    let structured_body = StructuredValue::from_json_value(payload);

    let res = cfg
        .service
        .handle_request(HttpMethod::Post, "/visits", Some(&structured_body))
        .await;

    // Cache invalidation mitigation identified in Phase 0 audit:
    if let Some(pid) = patient_id {
        let key = format!("GET:/patients/{}/summary", pid);
        let _ = cfg.cache.invalidate(&key).await;
    }

    match res {
        Ok(val) => {
            let json_val = serde_json::to_value(&val)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response())?;
            Ok(Json(json_val))
        }
        Err(e) => {
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": e.to_string(),
                    "source": "Orcrustrator Mutation Pipeline",
                    "saga_rollback_executed": true
                })),
            )
                .into_response())
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let reg_url = std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    let lab_url = std::env::var("LAB_URL").unwrap_or_else(|_| "http://localhost:8082".to_string());
    let pharm_url = std::env::var("PHARMACY_URL").unwrap_or_else(|_| "http://localhost:8083".to_string());
    let term_url = std::env::var("TERMINOLOGY_URL").unwrap_or_else(|_| "http://localhost:8084".to_string());
    let bill_url = std::env::var("BILLING_URL").unwrap_or_else(|_| "http://localhost:8085".to_string());
    let notif_url = std::env::var("NOTIFICATIONS_URL").unwrap_or_else(|_| "http://localhost:8086".to_string());

    let cache = Arc::new(MemoryCache::with_default_ttl(Duration::from_secs(300)));
    let transport = Arc::new(ChannelTransport::new(2048));

    let service = Arc::new(build_orcrustrator_service(
        &reg_url, &lab_url, &pharm_url, &term_url, &bill_url, &notif_url,
        cache.clone(), transport.clone(),
    ));

    let cfg = AppConfig {
        service,
        cache,
        transport,
    };

    let app = Router::new()
        .route("/patients/:id/summary", get(get_patient_summary))
        .route("/visits", post(create_visit))
        .layer(CorsLayer::permissive())
        .with_state(cfg);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8003".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Orcrustrator Gateway listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
