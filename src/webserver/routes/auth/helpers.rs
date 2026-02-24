//! Authentication helper utilities

use axum::{extract::Request, http::header, http::HeaderMap};

use super::types::SESSION_COOKIE_NAME;

/// Get a cookie value from headers by name
pub fn get_cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|cookies| cookies.to_str().ok())
        .and_then(|cookies| {
            for cookie in cookies.split(';') {
                let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                if parts.len() == 2 && parts[0] == name {
                    return Some(parts[1].to_string());
                }
            }
            None
        })
}

/// Build session cookie string with proper attributes
pub fn build_session_cookie(token: &str, timeout_secs: u64) -> String {
    let max_age = if timeout_secs > 0 {
        format!("; Max-Age={timeout_secs}")
    } else {
        // Session cookie (expires when browser closes) - no Max-Age
        String::new()
    };

    format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict{}",
        SESSION_COOKIE_NAME, token, max_age
    )
}

/// Extract session token from request cookies (for use in middleware)
pub fn extract_session_token(request: &Request) -> Option<String> {
    request
        .headers()
        .get(header::COOKIE)
        .and_then(|cookies| cookies.to_str().ok())
        .and_then(|cookies| {
            for cookie in cookies.split(';') {
                let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                if parts.len() == 2 && parts[0] == SESSION_COOKIE_NAME {
                    return Some(parts[1].to_string());
                }
            }
            None
        })
}
