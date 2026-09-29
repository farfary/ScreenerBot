//! Guards tying config field metadata to the `config-` localization messages.

use super::metadata::{
    category_key, collect_config_metadata, impact_key, ConfigMetadata, FieldMetadata, FieldType,
    SectionMetadata,
};
use crate::i18n::{format_message, source_message_ids, LanguageIdentifier};
use std::collections::BTreeSet;

/// One field with the name it is declared under.
struct FieldEntry<'a> {
    name: &'a str,
    meta: &'a FieldMetadata,
}

fn walk_section<'a>(fields: &'a SectionMetadata, out: &mut Vec<FieldEntry<'a>>) {
    for (name, meta) in fields {
        out.push(FieldEntry { name, meta });
        if let Some(children) = &meta.children {
            walk_section(children, out);
        }
    }
}

/// The fields of one section, nested children included.
fn section_fields(fields: &SectionMetadata) -> Vec<FieldEntry<'_>> {
    let mut out = Vec::new();
    walk_section(fields, &mut out);
    out
}

/// Every field, nested children included, in section order.
fn all_fields(metadata: &ConfigMetadata) -> Vec<FieldEntry<'_>> {
    let mut out = Vec::new();
    for fields in metadata.values() {
        walk_section(fields, &mut out);
    }
    out
}

/// Category of a field as the dashboard groups it.
fn category_of(meta: &FieldMetadata) -> &str {
    meta.category.unwrap_or("General")
}

/// Collapse Fluent's indentation handling: trim every line and the whole text.
fn normalize(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Text an attribute carries, `None` when the schema leaves it empty.
fn non_empty(text: Option<&str>) -> Option<&str> {
    text.filter(|value| !value.trim().is_empty())
}

/// Placeholder shown for a field: its own, or a one-per-line prompt for an
/// array that declares none.
fn placeholder_of(meta: &FieldMetadata) -> Option<&str> {
    match meta.placeholder {
        Some(text) => non_empty(Some(text)),
        None => (meta.field_type == FieldType::Array).then_some("Enter one value per line"),
    }
}

fn english() -> LanguageIdentifier {
    "en".parse().expect("valid language tag")
}

fn used_category_keys(fields: &[FieldEntry<'_>]) -> BTreeSet<String> {
    fields
        .iter()
        .map(|field| category_key(category_of(field.meta)))
        .chain(std::iter::once(category_key("General")))
        .collect()
}

fn used_impact_keys(fields: &[FieldEntry<'_>]) -> BTreeSet<String> {
    fields
        .iter()
        .filter_map(|field| field.meta.impact.map(impact_key))
        .collect()
}

/// Guarantee behind the dashboard's dynamic `config-` lookups: every key it can
/// build exists, and the catalog holds nothing else in that namespace.
#[test]
fn config_catalog_covers_fields() {
    let metadata = collect_config_metadata();
    let fields = all_fields(&metadata);
    assert!(!fields.is_empty());

    let mut expected = BTreeSet::new();
    for field in &fields {
        assert!(
            field.meta.key.starts_with("config-") && field.meta.key.len() > "config-".len(),
            "field `{}` has no catalog key",
            field.name
        );
        assert!(
            expected.insert(field.meta.key.clone()),
            "catalog key `{}` is produced by more than one field path",
            field.meta.key
        );
    }
    let field_keys = expected.clone();
    let categories = used_category_keys(&fields);
    let impacts = used_impact_keys(&fields);
    for key in categories.iter().chain(impacts.iter()) {
        assert!(
            expected.insert(key.clone()),
            "category or impact key `{key}` collides with a field key"
        );
    }

    for key in &expected {
        let message = format_message(&english(), key, None)
            .unwrap_or_else(|| panic!("`{key}` is missing from locales/en/config.ftl"));
        assert!(
            message
                .value
                .as_deref()
                .is_some_and(|value| !value.is_empty()),
            "`{key}` has no value"
        );
    }

    let orphans: Vec<&str> = source_message_ids()
        .iter()
        .copied()
        .filter(|id| id.starts_with("config-") && !expected.contains(*id))
        .collect();
    assert!(
        orphans.is_empty(),
        "config messages that no field, category or impact uses: {orphans:?}"
    );

    for key in &field_keys {
        let hint = format_message(&english(), key, None).unwrap();
        for (name, _) in &hint.attributes {
            assert!(
                matches!(name.as_str(), "hint" | "unit" | "placeholder"),
                "`{key}` has unsupported attribute `.{name}`"
            );
        }
    }
}

/// The catalog reproduces the text the schema declares: the label (or the
/// field name for a field without one), the hint (or the doc comment), the
/// unit, and the placeholder (arrays default to a one-per-line prompt).
#[test]
fn config_catalog_matches_schema_text() {
    let metadata = collect_config_metadata();
    for field in all_fields(&metadata) {
        let message = format_message(&english(), &field.meta.key, None)
            .unwrap_or_else(|| panic!("`{}` is missing", field.meta.key));
        let attribute = |name: &str| {
            message
                .attributes
                .iter()
                .find(|(attr, _)| attr == name)
                .map(|(_, text)| text.as_str())
        };

        let label = non_empty(field.meta.label).unwrap_or(field.name);
        assert_eq!(
            normalize(message.value.as_deref().unwrap_or_default()),
            normalize(label),
            "label of `{}`",
            field.meta.key
        );

        let hint = non_empty(field.meta.hint).or(non_empty(field.meta.docs));
        assert_eq!(
            attribute("hint").map(normalize),
            hint.map(normalize),
            "hint of `{}`",
            field.meta.key
        );
        assert_eq!(
            attribute("unit").map(normalize),
            non_empty(field.meta.unit).map(normalize),
            "unit of `{}`",
            field.meta.key
        );

        assert_eq!(
            attribute("placeholder").map(normalize),
            placeholder_of(field.meta).map(normalize),
            "placeholder of `{}`",
            field.meta.key
        );
    }
}

/// Category and impact messages carry the English the dashboard showed before
/// the names moved into the catalog.
#[test]
fn config_category_and_impact_messages_read_as_their_names() {
    let metadata = collect_config_metadata();
    let fields = all_fields(&metadata);
    for field in &fields {
        let name = category_of(field.meta);
        let text = crate::i18n::format(&english(), &category_key(name), None);
        assert_eq!(text, name);
        if let Some(impact) = field.meta.impact {
            assert_eq!(
                crate::i18n::format(&english(), &impact_key(impact), None),
                impact
            );
        }
    }
}
