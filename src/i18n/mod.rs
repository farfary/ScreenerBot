// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Localization foundation: locale registry, negotiation, Fluent formatting and
//! the wire type for backend-authored display text.
//!
//! Catalogs live under `locales/` and are validated and embedded by `build.rs`.
//! The generated `ids` module exposes one `MessageId` constant per source-locale
//! message, so call sites reference catalog keys by symbol.

mod html;
mod localizer;
mod markup;
mod negotiate;
mod pseudo;
mod registry;
mod text;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_html;
#[cfg(test)]
mod tests_markup;
#[cfg(test)]
mod tests_pseudo;

use serde::{Serialize, Serializer};
use std::fmt;

pub use html::{localize_html, L10N_ATTRIBUTES};
pub use localizer::{
    dashboard_catalog, dashboard_catalog_chain, format, format_en, format_message, LocalizedMessage,
};
pub(crate) use markup::escape_text;
pub use markup::ALLOWED_TAGS;
pub use negotiate::{
    app_locale_nonblocking, resolve_known_setting, resolve_locale, resolve_request_locale,
    SYSTEM_SETTING,
};
pub use pseudo::{transform_accented, transform_bidi, PseudoLocale, PSEUDO_LOCALES};
pub use registry::{
    available_locales, display_locale_info, locale_info, source_locale, text_direction, LocaleInfo,
    TextDirection,
};
pub use text::{UiArg, UiText};
pub use unic_langid::LanguageIdentifier;

/// Catalog domains that are rendered by the backend only and never sent to the
/// dashboard.
pub const SERVER_ONLY_DOMAINS: &[&str] = &["telegram", "startup", "desktop"];

/// One embedded Fluent file: `locales/<locale>/<domain>.ftl`.
pub(crate) struct CatalogFile {
    pub locale: &'static str,
    pub domain: &'static str,
    pub source: &'static str,
}

/// Compile-time catalog key. Constructed only through the generated `ids`
/// module or `MessageId::new` in tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageId(&'static str);

impl MessageId {
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Serialize for MessageId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

/// Every message id in the source-locale catalogs, sorted.
#[cfg(test)]
pub(crate) fn source_message_ids() -> &'static [&'static str] {
    SOURCE_IDS
}

include!(concat!(env!("OUT_DIR"), "/i18n_catalog.rs"));
