//! Locale resolution from the stored setting, the request and the operating system.

use super::registry::{available_langids, source_locale};
use crate::logger::{self, LogTag};
use axum::http::{header::ACCEPT_LANGUAGE, HeaderMap};
use fluent_langneg::{accepted_languages, negotiate_languages, NegotiationStrategy};
use unic_langid::LanguageIdentifier;

/// Setting value that defers to the request and operating-system languages.
pub const SYSTEM_SETTING: &str = "system";

fn negotiate(requested: &[LanguageIdentifier]) -> Option<LanguageIdentifier> {
    if requested.is_empty() {
        return None;
    }
    negotiate_languages(
        requested,
        available_langids(),
        None,
        NegotiationStrategy::Lookup,
    )
    .first()
    .map(|id| (*id).clone())
}

fn source_langid() -> LanguageIdentifier {
    source_locale()
        .parse()
        .unwrap_or_else(|_| LanguageIdentifier::default())
}

/// Resolve the display locale.
///
/// Order: an explicit `setting` that matches a registered locale, then the
/// `Accept-Language` header, then the operating-system locales, then the source
/// locale. An unrecognized setting is reported and treated as `"system"`.
pub fn resolve_locale(setting: &str, accept_language: Option<&str>) -> LanguageIdentifier {
    if setting != SYSTEM_SETTING {
        let explicit = setting
            .parse::<LanguageIdentifier>()
            .ok()
            .and_then(|id| negotiate(&[id]));
        match explicit {
            Some(id) => return id,
            None => logger::warning(
                LogTag::System,
                &format!(
                    "Unknown display language setting {setting:?}; following the system language"
                ),
            ),
        }
    }

    if let Some(header) = accept_language {
        if let Some(id) = negotiate(&accepted_languages::parse(header)) {
            return id;
        }
    }

    let system: Vec<LanguageIdentifier> = sys_locale::get_locales()
        .filter_map(|tag| tag.parse().ok())
        .collect();
    negotiate(&system).unwrap_or_else(source_langid)
}

/// Resolve the display locale for an HTTP request using the configured
/// dashboard language.
pub fn resolve_request_locale(headers: &HeaderMap) -> LanguageIdentifier {
    let setting = if crate::config::is_config_initialized() {
        crate::config::with_config(|cfg| cfg.gui.dashboard.interface.language.clone())
    } else {
        SYSTEM_SETTING.to_string()
    };
    let accept_language = headers
        .get(ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok());
    resolve_locale(&setting, accept_language)
}
