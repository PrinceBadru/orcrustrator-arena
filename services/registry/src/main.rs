use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use shared::{
    models::{Encounter, Patient},
    ChaosConfig, ChaosManager, ServiceStats,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    patients: Arc<RwLock<HashMap<String, Patient>>>,
    encounters: Arc<RwLock<HashMap<String, Encounter>>>,
    idempotency: Arc<RwLock<HashMap<String, Encounter>>>,
    chaos: ChaosManager,
    stats: ServiceStats,
}

impl AppState {
    fn new() -> Self {
        let mut patients = HashMap::new();
        patients.insert(
            "p1".to_string(),
            Patient {
                id: "p1".to_string(),
                national_id: "UG-100234".to_string(),
                first_name: "Amina".to_string(),
                last_name: "Okello".to_string(),
                birth_date: "1988-04-12".to_string(),
                gender: "Female".to_string(),
                blood_type: "O+".to_string(),
            },
        );
        patients.insert(
            "p2".to_string(),
            Patient {
                id: "p2".to_string(),
                national_id: "UG-200981".to_string(),
                first_name: "David".to_string(),
                last_name: "Mukasa".to_string(),
                birth_date: "1975-11-23".to_string(),
                gender: "Male".to_string(),
                blood_type: "A+".to_string(),
            },
        );

        Self {
            patients: Arc::new(RwLock::new(patients)),
            encounters: Arc::new(RwLock::new(HashMap::new())),
            idempotency: Arc::new(RwLock::new(HashMap::new())),
            chaos: ChaosManager::new(),
            stats: ServiceStats::new("registry"),
        }
    }
}

async fn get_patient(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Patient>, Response> {
    state.stats.record_start();
    if let Err(e) = state.chaos.apply().await {
        state.stats.record_failure();
        return Err(e);
    }

    let p = state.patients.read().await;
    match p.get(&id) {
        Some(patient) => {
            state.stats.record_success();
            Ok(Json(patient.clone()))
        }
        None => {
            state.stats.record_failure();
            Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Patient not found"})),
            )
                .into_response())
        }
    }
}

#[derive(serde::Deserialize)]
struct CreateEncounterRequest {
    patient_id: String,
    provider_id: String,
    encounter_type: Option<String>,
    department: Option<String>,
}

async fn create_encounter(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateEncounterRequest>,
) -> Result<Json<Encounter>, Response> {
    state.stats.record_start();
    if let Err(e) = state.chaos.apply().await {
        state.stats.record_failure();
        return Err(e);
    }

    // Check idempotency key if provided
    if let Some(key) = headers.get("X-Idempotency-Key").and_then(|v| v.to_str().ok()) {
        let idem = state.idempotency.read().await;
        if let Some(existing) = idem.get(key) {
            state.stats.record_success();
            return Ok(Json(existing.clone()));
        }
    }

    let encounter_id = format!("enc_{}", uuid::Uuid::new_v4().simple());
    let encounter = Encounter {
        id: encounter_id.clone(),
        patient_id: payload.patient_id,
        provider_id: payload.provider_id,
        encounter_type: payload.encounter_type.unwrap_or_else(|| "Outpatient".to_string()),
        department: payload.department.unwrap_or_else(|| "General Practice".to_string()),
        status: "ACTIVE".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut encs = state.encounters.write().await;
    encs.insert(encounter_id.clone(), encounter.clone());

    if let Some(key) = headers.get("X-Idempotency-Key").and_then(|v| v.to_str().ok()) {
        let mut idem = state.idempotency.write().await;
        idem.insert(key.to_string(), encounter.clone());
    }

    state.stats.record_success();
    Ok(Json(encounter))
}

async fn delete_encounter(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, Response> {
    state.stats.record_start();
    if let Err(e) = state.chaos.apply().await {
        state.stats.record_failure();
        return Err(e);
    }

    let mut encs = state.encounters.write().await;
    if let Some(mut enc) = encs.remove(&id) {
        enc.status = "CANCELLED".to_string();
        state.stats.record_success();
        Ok(Json(serde_json::json!({
            "status": "ROLLBACK_SUCCESS",
            "encounter_id": id
        })))
    } else {
        // Idempotent delete: if already deleted or absent, return success
        state.stats.record_success();
        Ok(Json(serde_json::json!({
            "status": "ALREADY_DELETED",
            "encounter_id": id
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
    let encs = state.encounters.read().await;
    Json(serde_json::json!({
        "encounters": *encs
    }))
}

async fn reset_state(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut encs = state.encounters.write().await;
    encs.clear();
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
        .route("/api/v1/patients/:id", get(get_patient))
        .route("/api/v1/encounters", post(create_encounter))
        .route("/api/v1/encounters/:id", delete(delete_encounter))
        .route("/chaos", post(update_chaos))
        .route("/stats", get(get_stats))
        .route("/inspect", get(inspect_records))
        .route("/reset", post(reset_state))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Registry service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
