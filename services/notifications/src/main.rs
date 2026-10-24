use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use shared::{models::Notification, ChaosConfig, ChaosManager, ServiceStats};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    notifications: Arc<RwLock<HashMap<String, Notification>>>,
    idempotency: Arc<RwLock<HashMap<String, Notification>>>,
    chaos: ChaosManager,
    stats: ServiceStats,
}

impl AppState {
    fn new() -> Self {
        Self {
            notifications: Arc::new(RwLock::new(HashMap::new())),
            idempotency: Arc::new(RwLock::new(HashMap::new())),
            chaos: ChaosManager::new(),
            stats: ServiceStats::new("notifications"),
        }
    }
}

#[derive(serde::Deserialize)]
struct SendNotificationRequest {
    patient_id: String,
    recipient: String,
    channel: Option<String>,
    message: String,
}

async fn send_notification(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SendNotificationRequest>,
) -> Result<Json<Notification>, Response> {
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

    let notif_id = format!("notif_{}", uuid::Uuid::new_v4().simple());
    let notification = Notification {
        id: notif_id.clone(),
        patient_id: payload.patient_id,
        recipient: payload.recipient,
        channel: payload.channel.unwrap_or_else(|| "SMS".to_string()),
        message: payload.message,
        status: "SENT".to_string(),
        sent_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut map = state.notifications.write().await;
    map.insert(notif_id.clone(), notification.clone());

    if let Some(key) = headers.get("X-Idempotency-Key").and_then(|v| v.to_str().ok()) {
        let mut idem = state.idempotency.write().await;
        idem.insert(key.to_string(), notification.clone());
    }

    state.stats.record_success();
    Ok(Json(notification))
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
    let map = state.notifications.read().await;
    Json(serde_json::json!({
        "notifications": *map
    }))
}

async fn reset_state(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut map = state.notifications.write().await;
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
        .route("/api/v1/notifications/send", post(send_notification))
        .route("/chaos", post(update_chaos))
        .route("/stats", get(get_stats))
        .route("/inspect", get(inspect_records))
        .route("/reset", post(reset_state))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8086".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Notifications service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
