use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosConfig {
    pub latency_ms: u64,
    pub jitter_ms: u64,
    pub error_rate: f64, // 0.0 to 1.0
    pub offline: bool,   // Instant kill switch
    pub failure_status_code: u16,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            latency_ms: 0,
            jitter_ms: 0,
            error_rate: 0.0,
            offline: false,
            failure_status_code: 500,
        }
    }
}

#[derive(Clone, Default)]
pub struct ChaosManager {
    config: Arc<RwLock<ChaosConfig>>,
}

impl ChaosManager {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(ChaosConfig::default())),
        }
    }

    pub async fn get_config(&self) -> ChaosConfig {
        self.config.read().await.clone()
    }

    pub async fn update_config(&self, new_config: ChaosConfig) {
        let mut w = self.config.write().await;
        *w = new_config;
    }

    pub async fn apply(&self) -> Result<(), Response> {
        let cfg = self.config.read().await.clone();

        if cfg.offline {
            let err_body = serde_json::json!({
                "error": "Service unavailable (simulated outage)",
                "status": 503
            });
            return Err((StatusCode::SERVICE_UNAVAILABLE, Json(err_body)).into_response());
        }

        // Apply latency + jitter
        if cfg.latency_ms > 0 || cfg.jitter_ms > 0 {
            let jitter = if cfg.jitter_ms > 0 {
                rand::thread_rng().gen_range(0..=cfg.jitter_ms)
            } else {
                0
            };
            let total_delay = cfg.latency_ms + jitter;
            tokio::time::sleep(Duration::from_millis(total_delay)).await;
        }

        // Apply error rate
        if cfg.error_rate > 0.0 {
            let roll: f64 = rand::thread_rng().gen();
            if roll < cfg.error_rate {
                let status = StatusCode::from_u16(cfg.failure_status_code)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                let err_body = serde_json::json!({
                    "error": "Simulated chaos failure",
                    "status": status.as_u16(),
                    "rate": cfg.error_rate
                });
                return Err((status, Json(err_body)).into_response());
            }
        }

        Ok(())
    }
}
