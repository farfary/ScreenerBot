// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests the Fluent markup pipeline - fixture-driven sanitize and render cases, argument escaping, and the data-l10n-markup attribute.

use super::html::rewrite_with;
use super::markup::{escape_args, escape_text, sanitize};
use super::pseudo::{transform_accented, transform_bidi};
use super::*;
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    sanitize: Vec<SanitizeCase>,
    render: Vec<RenderCase>,
}

#[derive(Deserialize)]
struct SanitizeCase {
    name: String,
    input: String,
    expected: String,
}

#[derive(Deserialize)]
struct RenderCase {
    name: String,
    ftl: String,
    args: serde_json::Map<String, serde_json::Value>,
    expected: String,
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("../../tools/tests/fixtures/i18n_markup.json"))
        .expect("markup fixture")
}

fn langid(tag: &str) -> LanguageIdentifier {
    tag.parse().expect("valid language tag")
}

fn fluent_args(map: &serde_json::Map<String, serde_json::Value>) -> FluentArgs<'static> {
    let mut args = FluentArgs::new();
    for (name, value) in map {
        match value {
            serde_json::Value::String(s) => args.set(name.clone(), FluentValue::from(s.clone())),
            serde_json::Value::Number(n) => args.set(
                name.clone(),
                FluentValue::from(n.as_i64().expect("integer arg")),
            ),
            other => panic!("unsupported fixture arg {other}"),
        }
    }
    args
}

/// Escape arguments, format the pattern, sanitize: the markup pipeline.
fn render_markup(ftl: &str, args: &FluentArgs) -> String {
    let resource = FluentResource::try_new(format!("{ftl}\n")).expect("valid fixture ftl");
    let mut bundle = FluentBundle::new_concurrent(vec![langid("en")]);
    bundle.add_resource(resource).expect("no overrides");
    let pattern = bundle
        .get_message("m")
        .and_then(|m| m.value())
        .expect("message m");
    let mut errors = Vec::new();
    let formatted = bundle.format_pattern(pattern, Some(&escape_args(args)), &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    sanitize(&formatted)
}

#[test]
fn sanitize_fixture_cases() {
    for case in fixture().sanitize {
        assert_eq!(sanitize(&case.input), case.expected, "{}", case.name);
    }
}

#[test]
fn render_fixture_cases() {
    for case in fixture().render {
        let args = fluent_args(&case.args);
        assert_eq!(
            render_markup(&case.ftl, &args),
            case.expected,
            "{}",
            case.name
        );
    }
}

#[test]
fn sanitize_is_idempotent() {
    for case in fixture().sanitize {
        let once = sanitize(&case.input);
        assert_eq!(sanitize(&once), once, "{}", case.name);
    }
}

#[test]
fn escape_text_covers_the_five_characters() {
    assert_eq!(escape_text(r#"&<>"'"#), "&amp;&lt;&gt;&quot;&#39;");
}

#[test]
fn escape_args_escapes_strings_and_keeps_numbers() {
    let mut args = FluentArgs::new();
    args.set("s", FluentValue::from("<b>"));
    args.set("n", FluentValue::from(7_i64));
    let escaped = escape_args(&args);
    assert_eq!(
        escaped.get("s"),
        Some(&FluentValue::from("&lt;b&gt;".to_string()))
    );
    assert_eq!(escaped.get("n"), Some(&FluentValue::from(7_i64)));
}

#[test]
fn pseudo_transforms_leave_tag_names_untouched() {
    let input = "Save <strong>all</strong><br/> a < b";
    assert_eq!(
        transform_accented(input),
        "Şȧȧṽḗḗ <strong>ȧȧŀŀ</strong><br/> ȧȧ < ƀ"
    );
    assert!(transform_bidi(input).contains("<strong>"));
    assert!(transform_bidi(input).contains("</strong><br/>"));
}

const FTL: &str = "\
range = Showing <strong>{ $from }</strong> of <strong>{ $total }</strong>
tip = Tip
    .title = Cost { $name }
";

fn html_lookup(html: &str) -> String {
    let resource = FluentResource::try_new(FTL.to_string()).expect("valid test catalog");
    let mut bundle = FluentBundle::new_concurrent(vec![langid("en")]);
    bundle.add_resource(resource).expect("no overrides");
    rewrite_with(html, &|id, args: Option<&FluentArgs>| {
        let message = bundle.get_message(id)?;
        let mut errors = Vec::new();
        Some(LocalizedMessage {
            value: message
                .value()
                .map(|p| bundle.format_pattern(p, args, &mut errors).into_owned()),
            attributes: message
                .attributes()
                .map(|a| {
                    (
                        a.id().to_string(),
                        bundle
                            .format_pattern(a.value(), args, &mut errors)
                            .into_owned(),
                    )
                })
                .collect(),
        })
    })
}

#[test]
fn markup_attribute_sets_sanitized_html() {
    let out = html_lookup(
        r#"<span data-l10n-id="range" data-l10n-markup data-l10n-args='{"from":"1","total":"<script>x</script>"}'>old</span>"#,
    );
    assert!(
        out.contains("Showing <strong>\u{2068}1\u{2069}</strong> of <strong>\u{2068}&lt;script&gt;x&lt;/script&gt;\u{2069}</strong>"),
        "{out}"
    );
    let body = out.split_once("'>").map_or("", |(_, body)| body);
    assert!(!body.contains("<script>"), "{out}");
}

#[test]
fn without_markup_attribute_the_value_stays_text() {
    let out = html_lookup(
        r#"<span data-l10n-id="range" data-l10n-args='{"from":"1","total":"2"}'>old</span>"#,
    );
    assert!(out.contains("Showing &lt;strong&gt;"), "{out}");
    assert!(!out.contains("<strong>"), "{out}");
}

#[test]
fn markup_element_attributes_use_unescaped_args() {
    let out = html_lookup(
        r#"<button data-l10n-id="tip" data-l10n-markup data-l10n-args='{"name":"a & b"}'>x</button>"#,
    );
    assert!(
        out.contains("title=\"Cost \u{2068}a & b\u{2069}\""),
        "{out}"
    );
}
