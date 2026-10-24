use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use shared::{
    models::{Prescription, StockReservation, StockReservationItem},
    ChaosConfig, ChaosManager, ServiceStats,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    prescriptions: Arc<RwLock<HashMap<String, Vec<Prescription>>>>,
    reservations: Arc<RwLock<HashMap<String, StockReservation>>>,
    idempotency: Arc<RwLock<HashMap<String, StockReservation>>>,
    chaos: ChaosManager,
    stats: ServiceStats,
}

impl AppState {
    fn new() -> Self {
        let mut prescriptions = HashMap::new();
        prescriptions.insert(
            "p1".to_string(),
            vec![
                Prescription {
                    id: "rx_01".to_string(),
                    patient_id: "p1".to_string(),
                    medication_code: "AMOX_500".to_string(),
                    medication_name: "Amoxicillin 500mg".to_string(),
                    concept_id: "RXNORM-7052".to_string(),
                    dosage: "500mg".to_string(),
                    frequency: "TID".to_string(),
                    duration_days: 7,
                    status: "ACTIVE".to_string(),
                    prescribed_at: "2026-10-21T10:00:00Z".to_string(),
                    concept: None,
                },
                Prescription {
                    id: "rx_02".to_string(),
                    patient_id: "p1".to_string(),
                    medication_code: "PARA_500".to_string(),
                    medication_name: "Paracetamol 500mg".to_string(),
                    concept_id: "RXNORM-161".to_string(),
                    dosage: "1000mg".to_string(),
                    frequency: "PRN".to_string(),
                    duration_days: 5,
                    status: "ACTIVE".to_string(),
                    prescribed_at: "2026-10-21T10:00:00Z".to_string(),
                    concept: None,
                },
            ],
        );

        Self {
            prescriptions: Arc::new(RwLock::new(prescriptions)),
            reservations: Arc::new(RwLock::new(HashMap::new())),
            idempotency: Arc::new(RwLock::new(HashMap::new())),
            chaos: ChaosManager::new(),
            stats: ServiceStats::new("pharmacy"),
        }
    }
}

async fn get_prescriptions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Prescription>>, Response> {
    state.stats.record_start();
    if let Err(e) = state.chaos.apply().await {
        state.stats.record_failure();
        return Err(e);
    }

    let rx = state.prescriptions.read().await;
    let list = rx.get(&id).cloned().unwrap_or_default();
    state.stats.record_success();
    Ok(Json(list))
}

#[derive(serde::Deserialize)]
struct CreateReservationRequest {
    patient_id: String,
    encounter_id: String,
    items: Vec<StockReservationItem>,
}

async fn create_reservation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateReservationRequest>,
) -> Result<Json<StockReservation>, Response> {
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

    let res_id = format!("res_{}", uuid::Uuid::new_v4().simple());
    let reservation = StockReservation {
        id: res_id.clone(),
        patient_id: payload.patient_id,
        encounter_id: payload.encounter_id,
        items: payload.items,
        status: "RESERVED".to_string(),
        reserved_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut map = state.reservations.write().await;
    map.insert(res_id.clone(), reservation.clone());

    if let Some(key) = headers.get("X-Idempotency-Key").and_then(|v| v.to_str().ok()) {
        let mut idem = state.idempotency.write().await;
        idem.insert(key.to_string(), reservation.clone());
    }

    state.stats.record_success();
    Ok(Json(reservation))
}

async fn delete_reservation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, Response> {
    state.stats.record_start();
    if let Err(e) = state.chaos.apply().await {
        state.stats.record_failure();
        return Err(e);
    }

    let mut map = state.reservations.write().await;
    if let Some(mut res) = map.remove(&id) {
        res.status = "CANCELLED".to_string();
        state.stats.record_success();
        Ok(Json(serde_json::json!({
            "status": "ROLLBACK_SUCCESS",
            "reservation_id": id
        })))
    } else {
        state.stats.record_success();
        Ok(Json(serde_json::json!({
            "status": "ALREADY_DELETED",
            "reservation_id": id
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
    let map = state.reservations.read().await;
    Json(serde_json::json!({
        "reservations": *map
    }))
}

async fn reset_state(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut map = state.reservations.write().await;
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
        .route("/api/v1/patients/:id/prescriptions", get(get_prescriptions))
        .route("/api/v1/pharmacy/reservations", post(create_reservation))
        .route("/api/v1/pharmacy/reservations/:id", delete(delete_reservation))
        .route("/chaos", post(update_chaos))
        .route("/stats", get(get_stats))
        .route("/inspect", get(inspect_records))
        .route("/reset", post(reset_state))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8083".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Pharmacy service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
