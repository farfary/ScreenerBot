//! Locale catalog validation and code generation.
//!
//! Every registered locale directory under `locales/` is parsed with the Fluent
//! syntax parser; any parse error or duplicate message id fails the build. The
//! generated `i18n_catalog.rs` embeds the catalogs and one `MessageId` constant
//! per source-locale message, plus the sorted list of those ids.

use fluent_syntax::ast::Entry;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Registry {
    source: String,
    locale: Vec<RegisteredLocale>,
}

#[derive(Deserialize)]
struct RegisteredLocale {
    code: String,
    name: String,
    dir: String,
}

/// Validate all catalogs and write `$OUT_DIR/i18n_catalog.rs`.
pub fn generate(manifest_dir: &Path, out_dir: &Path) {
    let locales_dir = manifest_dir.join("locales");
    let registry_path = locales_dir.join("registry.toml");
    let registry_text = std::fs::read_to_string(&registry_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", registry_path.display()));
    let registry: Registry = toml::from_str(&registry_text)
        .unwrap_or_else(|e| panic!("invalid {}: {e}", registry_path.display()));

    if registry.source != "en" {
        panic!(
            "registry source locale must be \"en\", found {:?}",
            registry.source
        );
    }
    if !registry.locale.iter().any(|l| l.code == "en") {
        panic!(
            "locale \"en\" must be registered in {}",
            registry_path.display()
        );
    }

    for locale in &registry.locale {
        if locale.name.trim().is_empty() {
            panic!("locale {:?} has an empty name", locale.code);
        }
        if locale.dir != "ltr" && locale.dir != "rtl" {
            panic!(
                "locale {:?} has dir {:?}; expected \"ltr\" or \"rtl\"",
                locale.code, locale.dir
            );
        }
    }

    let mut files: Vec<(String, String, PathBuf)> = Vec::new();
    let mut source_ids: BTreeSet<String> = BTreeSet::new();

    for locale in &registry.locale {
        let dir = locales_dir.join(&locale.code);
        if !dir.is_dir() {
            panic!(
                "registered locale {:?} has no directory {}",
                locale.code,
                dir.display()
            );
        }
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "ftl"))
            .collect();
        paths.sort();

        let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
        for path in paths {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            let resource = match fluent_syntax::parser::parse(text.as_str()) {
                Ok(r) => r,
                Err((_, errors)) => {
                    panic!("{}: Fluent parse errors: {:?}", path.display(), errors)
                }
            };
            for entry in &resource.body {
                if let Entry::Message(message) = entry {
                    let id = message.id.name.to_string();
                    if let Some(first) = seen.insert(id.clone(), path.clone()) {
                        panic!(
                            "{}: duplicate message id {:?} (first defined in {})",
                            path.display(),
                            id,
                            first.display()
                        );
                    }
                    if locale.code == registry.source {
                        source_ids.insert(id);
                    }
                }
            }
            let domain = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            files.push((locale.code.clone(), domain, path));
        }
    }
    files.sort();

    let mut consts: BTreeMap<String, String> = BTreeMap::new();
    for id in &source_ids {
        if id.is_empty()
            || !id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            panic!("message id {id:?} must be lowercase kebab-case [a-z0-9-]");
        }
        let name = id.replace('-', "_").to_ascii_uppercase();
        if let Some(other) = consts.insert(name.clone(), id.clone()) {
            panic!("message ids {other:?} and {id:?} both map to constant {name}");
        }
    }

    let mut out = String::new();
    out.push_str("pub(crate) static CATALOGS: &[CatalogFile] = &[\n");
    for (locale, domain, path) in &files {
        let _ = writeln!(
            out,
            "    CatalogFile {{ locale: {locale:?}, domain: {domain:?}, source: include_str!({:?}) }},",
            path.display().to_string()
        );
    }
    out.push_str("];\n\n");
    let _ = writeln!(
        out,
        "pub(crate) const REGISTRY_TOML: &str = include_str!({:?});\n",
        registry_path.display().to_string()
    );
    out.push_str("pub(crate) static SOURCE_IDS: &[&str] = &[\n");
    for id in &source_ids {
        let _ = writeln!(out, "    {id:?},");
    }
    out.push_str("];\n\n");
    out.push_str("pub mod ids {\n    use super::MessageId;\n\n");
    for (name, id) in &consts {
        let _ = writeln!(
            out,
            "    pub const {name}: MessageId = MessageId::new({id:?});"
        );
    }
    out.push_str("}\n");

    let target = out_dir.join("i18n_catalog.rs");
    std::fs::write(&target, out)
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", target.display()));
}
