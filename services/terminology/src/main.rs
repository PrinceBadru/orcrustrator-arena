use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use shared::{models::TerminologyConcept, ChaosConfig, ChaosManager, ServiceStats};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    concepts: Arc<RwLock<HashMap<String, TerminologyConcept>>>,
    chaos: ChaosManager,
    stats: ServiceStats,
}

impl AppState {
    fn new() -> Self {
        let mut map = HashMap::new();
        // LOINC concepts
        map.insert(
            "LOINC-718-7".to_string(),
            TerminologyConcept {
                concept_id: "LOINC-718-7".to_string(),
                system: "http://loinc.org".to_string(),
                code: "718-7".to_string(),
                display: "Hemoglobin [Mass/volume] in Blood".to_string(),
                category: "Laboratory / Hematology".to_string(),
                clinical_description: "Measures oxygen-carrying protein hemoglobin in whole blood.".to_string(),
            },
        );
        map.insert(
            "LOINC-2345-7".to_string(),
            TerminologyConcept {
                concept_id: "LOINC-2345-7".to_string(),
                system: "http://loinc.org".to_string(),
                code: "2345-7".to_string(),
                display: "Glucose [Mass/volume] in Serum or Plasma --Fasting".to_string(),
                category: "Laboratory / Chemistry".to_string(),
                clinical_description: "Standard screening test for prediabetes and type 2 diabetes mellitus.".to_string(),
            },
        );
        map.insert(
            "LOINC-2093-3".to_string(),
            TerminologyConcept {
                concept_id: "LOINC-2093-3".to_string(),
                system: "http://loinc.org".to_string(),
                code: "2093-3".to_string(),
                display: "Cholesterol [Mass/volume] in Serum or Plasma".to_string(),
                category: "Laboratory / Lipid Panel".to_string(),
                clinical_description: "Measures total circulating sterol lipid levels for cardiovascular risk assessment.".to_string(),
            },
        );
        // RxNorm concepts
        map.insert(
            "RXNORM-7052".to_string(),
            TerminologyConcept {
                concept_id: "RXNORM-7052".to_string(),
                system: "http://www.nlm.nih.gov/research/umls/rxnorm".to_string(),
                code: "7052".to_string(),
                display: "Amoxicillin 500 MG Oral Capsule".to_string(),
                category: "Pharmacy / Antibacterial".to_string(),
                clinical_description: "Broad-spectrum beta-lactam penicillin class antibacterial agent.".to_string(),
            },
        );
        map.insert(
            "RXNORM-161".to_string(),
            TerminologyConcept {
                concept_id: "RXNORM-161".to_string(),
                system: "http://www.nlm.nih.gov/research/umls/rxnorm".to_string(),
                code: "161".to_string(),
                display: "Acetaminophen 500 MG Oral Tablet".to_string(),
                category: "Pharmacy / Analgesic & Antipyretic".to_string(),
                clinical_description: "Central analgesic and antipyretic agent indicated for mild-to-moderate pain and fever.".to_string(),
            },
        );

        Self {
            concepts: Arc::new(RwLock::new(map)),
            chaos: ChaosManager::new(),
            stats: ServiceStats::new("terminology"),
        }
    }
}

async fn get_concept(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TerminologyConcept>, Response> {
    state.stats.record_start();
    if let Err(e) = state.chaos.apply().await {
        state.stats.record_failure();
        return Err(e);
    }

    let c = state.concepts.read().await;
    match c.get(&id) {
        Some(concept) => {
            state.stats.record_success();
            Ok(Json(concept.clone()))
        }
        None => {
            state.stats.record_failure();
            Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Concept '{}' not found", id)})),
            )
                .into_response())
        }
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

async fn reset_state(State(state): State<AppState>) -> Json<serde_json::Value> {
    state.chaos.update_config(ChaosConfig::default()).await;
    Json(serde_json::json!({"status": "RESET_OK"}))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let state = AppState::new();

    let app = Router::new()
        .route("/api/v1/terminology/:id", get(get_concept))
        .route("/chaos", post(update_chaos))
        .route("/stats", get(get_stats))
        .route("/reset", post(reset_state))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8084".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Terminology service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
