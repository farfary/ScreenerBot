//! TOTP utilities for Telegram 2FA authentication
//!
//! Wraps the webserver TOTP module for telegram-specific usage.
//! Uses the same TOTP secret as the dashboard lockscreen for unified 2FA.

/// Generate a new random TOTP secret
pub fn generate_secret() -> String {
    crate::webserver::totp::generate_secret()
}

/// Generate an otpauth:// URI for use with authenticator apps
pub fn generate_uri(secret: &str, account: &str) -> Result<String, String> {
    crate::webserver::totp::get_totp_uri(secret, account)
}

/// Verify a TOTP code against the secret
pub fn verify_code(secret: &str, code: &str) -> Result<bool, String> {
    crate::webserver::totp::verify_totp(secret, code)
}

/// Generate a QR code as a data URL
pub fn generate_qr_data_url(secret: &str, account: &str) -> Result<String, String> {
    crate::webserver::totp::generate_qr_data_url(secret, account)
}
