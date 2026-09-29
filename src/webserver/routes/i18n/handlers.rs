use axum::{
    extract::Path,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};

use super::types::{CatalogEntry, CatalogPayload, LocalesResponse};
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

/// Classic script that installs the dashboard catalogs for one locale as
/// `window.__SCREENERBOT_L10N__`, in fallback order from least to most specific.
pub async fn get_catalog_script(Path(locale): Path<String>) -> Response {
    let langid = i18n::display_locale_info(&locale)
        .and_then(|_| locale.parse::<i18n::LanguageIdentifier>().ok());
    let Some(langid) = langid else {
        return error_response(
            axum::http::StatusCode::NOT_FOUND,
            "LOCALE_NOT_FOUND",
            "Locale is not registered",
            Some(&locale),
        );
    };
    let payload = CatalogPayload {
        locale: &locale,
        intl_locale: format!("{locale}-u-nu-latn"),
        dir: i18n::text_direction(&langid),
        pseudo: i18n::PseudoLocale::from_code(&locale).map(|p| p.kind()),
        source: i18n::source_locale(),
        catalogs: i18n::dashboard_catalog_chain(&langid)
            .into_iter()
            .map(|(locale, ftl)| CatalogEntry { locale, ftl })
            .collect(),
    };
    match serde_json::to_string(&payload) {
        Ok(json) => no_store_response(
            "application/javascript; charset=utf-8",
            format!("window.__SCREENERBOT_L10N__ = {json};"),
        ),
        Err(err) => error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "CATALOG_ENCODE_FAILED",
            "Catalog could not be encoded",
            Some(&err.to_string()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{header, StatusCode};

    async fn body_of(response: Response) -> (StatusCode, String, String) {
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        (
            status,
            content_type,
            String::from_utf8(bytes.to_vec()).expect("utf8"),
        )
    }

    #[tokio::test]
    async fn catalog_script_is_an_assignment_of_json_in_fallback_order() {
        let (status, content_type, body) =
            body_of(get_catalog_script(Path("en".to_string())).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type, "application/javascript; charset=utf-8");
        let json = body
            .strip_prefix("window.__SCREENERBOT_L10N__ = ")
            .and_then(|rest| rest.strip_suffix(';'))
            .expect("assignment wrapper");
        let value: serde_json::Value = serde_json::from_str(json).expect("json");
        assert_eq!(value["locale"], "en");
        assert_eq!(value["intlLocale"], "en-u-nu-latn");
        assert_eq!(value["dir"], "ltr");
        assert_eq!(value["source"], "en");
        let catalogs = value["catalogs"].as_array().expect("catalogs");
        assert_eq!(catalogs[0]["locale"], "en");
        assert!(catalogs[0]["ftl"]
            .as_str()
            .unwrap()
            .contains("common-loading"));
    }

    #[tokio::test]
    async fn unregistered_locale_is_not_found() {
        let (status, _, _) = body_of(get_catalog_script(Path("xx".to_string())).await).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    async fn catalog_json(locale: &str) -> serde_json::Value {
        let (status, _, body) = body_of(get_catalog_script(Path(locale.to_string())).await).await;
        assert_eq!(status, StatusCode::OK);
        let json = body
            .strip_prefix("window.__SCREENERBOT_L10N__ = ")
            .and_then(|rest| rest.strip_suffix(';'))
            .expect("assignment wrapper");
        serde_json::from_str(json).expect("json")
    }

    #[tokio::test]
    async fn pseudo_bidi_catalog_is_source_only_and_rtl() {
        let value = catalog_json("ar-XB").await;
        assert_eq!(value["locale"], "ar-XB");
        assert_eq!(value["intlLocale"], "ar-XB-u-nu-latn");
        assert_eq!(value["dir"], "rtl");
        assert_eq!(value["pseudo"], "bidi");
        assert_eq!(value["source"], "en");
        assert_eq!(value["catalogs"].as_array().expect("catalogs").len(), 1);
        assert_eq!(value["catalogs"][0]["locale"], "en");
    }

    #[tokio::test]
    async fn pseudo_accented_catalog_is_ltr() {
        let value = catalog_json("en-XA").await;
        assert_eq!(value["dir"], "ltr");
        assert_eq!(value["pseudo"], "accented");
    }

    #[tokio::test]
    async fn real_locale_catalog_has_no_pseudo_field() {
        let value = catalog_json("en").await;
        assert!(value.get("pseudo").is_none());
    }
}
