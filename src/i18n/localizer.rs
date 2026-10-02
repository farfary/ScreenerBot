// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Fluent bundles built from the embedded catalogs, with per-key fallback.

use super::pseudo::PseudoLocale;
use super::{registry, CatalogFile, CATALOGS, SERVER_ONLY_DOMAINS};
use crate::logger::{self, LogTag};
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use std::collections::HashMap;
use std::sync::OnceLock;
use unic_langid::LanguageIdentifier;

type Bundle = FluentBundle<FluentResource>;

const TERMS_DOMAIN: &str = "terms";

fn locale_files(locale: &str) -> impl Iterator<Item = &'static CatalogFile> + '_ {
    CATALOGS.iter().filter(move |file| file.locale == locale)
}

/// Build a bundle for `code`. A pseudo-locale uses the source catalogs with its
/// text transform, which Fluent applies to text elements only.
fn build_bundle(code: &str) -> Option<Bundle> {
    let langid: LanguageIdentifier = code.parse().ok()?;
    let pseudo = PseudoLocale::from_code(code);
    let catalog_locale = if pseudo.is_some() {
        registry::source_locale()
    } else {
        code
    };
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    if let Some(pseudo) = pseudo {
        bundle.set_transform(Some(pseudo.transform()));
    }
    for file in locale_files(catalog_locale) {
        let resource = match FluentResource::try_new(file.source.to_string()) {
            Ok(resource) => resource,
            Err((resource, errors)) => {
                logger::error(
                    LogTag::System,
                    &format!(
                        "Catalog {}/{}.ftl has parse errors: {errors:?}",
                        code, file.domain
                    ),
                );
                resource
            }
        };
        if let Err(errors) = bundle.add_resource(resource) {
            logger::error(
                LogTag::System,
                &format!(
                    "Catalog {}/{}.ftl has overriding entries: {errors:?}",
                    code, file.domain
                ),
            );
        }
    }
    Some(bundle)
}

/// Bundle for a registered or pseudo locale, built on first use.
fn bundle_for(code: &str) -> Option<&'static Bundle> {
    static PSEUDO_BUNDLES: OnceLock<HashMap<&'static str, OnceLock<Option<Bundle>>>> =
        OnceLock::new();
    if let Some(pseudo) = PseudoLocale::from_code(code) {
        return PSEUDO_BUNDLES
            .get_or_init(|| {
                super::PSEUDO_LOCALES
                    .iter()
                    .map(|p| (p.code(), OnceLock::new()))
                    .collect()
            })
            .get(pseudo.code())?
            .get_or_init(|| build_bundle(code))
            .as_ref();
    }
    static BUNDLES: OnceLock<HashMap<&'static str, OnceLock<Option<Bundle>>>> = OnceLock::new();
    let bundles = BUNDLES.get_or_init(|| {
        registry::available_locales()
            .iter()
            .map(|info| (info.code.as_str(), OnceLock::new()))
            .collect()
    });
    bundles
        .get(code)?
        .get_or_init(|| build_bundle(code))
        .as_ref()
}

/// Lookup order for one key: the locale, its language-only form, then the source locale.
fn fallback_chain(locale: &LanguageIdentifier) -> Vec<String> {
    // A pseudo bundle already holds every source message, transformed; the
    // language-only form must not resolve to the untransformed source.
    if PseudoLocale::from_code(&locale.to_string()).is_some() {
        return vec![locale.to_string(), registry::source_locale().to_string()];
    }
    let mut chain = vec![locale.to_string()];
    let language_only =
        LanguageIdentifier::from_parts(locale.language, None, None, &[]).to_string();
    chain.push(language_only);
    chain.push(registry::source_locale().to_string());
    let mut seen = Vec::with_capacity(chain.len());
    chain.retain(|code| {
        let fresh = !seen.contains(code);
        if fresh {
            seen.push(code.clone());
        }
        fresh
    });
    chain
}

/// A formatted message: its value plus its attributes in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedMessage {
    pub value: Option<String>,
    pub attributes: Vec<(String, String)>,
}

/// Format a message and its attributes for `locale`. Falls back per key to the
/// language-only locale and then the source locale; `None` when no catalog has it.
pub fn format_message(
    locale: &LanguageIdentifier,
    id: &str,
    args: Option<&FluentArgs>,
) -> Option<LocalizedMessage> {
    for code in fallback_chain(locale) {
        let Some(bundle) = bundle_for(&code) else {
            continue;
        };
        let Some(message) = bundle.get_message(id) else {
            continue;
        };
        let mut errors = Vec::new();
        let value = message.value().map(|pattern| {
            bundle
                .format_pattern(pattern, args, &mut errors)
                .into_owned()
        });
        let attributes = message
            .attributes()
            .map(|attribute| {
                let text = bundle.format_pattern(attribute.value(), args, &mut errors);
                (attribute.id().to_string(), text.into_owned())
            })
            .collect();
        if !errors.is_empty() {
            logger::debug(
                LogTag::System,
                &format!("Formatting {id:?} for {code}: {errors:?}"),
            );
        }
        return Some(LocalizedMessage { value, attributes });
    }
    None
}

/// Format a message value for `locale`; returns the id when no catalog has a
/// value for it.
pub fn format(locale: &LanguageIdentifier, id: &str, args: Option<&FluentArgs>) -> String {
    format_message(locale, id, args)
        .and_then(|message| message.value)
        .unwrap_or_else(|| id.to_string())
}

/// Locales with dashboard catalogs in the fallback chain of `locale`, least
/// specific first (source locale, language-only form, then the locale itself).
/// A pseudo-locale contributes no catalog of its own, so it resolves to the source only.
fn catalog_order(locale: &LanguageIdentifier) -> Vec<String> {
    let mut chain = fallback_chain(locale);
    chain.retain(|code| registry::locale_info(code).is_some());
    chain.reverse();
    chain
}

/// Format a message in the source locale.
pub fn format_en(id: &str, args: Option<&FluentArgs>) -> String {
    let source: LanguageIdentifier = registry::source_locale()
        .parse()
        .unwrap_or_else(|_| LanguageIdentifier::default());
    format(&source, id, args)
}

/// Concatenated Fluent source served to the dashboard: the terms file first,
/// then every domain except the server-only ones. `None` for an unregistered locale.
pub fn dashboard_catalog(code: &str) -> Option<String> {
    registry::locale_info(code)?;
    let mut out = String::new();
    let terms = locale_files(code).filter(|f| f.domain == TERMS_DOMAIN);
    let rest = locale_files(code)
        .filter(|f| f.domain != TERMS_DOMAIN && !SERVER_ONLY_DOMAINS.contains(&f.domain));
    for file in terms.chain(rest) {
        out.push_str(file.source);
        out.push('\n');
    }
    Some(out)
}

/// Dashboard catalogs for `locale` in fallback order, least specific first, as
/// `(locale code, Fluent source)` pairs. Later entries override earlier ones.
pub fn dashboard_catalog_chain(locale: &LanguageIdentifier) -> Vec<(String, String)> {
    catalog_order(locale)
        .into_iter()
        .filter_map(|code| dashboard_catalog(&code).map(|ftl| (code, ftl)))
        .collect()
}
