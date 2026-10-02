// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Locale registry parsed from the embedded `locales/registry.toml`.

use super::pseudo::PseudoLocale;
use crate::logger::{self, LogTag};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use unic_langid::LanguageIdentifier;

/// Text direction of a locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDirection {
    Ltr,
    Rtl,
}

impl TextDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            TextDirection::Ltr => "ltr",
            TextDirection::Rtl => "rtl",
        }
    }
}

/// A shipped locale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocaleInfo {
    pub code: String,
    /// Native display name.
    pub name: String,
    pub dir: TextDirection,
}

#[derive(Deserialize)]
struct RegistryFile {
    source: String,
    locale: Vec<LocaleInfo>,
}

struct Registry {
    source: String,
    locales: Vec<LocaleInfo>,
    langids: Vec<LanguageIdentifier>,
}

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let file: RegistryFile = match toml::from_str(super::REGISTRY_TOML) {
            Ok(file) => file,
            Err(e) => {
                logger::error(LogTag::System, &format!("Invalid locale registry: {e}"));
                return Registry {
                    source: "en".to_string(),
                    locales: Vec::new(),
                    langids: Vec::new(),
                };
            }
        };
        let mut locales = Vec::new();
        let mut langids = Vec::new();
        for info in file.locale {
            match info.code.parse::<LanguageIdentifier>() {
                Ok(id) => {
                    langids.push(id);
                    locales.push(info);
                }
                Err(e) => logger::error(
                    LogTag::System,
                    &format!("Skipping locale {:?}: invalid language tag: {e}", info.code),
                ),
            }
        }
        Registry {
            source: file.source,
            locales,
            langids,
        }
    })
}

/// All shipped locales, in registry order.
pub fn available_locales() -> &'static [LocaleInfo] {
    &registry().locales
}

/// Registered locales as language identifiers, index-aligned with
/// `available_locales()`.
pub(crate) fn available_langids() -> &'static [LanguageIdentifier] {
    &registry().langids
}

/// Look up a registered locale by its exact code.
pub fn locale_info(code: &str) -> Option<&'static LocaleInfo> {
    registry().locales.iter().find(|l| l.code == code)
}

/// Locale metadata for rendering: registered locales and the pseudo-locales.
/// Use `locale_info` to offer locales; this is for locales already in effect.
pub fn display_locale_info(code: &str) -> Option<LocaleInfo> {
    locale_info(code)
        .cloned()
        .or_else(|| PseudoLocale::from_code(code).map(|p| p.info()))
}

/// Text direction of a resolved locale; left-to-right when the locale is unknown.
pub fn text_direction(locale: &LanguageIdentifier) -> TextDirection {
    display_locale_info(&locale.to_string()).map_or(TextDirection::Ltr, |info| info.dir)
}

/// Code of the locale that catalogs are authored in.
pub fn source_locale() -> &'static str {
    &registry().source
}
