//! BLAKE3 password hashing for lockscreen authentication.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use super::{Error, Result};

/// Generate a random 16-byte salt for password hashing.
pub fn generate_password_salt() -> String {
    let salt: [u8; 16] = rand::random();
    BASE64.encode(salt)
}

/// Hash a password using BLAKE3 with salt.
///
/// Uses BLAKE3 keyed hash for password hashing which provides:
/// - Fast hashing (important for PIN verification UX)
/// - Cryptographic security
/// - Resistance to rainbow table attacks (via salt)
pub fn hash_password(password: &str, salt: &str) -> Result<String> {
    let salt_bytes = BASE64
        .decode(salt)
        .map_err(|error| Error::InvalidEncoding {
            field: "salt",
            detail: error.to_string(),
        })?;

    let mut key = [0u8; 32];
    let mut key_hasher = blake3::Hasher::new();
    key_hasher.update(&salt_bytes);
    key_hasher.update(b"screenerbot-lockscreen-v1");
    let key_hash = key_hasher.finalize();
    key.copy_from_slice(key_hash.as_bytes());

    let mut hasher = blake3::Hasher::new_keyed(&key);
    hasher.update(password.as_bytes());
    let hash = hasher.finalize();

    Ok(BASE64.encode(hash.as_bytes()))
}

/// Verify a password against a stored hash using constant-time comparison.
pub fn verify_password(password: &str, salt: &str, stored_hash: &str) -> bool {
    let attempt_hash = match hash_password(password, salt) {
        Ok(h) => h,
        Err(_) => return false,
    };

    constant_time_compare(attempt_hash.as_bytes(), stored_hash.as_bytes())
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let result = a
        .iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y));

    result == 0
}
