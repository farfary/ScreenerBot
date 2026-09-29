use crate::i18n::LocaleInfo;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct LocalesResponse {
    /// Locale the requesting client resolves to.
    pub resolved: String,
    /// Configured display language setting ("system" or a locale code).
    pub setting: String,
    /// Locale the catalogs are authored in.
    pub source: &'static str,
    pub locales: Vec<LocaleInfo>,
}
