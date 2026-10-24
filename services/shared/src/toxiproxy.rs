use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub name: String,
    pub listen: String,
    pub upstream: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToxicAttribute {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jitter: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToxicPayload {
    pub name: String,
    pub r#type: String, // "latency", "bandwidth", "slow_close", "timeout", "slicer"
    pub stream: String, // "downstream" or "upstream"
    pub toxicity: f32,  // 0.0 to 1.0 (probability)
    pub attributes: serde_json::Value,
}

#[derive(Clone)]
pub struct ToxiproxyClient {
    base_url: String,
    client: Client,
}

impl ToxiproxyClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
        }
    }

    /// Reset all active toxics
    pub async fn reset(&self) -> Result<(), reqwest::Error> {
        let _ = self
            .client
            .post(format!("{}/reset", self.base_url))
            .send()
            .await?;
        Ok(())
    }

    /// Create or ensure a proxy exists
    pub async fn create_proxy(
        &self,
        name: &str,
        listen: &str,
        upstream: &str,
    ) -> Result<(), reqwest::Error> {
        let body = serde_json::json!({
            "name": name,
            "listen": listen,
            "upstream": upstream,
            "enabled": true
        });

        let _ = self
            .client
            .post(format!("{}/proxies", self.base_url))
            .json(&body)
            .send()
            .await?;
        Ok(())
    }

    /// Inject latency and jitter toxic on a proxy
    pub async fn inject_latency(
        &self,
        proxy_name: &str,
        latency_ms: u64,
        jitter_ms: u64,
        toxicity: f32,
    ) -> Result<(), reqwest::Error> {
        let payload = ToxicPayload {
            name: format!("{}_latency", proxy_name),
            r#type: "latency".to_string(),
            stream: "downstream".to_string(),
            toxicity,
            attributes: serde_json::json!({
                "latency": latency_ms,
                "jitter": jitter_ms
            }),
        };

        let _ = self
            .client
            .post(format!("{}/proxies/{}/toxics", self.base_url, proxy_name))
            .json(&payload)
            .send()
            .await?;
        Ok(())
    }

    /// Disable proxy (kill switch)
    pub async fn set_enabled(&self, proxy_name: &str, enabled: bool) -> Result<(), reqwest::Error> {
        let body = serde_json::json!({ "enabled": enabled });
        let _ = self
            .client
            .post(format!("{}/proxies/{}", self.base_url, proxy_name))
            .json(&body)
            .send()
            .await?;
        Ok(())
    }
}
