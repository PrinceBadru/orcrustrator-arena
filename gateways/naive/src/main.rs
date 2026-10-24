use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use reqwest::Client;
use shared::models::*;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppConfig {
    registry_url: String,
    lab_url: String,
    pharmacy_url: String,
    terminology_url: String,
    billing_url: String,
    notifications_url: String,
    client: Client,
}

impl AppConfig {
    fn from_env() -> Self {
        Self {
            registry_url: std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8081".to_string()),
            lab_url: std::env::var("LAB_URL").unwrap_or_else(|_| "http://localhost:8082".to_string()),
            pharmacy_url: std::env::var("PHARMACY_URL").unwrap_or_else(|_| "http://localhost:8083".to_string()),
            terminology_url: std::env::var("TERMINOLOGY_URL").unwrap_or_else(|_| "http://localhost:8084".to_string()),
            billing_url: std::env::var("BILLING_URL").unwrap_or_else(|_| "http://localhost:8085".to_string()),
            notifications_url: std::env::var("NOTIFICATIONS_URL").unwrap_or_else(|_| "http://localhost:8086".to_string()),
            client: Client::new(),
        }
    }
}

// Read scenario: GET /patients/:id/summary (Strictly Sequential)
async fn get_patient_summary(
    State(cfg): State<AppConfig>,
    Path(id): Path<String>,
) -> Result<Json<PatientSummaryResponse>, Response> {
    // 1. Fetch Patient
    let patient_res = cfg
        .client
        .get(format!("{}/api/v1/patients/{}", cfg.registry_url, id))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response())?;
    
    if !patient_res.status().is_success() {
        return Err((patient_res.status(), Json(serde_json::json!({"error": "Failed to fetch patient"}))).into_response());
    }
    let patient: Patient = patient_res.json().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response())?;

    // 2. Fetch Labs sequentially
    let lab_res = cfg
        .client
        .get(format!("{}/api/v1/patients/{}/labs", cfg.lab_url, id))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response())?;
    
    let mut labs: Vec<LabResult> = lab_res.json().await.unwrap_or_default();

    // 3. Enrich each lab result sequentially
    for lab in labs.iter_mut() {
        if let Ok(c_res) = cfg.client.get(format!("{}/api/v1/terminology/{}", cfg.terminology_url, lab.concept_id)).send().await {
            if let Ok(concept) = c_res.json::<TerminologyConcept>().await {
                lab.concept = Some(concept);
            }
        }
    }

    // 4. Fetch Prescriptions sequentially
    let rx_res = cfg
        .client
        .get(format!("{}/api/v1/patients/{}/prescriptions", cfg.pharmacy_url, id))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response())?;
    
    let mut prescriptions: Vec<Prescription> = rx_res.json().await.unwrap_or_default();

    // 5. Enrich each prescription sequentially
    for rx in prescriptions.iter_mut() {
        if let Ok(c_res) = cfg.client.get(format!("{}/api/v1/terminology/{}", cfg.terminology_url, rx.concept_id)).send().await {
            if let Ok(concept) = c_res.json::<TerminologyConcept>().await {
                rx.concept = Some(concept);
            }
        }
    }

    // 6. Fetch Billing Account sequentially
    let bill_res = cfg
        .client
        .get(format!("{}/api/v1/billing/accounts/{}", cfg.billing_url, id))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response())?;
    
    let billing: BillingAccount = bill_res.json().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response())?;

    Ok(Json(PatientSummaryResponse {
        patient,
        labs,
        prescriptions,
        billing,
    }))
}

// Write scenario: POST /visits (Naive sequential execution & naive single-pass rollback)
async fn create_visit(
    State(cfg): State<AppConfig>,
    Json(payload): Json<CreateVisitRequest>,
) -> Result<Json<CreateVisitResponse>, Response> {
    let idempotency_key = uuid::Uuid::new_v4().to_string();

    // Step 1: Create Encounter
    let encounter_req = serde_json::json!({
        "patient_id": payload.patient_id,
        "provider_id": payload.provider_id,
        "department": payload.department,
    });
    let enc_res = cfg
        .client
        .post(format!("{}/api/v1/encounters", cfg.registry_url))
        .header("X-Idempotency-Key", &idempotency_key)
        .json(&encounter_req)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string(), "failed_step": "encounter"}))).into_response())?;

    if !enc_res.status().is_success() {
        return Err((enc_res.status(), Json(serde_json::json!({"error": "Encounter creation failed", "failed_step": "encounter"}))).into_response());
    }
    let encounter: Encounter = enc_res.json().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response())?;

    // Step 2: Reserve Stock
    let res_req = serde_json::json!({
        "patient_id": payload.patient_id,
        "encounter_id": encounter.id,
        "items": payload.medications,
    });
    let stock_res = cfg
        .client
        .post(format!("{}/api/v1/pharmacy/reservations", cfg.pharmacy_url))
        .header("X-Idempotency-Key", &idempotency_key)
        .json(&res_req)
        .send()
        .await;

    let reservation: StockReservation = match stock_res {
        Ok(res) if res.status().is_success() => res.json().await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        })?,
        _ => {
            // Naive rollback: fire and forget encounter delete
            let _ = cfg.client.delete(format!("{}/api/v1/encounters/{}", cfg.registry_url, encounter.id)).send().await;
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Stock reservation failed",
                "failed_step": "reservation",
                "compensated_steps": ["encounter"]
            }))).into_response());
        }
    };

    // Step 3: Create Invoice
    let inv_req = serde_json::json!({
        "patient_id": payload.patient_id,
        "encounter_id": encounter.id,
        "items": payload.billing_items,
    });
    let inv_res = cfg
        .client
        .post(format!("{}/api/v1/billing/invoices", cfg.billing_url))
        .header("X-Idempotency-Key", &idempotency_key)
        .json(&inv_req)
        .send()
        .await;

    let invoice: Invoice = match inv_res {
        Ok(res) if res.status().is_success() => res.json().await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        })?,
        _ => {
            // Naive rollback of reservation and encounter
            let _ = cfg.client.delete(format!("{}/api/v1/pharmacy/reservations/{}", cfg.pharmacy_url, reservation.id)).send().await;
            let _ = cfg.client.delete(format!("{}/api/v1/encounters/{}", cfg.registry_url, encounter.id)).send().await;
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Invoice creation failed",
                "failed_step": "invoice",
                "compensated_steps": ["reservation", "encounter"]
            }))).into_response());
        }
    };

    // Step 4: Send Notification
    let notif_req = serde_json::json!({
        "patient_id": payload.patient_id,
        "recipient": "+256701234567",
        "channel": payload.notification_channel.unwrap_or_else(|| "SMS".to_string()),
        "message": format!("Visit confirmed. Encounter ID: {}, Invoice Total: ${:.2}", encounter.id, invoice.total_amount),
    });
    let notif_res = cfg
        .client
        .post(format!("{}/api/v1/notifications/send", cfg.notifications_url))
        .header("X-Idempotency-Key", &idempotency_key)
        .json(&notif_req)
        .send()
        .await;

    let notification: Notification = match notif_res {
        Ok(res) if res.status().is_success() => res.json().await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        })?,
        _ => {
            // Rollback invoice, reservation, encounter
            let _ = cfg.client.post(format!("{}/api/v1/billing/invoices/{}/void", cfg.billing_url, invoice.id)).send().await;
            let _ = cfg.client.delete(format!("{}/api/v1/pharmacy/reservations/{}", cfg.pharmacy_url, reservation.id)).send().await;
            let _ = cfg.client.delete(format!("{}/api/v1/encounters/{}", cfg.registry_url, encounter.id)).send().await;
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Notification send failed",
                "failed_step": "notification",
                "compensated_steps": ["invoice", "reservation", "encounter"]
            }))).into_response());
        }
    };

    Ok(Json(CreateVisitResponse {
        encounter,
        reservation,
        invoice,
        notification,
    }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cfg = AppConfig::from_env();

    let app = Router::new()
        .route("/patients/:id/summary", get(get_patient_summary))
        .route("/visits", post(create_visit))
        .layer(CorsLayer::permissive())
        .with_state(cfg);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8001".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Naive Gateway listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
