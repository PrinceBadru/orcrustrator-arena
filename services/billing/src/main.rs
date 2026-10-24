use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use shared::{
    models::{BillingAccount, Invoice, InvoiceItem},
    ChaosConfig, ChaosManager, ServiceStats,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    accounts: Arc<RwLock<HashMap<String, BillingAccount>>>,
    invoices: Arc<RwLock<HashMap<String, Invoice>>>,
    idempotency: Arc<RwLock<HashMap<String, Invoice>>>,
    chaos: ChaosManager,
    stats: ServiceStats,
}

impl AppState {
    fn new() -> Self {
        let mut accounts = HashMap::new();
        accounts.insert(
            "p1".to_string(),
            BillingAccount {
                patient_id: "p1".to_string(),
                account_number: "ACC-90021-UG".to_string(),
                status: "GOOD_STANDING".to_string(),
                currency: "USD".to_string(),
                balance: 145.50,
                credit_limit: 2500.00,
                insurance_provider: "Jubilee Health Insurance".to_string(),
            },
        );
        accounts.insert(
            "p2".to_string(),
            BillingAccount {
                patient_id: "p2".to_string(),
                account_number: "ACC-90089-UG".to_string(),
                status: "GOOD_STANDING".to_string(),
                currency: "USD".to_string(),
                balance: 0.00,
                credit_limit: 1500.00,
                insurance_provider: "UAP Old Mutual".to_string(),
            },
        );

        Self {
            accounts: Arc::new(RwLock::new(accounts)),
            invoices: Arc::new(RwLock::new(HashMap::new())),
            idempotency: Arc::new(RwLock::new(HashMap::new())),
            chaos: ChaosManager::new(),
            stats: ServiceStats::new("billing"),
        }
    }
}

async fn get_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<BillingAccount>, Response> {
    state.stats.record_start();
    if let Err(e) = state.chaos.apply().await {
        state.stats.record_failure();
        return Err(e);
    }

    let accs = state.accounts.read().await;
    match accs.get(&id) {
        Some(acc) => {
            state.stats.record_success();
            Ok(Json(acc.clone()))
        }
        None => {
            state.stats.record_failure();
            Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Billing account not found"})),
            )
                .into_response())
        }
    }
}

#[derive(serde::Deserialize)]
struct CreateInvoiceRequest {
    patient_id: String,
    encounter_id: String,
    items: Vec<InvoiceItem>,
}

async fn create_invoice(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<Json<Invoice>, Response> {
    state.stats.record_start();
    if let Err(e) = state.chaos.apply().await {
        state.stats.record_failure();
        return Err(e);
    }

    if let Some(key) = headers.get("X-Idempotency-Key").and_then(|v| v.to_str().ok()) {
        let idem = state.idempotency.read().await;
        if let Some(existing) = idem.get(key) {
            state.stats.record_success();
            return Ok(Json(existing.clone()));
        }
    }

    let total: f64 = payload.items.iter().map(|i| i.amount).sum();
    let invoice_id = format!("inv_{}", uuid::Uuid::new_v4().simple());
    let invoice = Invoice {
        id: invoice_id.clone(),
        patient_id: payload.patient_id,
        encounter_id: payload.encounter_id,
        items: payload.items,
        total_amount: total,
        currency: "USD".to_string(),
        status: "PENDING".to_string(),
        issued_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut invs = state.invoices.write().await;
    invs.insert(invoice_id.clone(), invoice.clone());

    if let Some(key) = headers.get("X-Idempotency-Key").and_then(|v| v.to_str().ok()) {
        let mut idem = state.idempotency.write().await;
        idem.insert(key.to_string(), invoice.clone());
    }

    state.stats.record_success();
    Ok(Json(invoice))
}

async fn void_invoice(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, Response> {
    state.stats.record_start();
    if let Err(e) = state.chaos.apply().await {
        state.stats.record_failure();
        return Err(e);
    }

    let mut invs = state.invoices.write().await;
    if let Some(inv) = invs.get_mut(&id) {
        inv.status = "VOIDED".to_string();
        state.stats.record_success();
        Ok(Json(serde_json::json!({
            "status": "ROLLBACK_SUCCESS",
            "invoice_id": id,
            "invoice_status": "VOIDED"
        })))
    } else {
        state.stats.record_success();
        Ok(Json(serde_json::json!({
            "status": "ALREADY_VOIDED_OR_NOT_FOUND",
            "invoice_id": id
        })))
    }
}

async fn update_chaos(
    State(state): State<AppState>,
    Json(config): Json<ChaosConfig>,
) -> Json<ChaosConfig> {
    state.chaos.update_config(config.clone()).await;
    Json(config)
}

async fn get_stats(State(state): State<AppState>) -> Json<shared::StatsSnapshot> {
    Json(state.stats.snapshot())
}

async fn inspect_records(State(state): State<AppState>) -> Json<serde_json::Value> {
    let map = state.invoices.read().await;
    Json(serde_json::json!({
        "invoices": *map
    }))
}

async fn reset_state(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut map = state.invoices.write().await;
    map.clear();
    let mut idem = state.idempotency.write().await;
    idem.clear();
    state.chaos.update_config(ChaosConfig::default()).await;
    Json(serde_json::json!({"status": "RESET_OK"}))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let state = AppState::new();

    let app = Router::new()
        .route("/api/v1/billing/accounts/:id", get(get_account))
        .route("/api/v1/billing/invoices", post(create_invoice))
        .route("/api/v1/billing/invoices/:id/void", post(void_invoice))
        .route("/chaos", post(update_chaos))
        .route("/stats", get(get_stats))
        .route("/inspect", get(inspect_records))
        .route("/reset", post(reset_state))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8085".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Billing service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
