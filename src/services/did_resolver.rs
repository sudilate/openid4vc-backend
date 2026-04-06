use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::error::AppError;

#[async_trait]
pub trait DidResolver: Send + Sync {
    fn method(&self) -> &'static str;
    async fn resolve(&self, did: &str) -> Result<Value, AppError>;
}

#[derive(Clone, Default)]
pub struct DidResolverRegistry {
    resolvers: HashMap<String, Arc<dyn DidResolver>>,
}

impl DidResolverRegistry {
    pub fn with_defaults() -> Self {
        let mut registry = Self::default();
        registry.register(Arc::new(DidKeyResolver));
        registry.register(Arc::new(DidWebResolver {
            client: Client::new(),
        }));
        registry.register(Arc::new(DidIonResolver {
            client: Client::new(),
        }));
        registry
    }

    pub fn register(&mut self, resolver: Arc<dyn DidResolver>) {
        self.resolvers
            .insert(resolver.method().to_string(), resolver.clone());
    }

    pub async fn resolve(&self, did: &str) -> Result<Value, AppError> {
        let method = did
            .split(':')
            .nth(1)
            .ok_or_else(|| AppError::BadRequest("invalid did format".to_string()))?;

        let resolver = self
            .resolvers
            .get(method)
            .ok_or_else(|| AppError::BadRequest(format!("unsupported did method: {method}")))?;

        resolver.resolve(did).await
    }
}

pub struct DidKeyResolver;

#[async_trait]
impl DidResolver for DidKeyResolver {
    fn method(&self) -> &'static str {
        "key"
    }

    async fn resolve(&self, did: &str) -> Result<Value, AppError> {
        if !did.starts_with("did:key:") {
            return Err(AppError::BadRequest("invalid did:key value".to_string()));
        }

        Ok(serde_json::json!({
            "id": did,
            "@context": ["https://www.w3.org/ns/did/v1"],
            "verificationMethod": [],
            "note": "did:key resolution placeholder; add full key decoding for production"
        }))
    }
}

pub struct DidWebResolver {
    client: Client,
}

#[async_trait]
impl DidResolver for DidWebResolver {
    fn method(&self) -> &'static str {
        "web"
    }

    async fn resolve(&self, did: &str) -> Result<Value, AppError> {
        let parts: Vec<&str> = did.split(':').collect();
        if parts.len() < 3 {
            return Err(AppError::BadRequest("invalid did:web value".to_string()));
        }

        let host = parts[2];
        let extra_path = if parts.len() > 3 {
            format!("/{}/did.json", parts[3..].join("/"))
        } else {
            "/.well-known/did.json".to_string()
        };
        let url = format!("https://{host}{extra_path}");

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

        let document = response
            .json::<Value>()
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

        Ok(document)
    }
}

pub struct DidIonResolver {
    client: Client,
}

#[async_trait]
impl DidResolver for DidIonResolver {
    fn method(&self) -> &'static str {
        "ion"
    }

    async fn resolve(&self, did: &str) -> Result<Value, AppError> {
        let encoded = urlencoding::encode(did);
        let url = format!("https://discover.did.msidentity.com/1.0/identifiers/{encoded}");

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

        let payload = response
            .json::<Value>()
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

        Ok(payload)
    }
}
