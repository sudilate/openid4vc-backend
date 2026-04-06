use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    pub database: DatabaseSettings,
    pub redis: RedisSettings,
    pub security: SecuritySettings,
    pub key_management: KeyManagementSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseSettings {
    pub admin_url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisSettings {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecuritySettings {
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub jwt_public_key_pem: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeyManagementSettings {
    pub backend: String,
    pub file_base_path: String,
    pub vault_addr: Option<String>,
    pub vault_token: Option<String>,
}

impl Settings {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let builder = config::Config::builder()
            .add_source(config::File::with_name("config/base").required(false))
            .add_source(config::Environment::with_prefix("OID4VC_BACKEND").separator("__"));

        builder.build()?.try_deserialize()
    }
}
