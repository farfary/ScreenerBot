use super::*;
use axum::http::{header::ACCEPT_LANGUAGE, HeaderMap, HeaderValue};
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};

fn langid(tag: &str) -> LanguageIdentifier {
    tag.parse().expect("valid language tag")
}

#[test]
fn registry_contains_source_locale() {
    assert_eq!(source_locale(), "en");
    let info = locale_info("en").expect("en is registered");
    assert_eq!(info.name, "English");
    assert_eq!(info.dir, TextDirection::Ltr);
    assert_eq!(serde_json::to_string(&info.dir).unwrap(), "\"ltr\"");
    assert_eq!(
        serde_json::to_string(&TextDirection::Rtl).unwrap(),
        "\"rtl\""
    );
    assert!(available_locales().iter().any(|l| l.code == "en"));
    assert!(locale_info("xx").is_none());
}

#[test]
fn every_catalog_file_builds_into_a_bundle() {
    assert!(!CATALOGS.is_empty());
    for file in CATALOGS {
        let resource = FluentResource::try_new(file.source.to_string())
            .unwrap_or_else(|(_, e)| panic!("{}/{}: {e:?}", file.locale, file.domain));
        let mut bundle = FluentBundle::new_concurrent(vec![langid(file.locale)]);
        bundle
            .add_resource(resource)
            .unwrap_or_else(|e| panic!("{}/{}: {e:?}", file.locale, file.domain));
    }
}

#[test]
fn explicit_setting_wins() {
    assert_eq!(resolve_locale("en", None), langid("en"));
    assert_eq!(resolve_locale("en-US", Some("fr")), langid("en"));
}

#[test]
fn system_setting_negotiates_request_language() {
    let unregistered = resolve_locale("system", Some("sw-KE,sw;q=0.9,en;q=0.8"));
    assert_eq!(unregistered, langid("en"));
    let regional = resolve_locale("system", Some("fr-CA,fr;q=0.9,en;q=0.8"));
    let expected = if locale_info("fr").is_some() { "fr" } else { "en" };
    assert_eq!(regional, langid(expected));
}

#[test]
fn unknown_setting_falls_through() {
    assert_eq!(resolve_locale("xx", Some("en-GB")), langid("en"));
    let resolved = resolve_locale("not a tag!", None);
    assert!(locale_info(&resolved.to_string()).is_some());
}

#[test]
fn empty_inputs_resolve_to_a_registered_locale() {
    let resolved = resolve_locale("system", None);
    assert!(locale_info(&resolved.to_string()).is_some());
}

#[test]
fn request_locale_reads_accept_language() {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-GB,en;q=0.8"));
    assert_eq!(resolve_request_locale(&headers), langid("en"));
}

#[test]
fn format_falls_back_to_source_then_id() {
    assert_eq!(format(&langid("pt-BR"), "common-loading", None), "Loading…");
    assert_eq!(format_en("common-language-system", None), "System");
    assert_eq!(format(&langid("en"), "no-such-key", None), "no-such-key");
}

#[test]
fn ui_text_json_shape_and_round_trip() {
    let text = UiText::new(ids::COMMON_LOADING)
        .arg("count", UiArg::Count(3))
        .arg(
            "inner",
            UiArg::Nested(Box::new(UiText::new(ids::COMMON_LANGUAGE_SYSTEM))),
        );
    let json = serde_json::to_string(&text).unwrap();
    assert_eq!(
        json,
        r#"{"id":"common-loading","args":{"count":{"type":"count","value":3},"inner":{"type":"nested","value":{"id":"common-language-system"}}}}"#
    );
    let back: UiText = serde_json::from_str(&json).unwrap();
    assert_eq!(back, text);
    assert_eq!(
        serde_json::to_string(&UiText::from_stored("common-loading".to_string())).unwrap(),
        r#"{"id":"common-loading"}"#
    );
}

#[test]
fn ui_text_renders_nested_and_args() {
    let text = UiText::new(ids::COMMON_LOADING)
        .arg("n", UiArg::Number(1.5))
        .arg("d", UiArg::Duration(1000));
    assert_eq!(text.render(&langid("en")), "Loading…");
    let nested = UiText::new(MessageId::new("common-language-system"));
    assert_eq!(nested.render(&langid("en")), "System");
    let mut args = FluentArgs::new();
    args.set("x", 1);
    assert_eq!(format_en("common-loading", Some(&args)), "Loading…");
}

#[test]
fn message_ids_are_plain_strings() {
    assert_eq!(ids::COMMON_LOADING.as_str(), "common-loading");
    assert_eq!(ids::COMMON_LOADING.to_string(), "common-loading");
    assert_eq!(
        serde_json::to_string(&ids::COMMON_LOADING).unwrap(),
        "\"common-loading\""
    );
}

#[test]
fn dashboard_catalog_excludes_server_only_domains() {
    let catalog = dashboard_catalog("en").expect("en is registered");
    assert!(catalog.starts_with("# Terms are never translated"));
    assert!(catalog.contains("common-loading"));
    assert!(dashboard_catalog("xx").is_none());
}

#[test]
fn render_plain_strips_isolation_marks() {
    let text =
        UiText::new(ids::ERRORS_STRATEGIES_ALREADY_EXISTS).arg("id", UiArg::Text("a".into()));
    let rendered = text.render(&langid("en"));
    assert!(rendered.contains('\u{2068}'));
    let plain = text.render_plain(&langid("en"));
    assert_eq!(plain, "Strategy with ID 'a' already exists");
    assert!(!plain.contains('\u{2068}') && !plain.contains('\u{2069}'));
}
