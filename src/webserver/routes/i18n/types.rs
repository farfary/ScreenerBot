// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Response types for the i18n routes - LocalesResponse, CatalogPayload and CatalogEntry.

use crate::i18n::{LocaleInfo, TextDirection};
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

/// Body of `/i18n/{locale}/catalog.js`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPayload<'a> {
    pub locale: &'a str,
    /// BCP 47 tag for `Intl` APIs, forcing Latin digits.
    pub intl_locale: String,
    pub dir: TextDirection,
    /// `"accented"` or `"bidi"` for a pseudo-locale; omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pseudo: Option<&'static str>,
    pub source: &'static str,
    /// Least specific first; later catalogs override earlier ones per key.
    pub catalogs: Vec<CatalogEntry>,
}

#[derive(Debug, Serialize)]
pub struct CatalogEntry {
    pub locale: String,
    pub ftl: String,
}
