use axum::{
    extract::{Path, State},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use shared::{models::LabResult, ChaosConfig, ChaosManager, ServiceStats};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    labs: Arc<RwLock<HashMap<String, Vec<LabResult>>>>,
    chaos: ChaosManager,
    stats: ServiceStats,
}

impl AppState {
    fn new() -> Self {
        let mut labs = HashMap::new();
        labs.insert(
            "p1".to_string(),
            vec![
                LabResult {
                    id: "lab_01".to_string(),
                    patient_id: "p1".to_string(),
                    test_code: "CBC_HB".to_string(),
                    test_name: "Hemoglobin".to_string(),
                    concept_id: "LOINC-718-7".to_string(),
                    value: 13.8,
                    unit: "g/dL".to_string(),
                    reference_range: "12.0 - 15.5".to_string(),
                    status: "FINAL".to_string(),
                    collected_at: "2026-10-20T09:15:00Z".to_string(),
                    concept: None,
                },
                LabResult {
                    id: "lab_02".to_string(),
                    patient_id: "p1".to_string(),
                    test_code: "GLU_FAST".to_string(),
                    test_name: "Fasting Glucose".to_string(),
                    concept_id: "LOINC-2345-7".to_string(),
                    value: 94.2,
                    unit: "mg/dL".to_string(),
                    reference_range: "70.0 - 99.0".to_string(),
                    status: "FINAL".to_string(),
                    collected_at: "2026-10-20T09:15:00Z".to_string(),
                    concept: None,
                },
            ],
        );
        labs.insert(
            "p2".to_string(),
            vec![LabResult {
                id: "lab_03".to_string(),
                patient_id: "p2".to_string(),
                test_code: "CHOL_TOTAL".to_string(),
                test_name: "Total Cholesterol".to_string(),
                concept_id: "LOINC-2093-3".to_string(),
                value: 185.0,
                unit: "mg/dL".to_string(),
                reference_range: "< 200.0".to_string(),
                status: "FINAL".to_string(),
                collected_at: "2026-10-18T14:30:00Z".to_string(),
                concept: None,
            }],
        );

        Self {
            labs: Arc::new(RwLock::new(labs)),
            chaos: ChaosManager::new(),
            stats: ServiceStats::new("lab"),
        }
    }
}

async fn get_patient_labs(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<LabResult>>, Response> {
    state.stats.record_start();
    if let Err(e) = state.chaos.apply().await {
        state.stats.record_failure();
        return Err(e);
    }

    let labs = state.labs.read().await;
    let list = labs.get(&id).cloned().unwrap_or_default();
    state.stats.record_success();
    Ok(Json(list))
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

async fn reset_state(State(state): State<AppState>) -> Json<serde_json::Value> {
    state.chaos.update_config(ChaosConfig::default()).await;
    Json(serde_json::json!({"status": "RESET_OK"}))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let state = AppState::new();

    let app = Router::new()
        .route("/api/v1/patients/:id/labs", get(get_patient_labs))
        .route("/chaos", post(update_chaos))
        .route("/stats", get(get_stats))
        .route("/reset", post(reset_state))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8082".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Lab service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
