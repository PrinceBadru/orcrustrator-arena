use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, net::SocketAddr, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSpan {
    pub stage_name: String,
    pub target_service: String,
    pub start_ms: f64,
    pub duration_ms: f64,
    pub status: String, // "OK", "ERROR", "CACHED"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestTrace {
    pub trace_id: String,
    pub gateway: String, // "naive", "hand-rolled", "orcrustrator"
    pub scenario: String, // "read_summary", "write_visit"
    pub total_duration_ms: f64,
    pub status_code: u16,
    pub stages: Vec<StageSpan>,
    pub timestamp: String,
    pub upstream_calls_count: usize,
    pub cache_hit: bool,
}

#[derive(Clone)]
struct AppState {
    history: Arc<RwLock<VecDeque<RequestTrace>>>,
    tx: broadcast::Sender<String>,
}

impl AppState {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self {
            history: Arc::new(RwLock::new(VecDeque::with_capacity(500))),
            tx,
        }
    }
}

async fn ingest_telemetry(
    State(state): State<AppState>,
    Json(trace): Json<RequestTrace>,
) -> Json<serde_json::Value> {
    {
        let mut h = state.history.write().await;
        if h.len() >= 500 {
            h.pop_front();
        }
        h.push_back(trace.clone());
    }

    // Broadcast to WebSocket clients
    if let Ok(msg) = serde_json::to_string(&trace) {
        let _ = state.tx.send(msg);
    }

    Json(serde_json::json!({"status": "INGESTED"}))
}

async fn get_history(State(state): State<AppState>) -> Json<Vec<RequestTrace>> {
    let h = state.history.read().await;
    Json(h.iter().cloned().collect())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(_)) = receiver.next().await {}
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let state = AppState::new();

    let app = Router::new()
        .route("/api/telemetry", post(ingest_telemetry))
        .route("/api/history", get(get_history))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8095".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Recorder & WebSocket service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
