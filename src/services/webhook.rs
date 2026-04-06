use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEvent {
    pub specversion: String,
    pub event_type: String,
    pub source: String,
    pub id: String,
    pub time: DateTime<Utc>,
    pub datacontenttype: String,
    pub subject: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct WebhookDeliveryService {
    client: reqwest::Client,
}

impl WebhookDeliveryService {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn deliver(
        &self,
        target_url: &str,
        secret: &str,
        event: &CloudEvent,
    ) -> anyhow::Result<StatusCode> {
        let payload = serde_json::to_vec(event)?;
        let signature = sign_payload(secret, &payload);

        let response = self
            .client
            .post(target_url)
            .header("content-type", "application/json")
            .header("x-signature", signature)
            .body(payload)
            .send()
            .await?;

        Ok(response.status())
    }
}

fn sign_payload(secret: &str, payload: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac key initialization");
    mac.update(payload);
    let result = mac.finalize().into_bytes();
    format!("sha256={}", hex::encode(result))
}
