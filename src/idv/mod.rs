use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSession {
    pub provider_reference: String,
    pub redirect_url: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationEvent {
    pub provider_reference: String,
    pub status: String,
    pub payload: serde_json::Value,
}

#[async_trait]
pub trait IdvProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn start_verification(
        &self,
        subject_reference: &str,
        callback_url: &str,
    ) -> anyhow::Result<VerificationSession>;
    async fn verify_webhook_signature(
        &self,
        payload: &[u8],
        signature: &str,
    ) -> anyhow::Result<bool>;
    async fn parse_event(&self, payload: &[u8]) -> anyhow::Result<VerificationEvent>;
}

#[derive(Default, Clone)]
pub struct IdvRegistry {
    providers: HashMap<String, Arc<dyn IdvProvider>>,
}

impl IdvRegistry {
    pub fn register(&mut self, provider: Arc<dyn IdvProvider>) {
        self.providers.insert(provider.name().to_string(), provider);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn IdvProvider>> {
        self.providers.get(name).cloned()
    }
}
