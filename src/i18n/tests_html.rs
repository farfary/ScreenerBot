use super::html::rewrite_with;
use super::*;
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};

const FTL: &str = "\
markup = Use <b>bold</b> & more
tip = Tip
    .title = Hover text
    .aria-label = Screen reader text
    .onclick = alert(1)
attrs-only =
    .placeholder = Type here
count = { $n ->
    [one] One item
   *[other] { $n } items
}
greeting = Hello { $name }
";

fn render(html: &str) -> String {
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

fn langid(tag: &str) -> LanguageIdentifier {
    tag.parse().expect("valid language tag")
}

#[test]
fn value_replaces_text_as_escaped_text() {
    let out = render(r#"<p data-l10n-id="markup"><i>old</i></p>"#);
    assert!(
        out.contains("Use &lt;b&gt;bold&lt;/b&gt; &amp; more"),
        "{out}"
    );
    assert!(!out.contains("<i>"), "{out}");
}

#[test]
fn allowlisted_attributes_are_set_and_others_skipped() {
    let out = render(r#"<button data-l10n-id="tip">x</button>"#);
    assert!(out.contains(r#"title="Hover text""#), "{out}");
    assert!(out.contains(r#"aria-label="Screen reader text""#), "{out}");
    assert!(!out.contains("onclick"), "{out}");
    assert!(out.contains(">Tip</button>"), "{out}");
}

#[test]
fn args_json_is_interpolated() {
    let out = render(
        r#"<span data-l10n-id="greeting" data-l10n-args='{"name":"Ada"}'></span><span data-l10n-id="count" data-l10n-args='{"n":3}'></span><span data-l10n-id="count" data-l10n-args='{"n":1}'></span>"#,
    );
    assert!(out.contains(">Hello \u{2068}Ada\u{2069}<"), "{out}");
    assert!(out.contains(">\u{2068}3\u{2069} items<"), "{out}");
    assert!(out.contains(">One item<"), "{out}");
}

#[test]
fn unknown_id_leaves_element_untouched() {
    let html = r#"<p data-l10n-id="missing">keep <b>this</b></p>"#;
    assert_eq!(render(html), html);
}

#[test]
fn attribute_only_message_keeps_children() {
    let out = render(
        r#"<input data-l10n-id="attrs-only"><div data-l10n-id="attrs-only"><b>kept</b></div>"#,
    );
    assert!(out.contains(r#"placeholder="Type here""#), "{out}");
    assert!(out.contains("<b>kept</b>"), "{out}");
}

#[test]
fn localize_html_uses_real_catalog() {
    let out = localize_html(
        r#"<span data-l10n-id="common-loading">…</span><b data-l10n-id="common-language-system">?</b>"#,
        &langid("en"),
    );
    assert_eq!(out, "<span data-l10n-id=\"common-loading\">Loading…</span><b data-l10n-id=\"common-language-system\">System</b>");
}

#[test]
fn catalog_chain_runs_source_first() {
    let chain = dashboard_catalog_chain(&langid("en"));
    let codes: Vec<&str> = chain.iter().map(|(code, _)| code.as_str()).collect();
    assert_eq!(codes, ["en"]);
    assert!(chain[0].1.contains("common-loading"));
}

#[test]
fn dashboard_and_login_templates_survive_the_rewrite() {
    use crate::webserver::templates::{base_template, login_template};
    let locale = langid("en");
    let page = base_template("home", "<p data-l10n-id=\"common-loading\">x</p>", &locale);
    assert!(page.contains(">Loading…</p>"));
    assert!(page.contains("/i18n/en/catalog.js?v="));
    assert!(page.contains("/scripts/core/i18n.js?v="));
    let login = login_template("<b data-l10n-id=\"common-loading\">x</b>", &locale);
    assert!(login.contains(">Loading…</b>"));
    assert!(login.contains("/i18n/en/catalog.js?v="));
}
