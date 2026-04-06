use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::{Argon2, PasswordHash, PasswordVerifier};

pub fn hash_api_key(raw_key: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(raw_key.as_bytes(), &salt)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?
        .to_string();
    Ok(hash)
}

pub fn verify_api_key(raw_key: &str, encoded_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(encoded_hash) {
        Ok(value) => value,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(raw_key.as_bytes(), &parsed_hash)
        .is_ok()
}
