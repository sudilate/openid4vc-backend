use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use did_key::{Ed25519KeyPair, generate};
use rand::RngCore;
use reqwest::Client;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use oid4vc_core::Subject;
use oid4vc_manager::methods::key_method::KeySubject;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub enum KeyBackend {
    File { base_path: PathBuf },
    Vault { addr: String, token: String },
}

#[derive(Clone)]
pub struct KeyManagementService {
    pub admin_pool: PgPool,
    pub backend: KeyBackend,
    http: Client,
}

impl KeyManagementService {
    pub fn new(admin_pool: PgPool, backend: KeyBackend) -> Self {
        Self {
            admin_pool,
            backend,
            http: Client::new(),
        }
    }

    pub async fn ensure_primary_subject(
        &self,
        tenant_id: Uuid,
    ) -> Result<Arc<dyn Subject>, AppError> {
        let maybe_seed = sqlx::query_scalar::<_, Option<String>>(
            "SELECT seed_hex FROM tenant_issuer_keys WHERE tenant_id = $1 AND is_primary = true AND status = 'active' ORDER BY created_at DESC LIMIT 1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.admin_pool)
        .await?
        .flatten();

        let seed = match maybe_seed {
            Some(seed_hex) => decode_seed(&seed_hex)?,
            None => self.create_primary_key(tenant_id).await?,
        };

        let keypair = generate::<Ed25519KeyPair>(Some(&seed));
        Ok(Arc::new(KeySubject::from_keypair(keypair, None)))
    }

    pub async fn rotate_primary_key(&self, tenant_id: Uuid) -> Result<String, AppError> {
        sqlx::query(
            "UPDATE tenant_issuer_keys SET is_primary = false, status = 'rotated', rotated_at = NOW() WHERE tenant_id = $1 AND is_primary = true",
        )
        .bind(tenant_id)
        .execute(&self.admin_pool)
        .await?;

        let seed = self.create_primary_key(tenant_id).await?;
        Ok(hex::encode(seed))
    }

    async fn create_primary_key(&self, tenant_id: Uuid) -> Result<[u8; 32], AppError> {
        let mut seed = [0_u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);

        let key_id = format!("issuer-key-{}", Uuid::new_v4());
        let seed_hex = hex::encode(seed);
        let did = format!("did:key:{}", key_id);

        self.persist_secret(tenant_id, &key_id, &seed_hex).await?;

        sqlx::query(
            "INSERT INTO tenant_issuer_keys (id, tenant_id, key_id, did, backend, algorithm, seed_hex, status, is_primary, created_at)
             VALUES ($1, $2, $3, $4, $5, 'EdDSA', $6, 'active', true, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(&key_id)
        .bind(&did)
        .bind(self.backend_name())
        .bind(seed_hex)
        .execute(&self.admin_pool)
        .await?;

        Ok(seed)
    }

    async fn persist_secret(
        &self,
        tenant_id: Uuid,
        key_id: &str,
        seed_hex: &str,
    ) -> Result<(), AppError> {
        match &self.backend {
            KeyBackend::File { base_path } => {
                let path = base_path.join(tenant_id.to_string());
                fs::create_dir_all(&path).map_err(|err| AppError::Internal(err.to_string()))?;
                fs::write(path.join(format!("{key_id}.seed")), seed_hex)
                    .map_err(|err| AppError::Internal(err.to_string()))?;
                Ok(())
            }
            KeyBackend::Vault { addr, token } => {
                let endpoint = format!("{addr}/v1/secret/data/openid4vc/{tenant_id}/{key_id}");
                self.http
                    .post(endpoint)
                    .header("x-vault-token", token)
                    .json(&json!({"data": {"seed_hex": seed_hex}}))
                    .send()
                    .await
                    .map_err(|err| AppError::Internal(format!("vault write failed: {err}")))?;
                Ok(())
            }
        }
    }

    fn backend_name(&self) -> &'static str {
        match self.backend {
            KeyBackend::File { .. } => "file",
            KeyBackend::Vault { .. } => "vault",
        }
    }
}

fn decode_seed(seed_hex: &str) -> Result<[u8; 32], AppError> {
    let raw = hex::decode(seed_hex).map_err(|err| AppError::Internal(err.to_string()))?;
    if raw.len() != 32 {
        return Err(AppError::Internal("invalid seed length".to_string()));
    }
    let mut out = [0_u8; 32];
    out.copy_from_slice(&raw);
    Ok(out)
}
