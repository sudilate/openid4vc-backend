use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use dashmap::DashMap;
use futures::executor::block_on;
use jsonwebtoken::{Algorithm, Header};
use oid4vc_core::{Subject, generate_authorization_code, jwt};
use oid4vc_manager::managers::credential_issuer::CredentialIssuerManager;
use oid4vc_manager::storage::Storage;
use oid4vci::VerifiableCredentialJwt;
use oid4vci::authorization_response::AuthorizationResponse;
use oid4vci::credential_issuer::credential_configurations_supported::CredentialConfigurationsSupportedObject;
use oid4vci::credential_offer::{AuthorizationCode, PreAuthorizedCode};
use oid4vci::credential_response::{
    CredentialResponse, CredentialResponseObject, CredentialResponseType,
};
use oid4vci::token_request::TokenRequest;
use oid4vci::token_response::TokenResponse;
use oid4vci::wallet::PushedAuthorizationResponse;
use reqwest::Url;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::services::key_management::KeyManagementService;

#[derive(Clone)]
pub struct RuntimeStorage {
    inner: Arc<RwLock<RuntimeStorageInner>>,
}

#[derive(Clone)]
struct RuntimeStorageInner {
    tenant_id: Uuid,
    tenant_pool: PgPool,
    credential_configs: HashMap<String, CredentialConfigurationsSupportedObject>,
    authorization_code: String,
    pre_authorized_code: String,
    request_uri: String,
    valid_access_tokens: HashSet<String>,
    state: Option<String>,
}

impl RuntimeStorage {
    pub fn new(
        tenant_id: Uuid,
        tenant_pool: PgPool,
        credential_configs: HashMap<String, CredentialConfigurationsSupportedObject>,
    ) -> Self {
        Self {
            inner: Arc::new(RwLock::new(RuntimeStorageInner {
                tenant_id,
                tenant_pool,
                credential_configs,
                authorization_code: generate_authorization_code(16),
                pre_authorized_code: generate_authorization_code(16),
                request_uri: Uuid::new_v4().to_string(),
                valid_access_tokens: HashSet::new(),
                state: None,
            })),
        }
    }

    pub fn set_pre_authorized_code(&self, value: String) {
        if let Ok(mut inner) = self.inner.write() {
            inner.pre_authorized_code = value;
        }
    }
}

impl Storage for RuntimeStorage {
    fn get_credential_configurations_supported(
        &self,
    ) -> HashMap<String, CredentialConfigurationsSupportedObject> {
        self.inner
            .read()
            .map(|inner| inner.credential_configs.clone())
            .unwrap_or_default()
    }

    fn get_pushed_authorization_response(&self) -> Option<PushedAuthorizationResponse> {
        self.inner
            .read()
            .ok()
            .map(|inner| PushedAuthorizationResponse {
                request_uri: inner.request_uri.clone(),
                expires_in: 3600,
            })
    }

    fn get_authorization_response(&self) -> Option<AuthorizationResponse> {
        self.inner.read().ok().map(|inner| AuthorizationResponse {
            code: inner.authorization_code.clone(),
            state: inner.state.clone(),
        })
    }

    fn get_authorization_code(&self) -> Option<AuthorizationCode> {
        self.inner.read().ok().map(|inner| AuthorizationCode {
            issuer_state: inner.state.clone(),
            authorization_server: None,
        })
    }

    fn get_pre_authorized_code(&self) -> Option<PreAuthorizedCode> {
        self.inner.read().ok().map(|inner| PreAuthorizedCode {
            pre_authorized_code: inner.pre_authorized_code.clone(),
            tx_code: None,
            interval: None,
            authorization_server: None,
        })
    }

    fn get_token_response(&self, token_request: TokenRequest) -> Option<TokenResponse> {
        let mut inner = self.inner.write().ok()?;

        let is_valid = match token_request {
            TokenRequest::AuthorizationCode { code, .. } => code == inner.authorization_code,
            TokenRequest::PreAuthorizedCode {
                pre_authorized_code,
                ..
            } => pre_authorized_code == inner.pre_authorized_code,
        };

        if !is_valid {
            return None;
        }

        let access_token = Uuid::new_v4().to_string();
        inner.valid_access_tokens.insert(access_token.clone());

        Some(TokenResponse {
            access_token,
            token_type: "bearer".to_string(),
            expires_in: Some(3600),
            refresh_token: None,
            scope: None,
            authorization_details: None,
        })
    }

    fn get_credential_response(
        &self,
        access_token: String,
        credential_configuration_id: String,
        subject_did: Url,
        issuer_did: Url,
        signer: oid4vc_core::authentication::subject::SigningSubject,
    ) -> Option<CredentialResponse> {
        let inner = self.inner.read().ok()?;

        if !inner.valid_access_tokens.contains(&access_token) {
            return None;
        }

        if !inner
            .credential_configs
            .contains_key(&credential_configuration_id)
        {
            return None;
        }

        let claims = json!({
            "@context": [
                "https://www.w3.org/2018/credentials/v1",
                "https://www.w3.org/2018/credentials/examples/v1"
            ],
            "id": credential_configuration_id,
            "type": ["VerifiableCredential", "CustomCredential"],
            "issuer": issuer_did,
            "credentialSubject": {
                "id": subject_did,
                "status": "active",
                "issuedBy": "openid4vc-backend"
            }
        });

        let jwt_vc = block_on(async {
            jwt::encode(
                signer.clone(),
                Header::new(Algorithm::EdDSA),
                VerifiableCredentialJwt::builder()
                    .sub(subject_did.clone())
                    .iss(issuer_did.clone())
                    .iat(0)
                    .exp(9_999_999_999_i64)
                    .verifiable_credential(claims.clone())
                    .build()
                    .ok(),
                "did:key",
            )
            .await
            .ok()
        })?;

        let credential_id = Uuid::new_v4().to_string();
        let tenant_pool = inner.tenant_pool.clone();
        let tenant_id = inner.tenant_id;
        let claims_copy = claims.clone();
        let subject_did_text = subject_did.to_string();
        let issuer_did_text = issuer_did.to_string();
        let credential_configuration_id_copy = credential_configuration_id.clone();
        let jwt_vc_copy = jwt_vc.clone();

        let _ = block_on(async move {
            sqlx::query(
                "INSERT INTO issued_credentials (id, tenant_id, credential_id, credential_configuration_id, subject_did, issuer_did, credential_raw, claims, status, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'active', NOW())",
            )
            .bind(Uuid::new_v4())
            .bind(tenant_id)
            .bind(credential_id)
            .bind(credential_configuration_id_copy)
            .bind(subject_did_text)
            .bind(issuer_did_text)
            .bind(jwt_vc_copy)
            .bind(claims_copy)
            .execute(&tenant_pool)
            .await
        });

        Some(CredentialResponse {
            credential: CredentialResponseType::Immediate {
                credentials: vec![CredentialResponseObject { credential: jwt_vc }],
                notification_id: Some(Uuid::new_v4().to_string()),
            },
        })
    }

    fn get_state(&self) -> Option<String> {
        self.inner.read().ok().and_then(|inner| inner.state.clone())
    }

    fn set_state(&mut self, state: String) {
        if let Ok(mut inner) = self.inner.write() {
            inner.state = Some(state);
        }
    }
}

pub struct TenantRuntime {
    pub tenant_id: Uuid,
    pub tenant_slug: String,
    pub manager: CredentialIssuerManager<RuntimeStorage>,
}

#[derive(Clone)]
pub struct Oid4vciRuntimeRegistry {
    runtimes: Arc<DashMap<Uuid, Arc<TenantRuntime>>>,
    admin_pool: PgPool,
    base_url: String,
    key_management: Arc<KeyManagementService>,
}

impl Oid4vciRuntimeRegistry {
    pub fn new(
        admin_pool: PgPool,
        base_url: String,
        key_management: Arc<KeyManagementService>,
    ) -> Self {
        Self {
            runtimes: Arc::new(DashMap::new()),
            admin_pool,
            base_url,
            key_management,
        }
    }

    pub async fn get_by_slug(&self, tenant_slug: &str) -> Result<Arc<TenantRuntime>, AppError> {
        let tenant_row = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT id, slug, database_url FROM tenants WHERE slug = $1 AND is_active = true",
        )
        .bind(tenant_slug)
        .fetch_optional(&self.admin_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("tenant not found: {tenant_slug}")))?;

        if let Some(existing) = self.runtimes.get(&tenant_row.0) {
            return Ok(existing.clone());
        }

        let tenant_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(&tenant_row.2)
            .await
            .map_err(|err| {
                AppError::Internal(format!("unable to connect tenant runtime pool: {err}"))
            })?;
        let credential_configs = load_credential_configs(&tenant_pool).await?;
        let storage = RuntimeStorage::new(tenant_row.0, tenant_pool, credential_configs);
        let subject = self
            .key_management
            .ensure_primary_subject(tenant_row.0)
            .await? as Arc<dyn Subject>;

        let mut manager = CredentialIssuerManager::new(None, storage, subject)
            .map_err(|err| AppError::Internal(err.to_string()))?;

        let issuer_url = Url::parse(&format!("{}/oid4vci/{}", self.base_url, tenant_row.1))
            .map_err(|err| AppError::Internal(err.to_string()))?;
        manager.credential_issuer.metadata.credential_issuer = issuer_url.clone();
        manager.credential_issuer.metadata.credential_endpoint = issuer_url
            .join("credential")
            .map_err(|err| AppError::Internal(err.to_string()))?;
        manager.credential_issuer.metadata.notification_endpoint = Some(
            issuer_url
                .join("notification")
                .map_err(|err| AppError::Internal(err.to_string()))?,
        );
        manager
            .credential_issuer
            .authorization_server_metadata
            .issuer = issuer_url.clone();
        manager
            .credential_issuer
            .authorization_server_metadata
            .pushed_authorization_request_endpoint = Some(
            issuer_url
                .join("par")
                .map_err(|err| AppError::Internal(err.to_string()))?,
        );
        manager
            .credential_issuer
            .authorization_server_metadata
            .authorization_endpoint = Some(
            issuer_url
                .join("authorize")
                .map_err(|err| AppError::Internal(err.to_string()))?,
        );
        manager
            .credential_issuer
            .authorization_server_metadata
            .token_endpoint = Some(
            issuer_url
                .join("token")
                .map_err(|err| AppError::Internal(err.to_string()))?,
        );

        let runtime = Arc::new(TenantRuntime {
            tenant_id: tenant_row.0,
            tenant_slug: tenant_row.1.clone(),
            manager,
        });

        self.runtimes.insert(tenant_row.0, runtime.clone());
        Ok(runtime)
    }

    pub async fn set_pre_authorized_code(
        &self,
        tenant_slug: &str,
        code: String,
    ) -> Result<(), AppError> {
        let runtime = self.get_by_slug(tenant_slug).await?;
        runtime.manager.storage.set_pre_authorized_code(code);
        Ok(())
    }
}

async fn load_credential_configs(
    tenant_pool: &PgPool,
) -> Result<HashMap<String, CredentialConfigurationsSupportedObject>, AppError> {
    let rows = sqlx::query_as::<_, (String, serde_json::Value)>(
        "SELECT name, schema FROM credential_definitions WHERE is_active = true",
    )
    .fetch_all(tenant_pool)
    .await
    .unwrap_or_default();

    let mut map = HashMap::new();

    for (name, schema) in rows {
        if let Ok(value) = serde_json::from_value::<CredentialConfigurationsSupportedObject>(schema)
        {
            map.insert(name, value);
        }
    }

    if map.is_empty() {
        let default_config: CredentialConfigurationsSupportedObject =
            serde_json::from_value(json!({
                "format": "jwt_vc_json",
                "credential_definition": {
                    "type": ["VerifiableCredential", "CustomCredential"]
                },
                "scope": "custom_credential",
                "cryptographic_binding_methods_supported": ["did:key", "did:web", "did:ion"],
                "credential_signing_alg_values_supported": ["EdDSA"],
                "proof_types_supported": {
                    "jwt": {
                        "proof_signing_alg_values_supported": ["EdDSA"]
                    }
                }
            }))
            .map_err(|err| AppError::Internal(err.to_string()))?;

        map.insert("CustomCredential_JWT".to_string(), default_config);
    }

    Ok(map)
}
