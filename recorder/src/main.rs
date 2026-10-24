use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
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
    pub status: String, // "OK", "ERROR", "CACHED", "COMPENSATED"
    pub measurement_type: String, // "Observed (OTel Span)" or "Derived (Gateway Event)"
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
    pub measurement_type: String, // "Observed" or "Derived"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayStats {
    pub gateway: String,
    pub total_requests: usize,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub mean_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub error_rate_percent: f64,
    pub cache_hit_rate_percent: f64,
    pub upstream_calls_saved: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkStatsResponse {
    pub generated_at: String,
    pub methodology: String,
    pub gateways: Vec<GatewayStats>,
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
            history: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            tx,
        }
    }
}

async fn ingest_telemetry(
    State(state): State<AppState>,
    Json(mut trace): Json<RequestTrace>,
) -> Json<serde_json::Value> {
    if trace.measurement_type.is_empty() {
        trace.measurement_type = "Observed (OTel Span)".to_string();
    }

    {
        let mut h = state.history.write().await;
        if h.len() >= 1000 {
            h.pop_front();
        }
        h.push_back(trace.clone());
    }

    // Broadcast to WebSocket clients
    if let Ok(msg) = serde_json::to_string(&trace) {
        let _ = state.tx.send(msg);
    }

    Json(serde_json::json!({"status": "INGESTED", "trace_id": trace.trace_id}))
}

async fn get_history(State(state): State<AppState>) -> Json<Vec<RequestTrace>> {
    let h = state.history.read().await;
    Json(h.iter().cloned().collect())
}

fn calculate_percentiles(mut latencies: Vec<f64>) -> (f64, f64, f64, f64, f64, f64) {
    if latencies.is_empty() {
        return (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    }
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let len = latencies.len();
    let min = latencies[0];
    let max = latencies[len - 1];
    let sum: f64 = latencies.iter().sum();
    let mean = sum / len as f64;

    let p50 = latencies[((len as f64 * 0.50).ceil() as usize).saturating_sub(1)];
    let p95 = latencies[((len as f64 * 0.95).ceil() as usize).saturating_sub(1)];
    let p99 = latencies[((len as f64 * 0.99).ceil() as usize).saturating_sub(1)];

    (min, max, mean, p50, p95, p99)
}

async fn get_aggregate_stats(State(state): State<AppState>) -> Json<BenchmarkStatsResponse> {
    let h = state.history.read().await;

    let gateway_names = ["naive", "hand-rolled", "orcrustrator"];
    let mut stats_list = Vec::new();

    for gw in &gateway_names {
        let matching: Vec<&RequestTrace> = h.iter().filter(|t| t.gateway.to_lowercase().contains(gw)).collect();
        let total = matching.len();

        if total == 0 {
            stats_list.push(GatewayStats {
                gateway: gw.to_string(),
                total_requests: 0,
                p50_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
                mean_ms: 0.0,
                min_ms: 0.0,
                max_ms: 0.0,
                error_rate_percent: 0.0,
                cache_hit_rate_percent: 0.0,
                upstream_calls_saved: 0,
            });
            continue;
        }

        let latencies: Vec<f64> = matching.iter().map(|t| t.total_duration_ms).collect();
        let (min, max, mean, p50, p95, p99) = calculate_percentiles(latencies);

        let error_count = matching.iter().filter(|t| t.status_code >= 400).count();
        let cache_hit_count = matching.iter().filter(|t| t.cache_hit).count();
        let saved_calls = matching.iter().filter(|t| t.cache_hit).map(|t| t.upstream_calls_count).sum();

        stats_list.push(GatewayStats {
            gateway: gw.to_string(),
            total_requests: total,
            p50_ms: NumberFormat(p50),
            p95_ms: NumberFormat(p95),
            p99_ms: NumberFormat(p99),
            mean_ms: NumberFormat(mean),
            min_ms: NumberFormat(min),
            max_ms: NumberFormat(max),
            error_rate_percent: NumberFormat((error_count as f64 / total as f64) * 100.0),
            cache_hit_rate_percent: NumberFormat((cache_hit_count as f64 / total as f64) * 100.0),
            upstream_calls_saved: saved_calls,
        });
    }

    Json(BenchmarkStatsResponse {
        generated_at: chrono::Utc::now().to_rfc3339(),
        methodology: "Observed spans recorded via OpenTelemetry instrumentation; Constant Arrival Rate".to_string(),
        gateways: stats_list,
    })
}

fn NumberFormat(val: f64) -> f64 {
    (val * 100.0).round() / 100.0
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
        .route("/api/stats", get(get_aggregate_stats))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8095".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Recorder & Observability service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
