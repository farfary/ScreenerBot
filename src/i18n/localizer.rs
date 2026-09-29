//! Fluent bundles built from the embedded catalogs, with per-key fallback.

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

fn build_bundle(code: &str) -> Option<Bundle> {
    let langid: LanguageIdentifier = code.parse().ok()?;
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    for file in locale_files(code) {
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

/// Bundle for a registered locale, built on first use.
fn bundle_for(code: &str) -> Option<&'static Bundle> {
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

/// Format a message for `locale`. Falls back per key to the language-only
/// locale and then the source locale; returns the id when no catalog has it.
pub fn format(locale: &LanguageIdentifier, id: &str, args: Option<&FluentArgs>) -> String {
    for code in fallback_chain(locale) {
        let Some(bundle) = bundle_for(&code) else {
            continue;
        };
        let Some(pattern) = bundle.get_message(id).and_then(|m| m.value()) else {
            continue;
        };
        let mut errors = Vec::new();
        let text = bundle.format_pattern(pattern, args, &mut errors);
        if !errors.is_empty() {
            logger::debug(
                LogTag::System,
                &format!("Formatting {id:?} for {code}: {errors:?}"),
            );
        }
        return text.into_owned();
    }
    id.to_string()
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
