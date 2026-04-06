use argon2::{Argon2, PasswordHash, PasswordVerifier};

pub fn verify_api_key(raw_key: &str, encoded_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(encoded_hash) {
        Ok(value) => value,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(raw_key.as_bytes(), &parsed_hash)
        .is_ok()
}
