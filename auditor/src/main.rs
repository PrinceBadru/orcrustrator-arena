use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shared::models::*;
use std::{collections::HashMap, net::SocketAddr};
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AuditorConfig {
    registry_url: String,
    pharmacy_url: String,
    billing_url: String,
    notifications_url: String,
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsistencyViolation {
    pub rule: String,
    pub entity_type: String,
    pub entity_id: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditReport {
    pub audited_at: String,
    pub total_encounters: usize,
    pub total_reservations: usize,
    pub total_invoices: usize,
    pub total_notifications: usize,
    pub violations_count: usize,
    pub violations: Vec<ConsistencyViolation>,
    pub is_clean: bool,
}

impl AuditorConfig {
    fn from_env() -> Self {
        Self {
            registry_url: std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8081".to_string()),
            pharmacy_url: std::env::var("PHARMACY_URL").unwrap_or_else(|_| "http://localhost:8083".to_string()),
            billing_url: std::env::var("BILLING_URL").unwrap_or_else(|_| "http://localhost:8085".to_string()),
            notifications_url: std::env::var("NOTIFICATIONS_URL").unwrap_or_else(|_| "http://localhost:8086".to_string()),
            client: Client::new(),
        }
    }

    async fn audit_all(&self) -> Result<AuditReport, String> {
        // 1. Fetch encounters
        let enc_res = self.client.get(format!("{}/inspect", self.registry_url)).send().await
            .map_err(|e| format!("Registry inspect failed: {}", e))?;
        let enc_json: serde_json::Value = enc_res.json().await.map_err(|e| e.to_string())?;
        let encounters: HashMap<String, Encounter> = serde_json::from_value(enc_json.get("encounters").cloned().unwrap_or_default()).unwrap_or_default();

        // 2. Fetch reservations
        let res_res = self.client.get(format!("{}/inspect", self.pharmacy_url)).send().await
            .map_err(|e| format!("Pharmacy inspect failed: {}", e))?;
        let res_json: serde_json::Value = res_res.json().await.map_err(|e| e.to_string())?;
        let reservations: HashMap<String, StockReservation> = serde_json::from_value(res_json.get("reservations").cloned().unwrap_or_default()).unwrap_or_default();

        // 3. Fetch invoices
        let inv_res = self.client.get(format!("{}/inspect", self.billing_url)).send().await
            .map_err(|e| format!("Billing inspect failed: {}", e))?;
        let inv_json: serde_json::Value = inv_res.json().await.map_err(|e| e.to_string())?;
        let invoices: HashMap<String, Invoice> = serde_json::from_value(inv_json.get("invoices").cloned().unwrap_or_default()).unwrap_or_default();

        // 4. Fetch notifications
        let notif_res = self.client.get(format!("{}/inspect", self.notifications_url)).send().await
            .map_err(|e| format!("Notifications inspect failed: {}", e))?;
        let notif_json: serde_json::Value = notif_res.json().await.map_err(|e| e.to_string())?;
        let notifications: HashMap<String, Notification> = serde_json::from_value(notif_json.get("notifications").cloned().unwrap_or_default()).unwrap_or_default();

        let mut violations = Vec::new();

        // Invariant 1: No invoice without an active encounter
        for (inv_id, inv) in &invoices {
            if inv.status != "VOIDED" {
                if !encounters.contains_key(&inv.encounter_id) {
                    violations.push(ConsistencyViolation {
                        rule: "No invoice without an encounter".to_string(),
                        entity_type: "Invoice".to_string(),
                        entity_id: inv_id.clone(),
                        description: format!("Active invoice references missing encounter '{}'", inv.encounter_id),
                    });
                }
            }
        }

        // Invariant 2: No active stock reservation without an active encounter & invoice
        for (res_id, res) in &reservations {
            if res.status != "CANCELLED" {
                if !encounters.contains_key(&res.encounter_id) {
                    violations.push(ConsistencyViolation {
                        rule: "No reserved stock without an encounter".to_string(),
                        entity_type: "StockReservation".to_string(),
                        entity_id: res_id.clone(),
                        description: format!("Active reservation references missing encounter '{}'", res.encounter_id),
                    });
                }
                let has_invoice = invoices.values().any(|inv| inv.encounter_id == res.encounter_id && inv.status != "VOIDED");
                if !has_invoice {
                    violations.push(ConsistencyViolation {
                        rule: "No reserved stock without an active invoice".to_string(),
                        entity_type: "StockReservation".to_string(),
                        entity_id: res_id.clone(),
                        description: format!("Active reservation exists but no valid invoice found for encounter '{}'", res.encounter_id),
                    });
                }
            }
        }

        let is_clean = violations.is_empty();

        Ok(AuditReport {
            audited_at: chrono::Utc::now().to_rfc3339(),
            total_encounters: encounters.len(),
            total_reservations: reservations.len(),
            total_invoices: invoices.len(),
            total_notifications: notifications.len(),
            violations_count: violations.len(),
            violations,
            is_clean,
        })
    }
}

async fn run_audit(State(cfg): State<AuditorConfig>) -> Result<Json<AuditReport>, Response> {
    match cfg.audit_all().await {
        Ok(report) => Ok(Json(report)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e}))).into_response()),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cfg = AuditorConfig::from_env();

    let app = Router::new()
        .route("/audit", get(run_audit))
        .route("/audit/run", post(run_audit))
        .layer(CorsLayer::permissive())
        .with_state(cfg);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8090".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Auditor service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
