// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! TOTP (Time-based One-Time Password) utilities for 2FA authentication
//!
//! Provides functions for generating and verifying TOTP codes using the standard
//! algorithm: SHA1, 6 digits, 30-second window, with 1-step tolerance for clock drift.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use qrcode::{render::svg, QrCode};
use totp_rs::{Algorithm, Secret, TOTP};

use super::{Error, Result};

/// TOTP configuration constants
const TOTP_ALGORITHM: Algorithm = Algorithm::SHA1;
const TOTP_DIGITS: usize = 6;
const TOTP_STEP: u64 = 30;
const TOTP_SKEW: u8 = 1; // Allow ±1 step (±30 seconds) for clock drift

/// Generate a new random TOTP secret
///
/// Returns a base32-encoded secret suitable for storage and use with authenticator apps.
pub fn generate_secret() -> String {
    Secret::generate().to_base32()
}

/// Create a TOTP instance from a base32-encoded secret
fn create_totp(secret: &str, account: &str, issuer: &str) -> Result<TOTP> {
    let secret = Secret::try_from_base32(secret)
        .map_err(|e| Error::TotpSecret {
            operation: "decode",
            detail: e.to_string(),
        })?
        .as_bytes()
        .to_vec();

    TOTP::new(
        TOTP_ALGORITHM,
        TOTP_DIGITS,
        TOTP_SKEW,
        TOTP_STEP,
        secret,
        Some(issuer.to_string()),
        account.to_string(),
    )
    .map_err(|e| Error::TotpSecret {
        operation: "create",
        detail: e.to_string(),
    })
}

/// Generate an otpauth:// URI for use with authenticator apps
///
/// This URI can be encoded as a QR code for easy setup.
pub fn get_totp_uri(secret: &str, account: &str) -> Result<String> {
    let totp = create_totp(secret, account, "ScreenerBot")?;
    Ok(totp.get_url())
}

/// Verify a TOTP code against the secret
///
/// Allows for 1-step clock drift (±30 seconds).
/// Returns true if the code is valid.
pub fn verify_totp(secret: &str, code: &str) -> Result<bool> {
    // Validate code format (must be 6 digits)
    if code.len() != TOTP_DIGITS || !code.chars().all(|c| c.is_ascii_digit()) {
        return Ok(false);
    }

    let totp = create_totp(secret, "user", "ScreenerBot")?;

    // Use current time for verification
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::TotpSecret {
            operation: "read the system clock for",
            detail: e.to_string(),
        })?
        .as_secs();

    Ok(totp.check(code, time).is_some())
}

/// Generate an SVG QR code for an arbitrary public value.
pub fn generate_qr_svg(value: &str) -> Result<String> {
    let code = QrCode::new(value.as_bytes()).map_err(|e| Error::TotpQrCode {
        detail: e.to_string(),
    })?;

    Ok(code
        .render()
        .min_dimensions(200, 200)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

/// Generate a QR code data URL for an arbitrary public value.
pub fn generate_qr_data_url_for_value(value: &str) -> Result<String> {
    let svg_string = generate_qr_svg(value)?;
    let encoded = BASE64.encode(svg_string.as_bytes());
    Ok(format!("data:image/svg+xml;base64,{encoded}"))
}

/// Generate a QR code as a data URL (data:image/svg+xml;base64,...)
///
/// The QR code encodes the otpauth:// URI for easy setup with authenticator apps.
pub fn generate_qr_data_url(secret: &str, account: &str) -> Result<String> {
    let uri = get_totp_uri(secret, account)?;
    generate_qr_data_url_for_value(&uri)
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
    fn generated_code_verifies_with_stored_secret() {
        let secret = generate_secret();
        let code = create_totp(&secret, "user", "ScreenerBot")
            .unwrap()
            .generate_current();
        assert!(verify_totp(&secret, &code.to_string()).unwrap());
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
