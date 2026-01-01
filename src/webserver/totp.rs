//! TOTP (Time-based One-Time Password) utilities for 2FA authentication
//!
//! Provides functions for generating and verifying TOTP codes using the standard
//! algorithm: SHA1, 6 digits, 30-second window, with 1-step tolerance for clock drift.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use qrcode::{render::svg, QrCode};
use rand::Rng;
use totp_rs::{Algorithm, Secret, TOTP};

/// TOTP configuration constants
const TOTP_ALGORITHM: Algorithm = Algorithm::SHA1;
const TOTP_DIGITS: usize = 6;
const TOTP_STEP: u64 = 30;
const TOTP_SKEW: u8 = 1; // Allow ±1 step (±30 seconds) for clock drift
const SECRET_LENGTH: usize = 20; // 160 bits for SHA1

/// Generate a new random TOTP secret
///
/// Returns a base32-encoded secret suitable for storage and use with authenticator apps.
pub fn generate_secret() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..SECRET_LENGTH).map(|_| rng.gen::<u8>()).collect();

    // Use the Secret type to properly encode as base32
    let secret = Secret::Raw(bytes);
    secret.to_encoded().to_string()
}

/// Create a TOTP instance from a base32-encoded secret
fn create_totp(secret: &str, account: &str, issuer: &str) -> Result<TOTP, String> {
    let secret = Secret::Encoded(secret.to_string())
        .to_bytes()
        .map_err(|e| format!("Invalid secret: {}", e))?;

    TOTP::new(
        TOTP_ALGORITHM,
        TOTP_DIGITS,
        TOTP_SKEW,
        TOTP_STEP,
        secret,
        Some(issuer.to_string()),
        account.to_string(),
    )
    .map_err(|e| format!("Failed to create TOTP: {}", e))
}

/// Generate an otpauth:// URI for use with authenticator apps
///
/// This URI can be encoded as a QR code for easy setup.
pub fn get_totp_uri(secret: &str, account: &str) -> Result<String, String> {
    let totp = create_totp(secret, account, "ScreenerBot")?;
    Ok(totp.get_url())
}

/// Verify a TOTP code against the secret
///
/// Allows for 1-step clock drift (±30 seconds).
/// Returns true if the code is valid.
pub fn verify_totp(secret: &str, code: &str) -> Result<bool, String> {
    // Validate code format (must be 6 digits)
    if code.len() != TOTP_DIGITS || !code.chars().all(|c| c.is_ascii_digit()) {
        return Ok(false);
    }

    let totp = create_totp(secret, "user", "ScreenerBot")?;

    // Use current time for verification
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Time error: {}", e))?
        .as_secs();

    Ok(totp.check(code, time))
}

/// Generate the current TOTP code (for testing/debugging only)
#[allow(dead_code)]
pub fn generate_current_code(secret: &str) -> Result<String, String> {
    let totp = create_totp(secret, "user", "ScreenerBot")?;

    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Time error: {}", e))?
        .as_secs();

    Ok(totp.generate(time))
}

/// Generate a QR code as a data URL (data:image/svg+xml;base64,...)
///
/// The QR code encodes the otpauth:// URI for easy setup with authenticator apps.
pub fn generate_qr_data_url(secret: &str, account: &str) -> Result<String, String> {
    let uri = get_totp_uri(secret, account)?;

    let code = QrCode::new(uri.as_bytes()).map_err(|e| format!("QR generation failed: {}", e))?;

    let svg_string = code
        .render()
        .min_dimensions(200, 200)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();

    let encoded = BASE64.encode(svg_string.as_bytes());
    Ok(format!("data:image/svg+xml;base64,{}", encoded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret() {
        let secret = generate_secret();
        // Base32 encoded 20-byte secret should be 32 characters
        assert!(!secret.is_empty());
        assert!(secret.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_get_totp_uri() {
        let secret = generate_secret();
        let uri = get_totp_uri(&secret, "test@example.com").unwrap();
        assert!(uri.starts_with("otpauth://totp/"));
        assert!(uri.contains("ScreenerBot"));
    }

    #[test]
    fn test_generate_qr_data_url() {
        let secret = generate_secret();
        let data_url = generate_qr_data_url(&secret, "test@example.com").unwrap();
        assert!(data_url.starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn test_verify_totp_invalid_format() {
        let secret = generate_secret();
        // Invalid formats should return false, not error
        assert!(!verify_totp(&secret, "12345").unwrap()); // Too short
        assert!(!verify_totp(&secret, "1234567").unwrap()); // Too long
        assert!(!verify_totp(&secret, "abcdef").unwrap()); // Not digits
    }
}
