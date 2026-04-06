use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub tenant_id: Uuid,
    pub role: String,
    pub aud: String,
    pub iss: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn decode_and_validate_jwt(
    token: &str,
    public_key_pem: &str,
    issuer: &str,
    audience: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[issuer]);
    validation.set_audience(&[audience]);

    let decoded = decode::<Claims>(
        token,
        &DecodingKey::from_rsa_pem(public_key_pem.as_bytes())?,
        &validation,
    )?;

    Ok(decoded.claims)
}
