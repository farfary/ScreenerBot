use axum::{
    extract::Path,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};

use super::types::LocalesResponse;
use crate::i18n;
use crate::webserver::utils::{error_response, no_store_response};

/// Registered locales plus the locale and setting in effect for this request.
pub async fn get_locales(headers: HeaderMap) -> Response {
    let setting = if crate::config::is_config_initialized() {
        crate::config::with_config(|cfg| cfg.gui.dashboard.interface.language.clone())
    } else {
        i18n::SYSTEM_SETTING.to_string()
    };
    Json(LocalesResponse {
        resolved: i18n::resolve_request_locale(&headers).to_string(),
        setting,
        source: i18n::source_locale(),
        locales: i18n::available_locales().to_vec(),
    })
    .into_response()
}

/// Fluent source for one locale, limited to the domains the dashboard renders.
pub async fn get_dashboard_catalog(Path(locale): Path<String>) -> Response {
    match i18n::dashboard_catalog(&locale) {
        Some(catalog) => no_store_response("text/plain; charset=utf-8", catalog),
        None => error_response(
            axum::http::StatusCode::NOT_FOUND,
            "LOCALE_NOT_FOUND",
            "Locale is not registered",
            Some(&locale),
        ),
    }
}
