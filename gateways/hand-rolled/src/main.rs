use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures::future::join_all;
use reqwest::Client;
use shared::models::*;
use std::{collections::{HashMap, HashSet}, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppConfig {
    registry_url: String,
    lab_url: String,
    pharmacy_url: String,
    terminology_url: String,
    billing_url: String,
    notifications_url: String,
    client: Client,
    term_cache: Arc<RwLock<HashMap<String, TerminologyConcept>>>,
}

impl AppConfig {
    fn from_env() -> Self {
        Self {
            registry_url: std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8081".to_string()),
            lab_url: std::env::var("LAB_URL").unwrap_or_else(|_| "http://localhost:8082".to_string()),
            pharmacy_url: std::env::var("PHARMACY_URL").unwrap_or_else(|_| "http://localhost:8083".to_string()),
            terminology_url: std::env::var("TERMINOLOGY_URL").unwrap_or_else(|_| "http://localhost:8084".to_string()),
            billing_url: std::env::var("BILLING_URL").unwrap_or_else(|_| "http://localhost:8085".to_string()),
            notifications_url: std::env::var("NOTIFICATIONS_URL").unwrap_or_else(|_| "http://localhost:8086".to_string()),
            client: Client::builder().pool_max_idle_per_host(50).build().unwrap(),
            term_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn fetch_concept(&self, concept_id: &str) -> Option<TerminologyConcept> {
        // Fast path: Cache hit
        {
            let cache = self.term_cache.read().await;
            if let Some(c) = cache.get(concept_id) {
                return Some(c.clone());
            }
        }

        // Cache miss: network fetch
        if let Ok(res) = self.client.get(format!("{}/api/v1/terminology/{}", self.terminology_url, concept_id)).send().await {
            if let Ok(c) = res.json::<TerminologyConcept>().await {
                let mut cache = self.term_cache.write().await;
                cache.insert(concept_id.to_string(), c.clone());
                return Some(c);
            }
        }
        None
    }
}

// Read scenario: GET /patients/:id/summary (Tokio DAG concurrency & Terminology cache)
async fn get_patient_summary(
    State(cfg): State<AppConfig>,
    Path(id): Path<String>,
) -> Result<Json<PatientSummaryResponse>, Response> {
    let client = &cfg.client;

    // Stage 0: Concurrently fetch Patient, Labs, Prescriptions, Billing
    let patient_fut = client.get(format!("{}/api/v1/patients/{}", cfg.registry_url, id)).send();
    let labs_fut = client.get(format!("{}/api/v1/patients/{}/labs", cfg.lab_url, id)).send();
    let rx_fut = client.get(format!("{}/api/v1/patients/{}/prescriptions", cfg.pharmacy_url, id)).send();
    let bill_fut = client.get(format!("{}/api/v1/billing/accounts/{}", cfg.billing_url, id)).send();

    let (patient_res, labs_res, rx_res, bill_res) = tokio::join!(patient_fut, labs_fut, rx_fut, bill_fut);

    let patient_resp = patient_res.map_err(|e| (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response())?;
    if !patient_resp.status().is_success() {
        return Err((patient_resp.status(), Json(serde_json::json!({"error": "Failed to fetch patient"}))).into_response());
    }
    let patient: Patient = patient_resp.json().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response())?;

    let mut labs: Vec<LabResult> = match labs_res {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
        _ => Vec::new(),
    };

    let mut prescriptions: Vec<Prescription> = match rx_res {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
        _ => Vec::new(),
    };

    let billing: BillingAccount = match bill_res {
        Ok(r) if r.status().is_success() => r.json().await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        })?,
        _ => return Err((StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": "Failed to fetch billing account"}))).into_response()),
    };

    // Stage 1: Fan-out concurrent resolution of all unique terminology concepts
    let mut unique_concepts: HashSet<String> = HashSet::new();
    for l in &labs {
        unique_concepts.insert(l.concept_id.clone());
    }
    for r in &prescriptions {
        unique_concepts.insert(r.concept_id.clone());
    }

    let concept_futures: Vec<_> = unique_concepts
        .iter()
        .map(|cid| {
            let app_cfg = cfg.clone();
            let c = cid.clone();
            async move { (c.clone(), app_cfg.fetch_concept(&c).await) }
        })
        .collect();

    let resolved_concepts: HashMap<String, TerminologyConcept> = join_all(concept_futures)
        .await
        .into_iter()
        .filter_map(|(cid, opt)| opt.map(|concept| (cid, concept)))
        .collect();

    for l in labs.iter_mut() {
        if let Some(c) = resolved_concepts.get(&l.concept_id) {
            l.concept = Some(c.clone());
        }
    }

    for r in prescriptions.iter_mut() {
        if let Some(c) = resolved_concepts.get(&r.concept_id) {
            r.concept = Some(c.clone());
        }
    }

    Ok(Json(PatientSummaryResponse {
        patient,
        labs,
        prescriptions,
        billing,
    }))
}

// Helper with retries
async fn call_with_retry<F, Fut, T>(mut op: F, max_retries: usize) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let mut attempt = 0;
    loop {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(e);
                }
                tokio::time::sleep(Duration::from_millis(50 * (1 << attempt))).await;
            }
        }
    }
}

// Write scenario: POST /visits (Robust Saga Pattern with Backoff & Full Compensation Audit)
async fn create_visit(
    State(cfg): State<AppConfig>,
    Json(payload): Json<CreateVisitRequest>,
) -> Result<Json<CreateVisitResponse>, Response> {
    let idempotency_key = uuid::Uuid::new_v4().to_string();

    // Step 1: Create Encounter
    let encounter_req = serde_json::json!({
        "patient_id": payload.patient_id,
        "provider_id": payload.provider_id,
        "department": payload.department,
    });

    let client = cfg.client.clone();
    let reg_url = cfg.registry_url.clone();
    let enc_key = idempotency_key.clone();

    let encounter = match call_with_retry(|| {
        let cl = client.clone();
        let u = reg_url.clone();
        let k = enc_key.clone();
        let b = encounter_req.clone();
        async move {
            let res = cl.post(format!("{}/api/v1/encounters", u))
                .header("X-Idempotency-Key", k)
                .json(&b)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if res.status().is_success() {
                res.json::<Encounter>().await.map_err(|e| e.to_string())
            } else {
                Err(format!("Encounter endpoint status {}", res.status()))
            }
        }
    }, 2).await {
        Ok(enc) => enc,
        Err(e) => return Err((StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e, "failed_step": "encounter"}))).into_response()),
    };

    // Step 2: Reserve Pharmacy Stock
    let res_req = serde_json::json!({
        "patient_id": payload.patient_id,
        "encounter_id": encounter.id,
        "items": payload.medications,
    });
    let pharm_url = cfg.pharmacy_url.clone();
    let res_key = idempotency_key.clone();

    let reservation = match call_with_retry(|| {
        let cl = client.clone();
        let u = pharm_url.clone();
        let k = res_key.clone();
        let b = res_req.clone();
        async move {
            let res = cl.post(format!("{}/api/v1/pharmacy/reservations", u))
                .header("X-Idempotency-Key", k)
                .json(&b)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if res.status().is_success() {
                res.json::<StockReservation>().await.map_err(|e| e.to_string())
            } else {
                Err(format!("Reservation endpoint status {}", res.status()))
            }
        }
    }, 2).await {
        Ok(r) => r,
        Err(e) => {
            // Compensate Step 1
            let comp_res = cfg.client.delete(format!("{}/api/v1/encounters/{}", cfg.registry_url, encounter.id)).send().await;
            let comp_ok = comp_res.map(|r| r.status().is_success()).unwrap_or(false);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": e,
                "failed_step": "reservation",
                "compensated_steps": [{"step": "encounter", "success": comp_ok}]
            }))).into_response());
        }
    };

    // Step 3: Create Invoice
    let inv_req = serde_json::json!({
        "patient_id": payload.patient_id,
        "encounter_id": encounter.id,
        "items": payload.billing_items,
    });
    let bill_url = cfg.billing_url.clone();
    let inv_key = idempotency_key.clone();

    let invoice = match call_with_retry(|| {
        let cl = client.clone();
        let u = bill_url.clone();
        let k = inv_key.clone();
        let b = inv_req.clone();
        async move {
            let res = cl.post(format!("{}/api/v1/billing/invoices", u))
                .header("X-Idempotency-Key", k)
                .json(&b)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if res.status().is_success() {
                res.json::<Invoice>().await.map_err(|e| e.to_string())
            } else {
                Err(format!("Invoice endpoint status {}", res.status()))
            }
        }
    }, 2).await {
        Ok(inv) => inv,
        Err(e) => {
            // Compensate Step 2 & Step 1 concurrently
            let r2 = cfg.client.delete(format!("{}/api/v1/pharmacy/reservations/{}", cfg.pharmacy_url, reservation.id)).send();
            let r1 = cfg.client.delete(format!("{}/api/v1/encounters/{}", cfg.registry_url, encounter.id)).send();
            let (comp_r2, comp_r1) = tokio::join!(r2, r1);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": e,
                "failed_step": "invoice",
                "compensated_steps": [
                    {"step": "reservation", "success": comp_r2.map(|r| r.status().is_success()).unwrap_or(false)},
                    {"step": "encounter", "success": comp_r1.map(|r| r.status().is_success()).unwrap_or(false)}
                ]
            }))).into_response());
        }
    };

    // Step 4: Send Notification
    let notif_req = serde_json::json!({
        "patient_id": payload.patient_id,
        "recipient": "+256701234567",
        "channel": payload.notification_channel.unwrap_or_else(|| "SMS".to_string()),
        "message": format!("Visit confirmed. Encounter ID: {}, Invoice Total: ${:.2}", encounter.id, invoice.total_amount),
    });
    let notif_url = cfg.notifications_url.clone();
    let notif_key = idempotency_key.clone();

    let notification = match call_with_retry(|| {
        let cl = client.clone();
        let u = notif_url.clone();
        let k = notif_key.clone();
        let b = notif_req.clone();
        async move {
            let res = cl.post(format!("{}/api/v1/notifications/send", u))
                .header("X-Idempotency-Key", k)
                .json(&b)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if res.status().is_success() {
                res.json::<Notification>().await.map_err(|e| e.to_string())
            } else {
                Err(format!("Notification endpoint status {}", res.status()))
            }
        }
    }, 2).await {
        Ok(n) => n,
        Err(e) => {
            // Compensate Step 3, 2, 1
            let r3 = cfg.client.post(format!("{}/api/v1/billing/invoices/{}/void", cfg.billing_url, invoice.id)).send();
            let r2 = cfg.client.delete(format!("{}/api/v1/pharmacy/reservations/{}", cfg.pharmacy_url, reservation.id)).send();
            let r1 = cfg.client.delete(format!("{}/api/v1/encounters/{}", cfg.registry_url, encounter.id)).send();
            let (comp_r3, comp_r2, comp_r1) = tokio::join!(r3, r2, r1);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": e,
                "failed_step": "notification",
                "compensated_steps": [
                    {"step": "invoice", "success": comp_r3.map(|r| r.status().is_success()).unwrap_or(false)},
                    {"step": "reservation", "success": comp_r2.map(|r| r.status().is_success()).unwrap_or(false)},
                    {"step": "encounter", "success": comp_r1.map(|r| r.status().is_success()).unwrap_or(false)}
                ]
            }))).into_response());
        }
    };

    Ok(Json(CreateVisitResponse {
        encounter,
        reservation,
        invoice,
        notification,
    }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cfg = AppConfig::from_env();

    let app = Router::new()
        .route("/patients/:id/summary", get(get_patient_summary))
        .route("/visits", post(create_visit))
        .layer(CorsLayer::permissive())
        .with_state(cfg);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8002".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Hand-Rolled Gateway listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
