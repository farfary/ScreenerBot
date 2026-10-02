// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Guards tying config field metadata to the `config-` localization messages.

use super::metadata::{
    catalog_key, category_key, collect_config_metadata, impact_key, ConfigCategory, ConfigImpact,
    ConfigMetadata, FieldMetadata, FieldType, SectionMetadata,
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

fn english() -> LanguageIdentifier {
    "en".parse().expect("valid language tag")
}

/// The `config-section-<id>` message of every top-level section.
fn section_keys(metadata: &ConfigMetadata) -> BTreeSet<String> {
    metadata
        .keys()
        .map(|section| format!("config-section-{}", catalog_key(&[section])))
        .collect()
}

fn category_keys() -> BTreeSet<String> {
    ConfigCategory::ALL.into_iter().map(category_key).collect()
}

fn impact_keys() -> BTreeSet<String> {
    ConfigImpact::ALL.into_iter().map(impact_key).collect()
}

/// Guarantee behind the dashboard's dynamic `config-` lookups (fields, sections,
/// categories, impacts): every key it can build exists, and the catalog holds nothing else in that namespace.
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
    let categories = category_keys();
    let impacts = impact_keys();
    let sections = section_keys(&metadata);
    assert_eq!(
        sections.len(),
        metadata.len(),
        "two sections share a `config-section-` key"
    );
    for key in categories
        .iter()
        .chain(impacts.iter())
        .chain(sections.iter())
    {
        assert!(
            expected.insert(key.clone()),
            "category, impact or section key `{key}` collides with a field key"
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
        "config messages that no field, section, category or impact uses: {orphans:?}"
    );

    for key in &field_keys {
        let hint = format_message(&english(), key, None).unwrap();
        for (name, _) in &hint.attributes {
            assert!(
                matches!(name.as_str(), "hint" | "unit" | "placeholder" | "subject"),
                "`{key}` has unsupported attribute `.{name}`"
            );
        }
    }
}

/// Every category and impact variant has a catalog message, whether or not a
/// field currently uses it.
#[test]
fn every_category_and_impact_variant_has_a_message() {
    for category in ConfigCategory::ALL {
        let key = category_key(category);
        assert!(
            format_message(&english(), &key, None).is_some(),
            "`{key}` is missing from locales/en/config.ftl"
        );
    }
    for impact in ConfigImpact::ALL {
        let key = impact_key(impact);
        assert!(
            format_message(&english(), &key, None).is_some(),
            "`{key}` is missing from locales/en/config.ftl"
        );
    }
}

/// Exhaustive on purpose: a new field type fails to compile until the dashboard's
/// array-entry message for it is named here. Only the types an array entry can
/// fail to parse as have one; the others fall back to the generic value message.
fn array_item_message(item_type: FieldType) -> Option<&'static str> {
    match item_type {
        FieldType::Integer => Some("system-config-array-invalid-integer"),
        FieldType::Number => Some("system-config-array-invalid-number"),
        FieldType::Boolean => Some("system-config-array-invalid-boolean"),
        FieldType::Array | FieldType::String | FieldType::Object => None,
    }
}

#[test]
fn array_item_messages_exist_in_the_catalog() {
    for item_type in [
        FieldType::Boolean,
        FieldType::Number,
        FieldType::Integer,
        FieldType::Array,
        FieldType::String,
        FieldType::Object,
    ] {
        if let Some(key) = array_item_message(item_type) {
            let code = serde_json::to_value(item_type).unwrap();
            assert_eq!(
                key.strip_prefix("system-config-array-invalid-"),
                code.as_str(),
                "key does not follow the serialized type {code}"
            );
            assert!(
                format_message(&english(), key, None).is_some(),
                "`{key}` is missing from locales/en/system.ftl"
            );
        }
    }
    assert!(
        format_message(&english(), "system-config-array-invalid-value", None).is_some(),
        "the generic array-entry message is missing from locales/en/system.ftl"
    );
}
