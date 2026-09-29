//! Localization foundation: locale registry, negotiation, Fluent formatting and
//! the wire type for backend-authored display text.
//!
//! Catalogs live under `locales/` and are validated and embedded by `build.rs`.
//! The generated `ids` module exposes one `MessageId` constant per source-locale
//! message, so call sites reference catalog keys by symbol.

mod localizer;
mod negotiate;
mod registry;
mod text;

#[cfg(test)]
mod tests;

use serde::{Serialize, Serializer};
use std::fmt;

pub use localizer::{dashboard_catalog, format, format_en};
pub use negotiate::{resolve_locale, resolve_request_locale, SYSTEM_SETTING};
pub use registry::{
    available_locales, locale_info, source_locale, text_direction, LocaleInfo, TextDirection,
};
pub use text::{UiArg, UiText};
pub use unic_langid::LanguageIdentifier;

/// Catalog domains that are rendered by the backend only and never sent to the
/// dashboard.
pub const SERVER_ONLY_DOMAINS: &[&str] = &["telegram", "shell"];

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

include!(concat!(env!("OUT_DIR"), "/i18n_catalog.rs"));
