//! Catalog text carried by success responses: every id renders its English wording.

use crate::i18n::{ids, LanguageIdentifier, UiArg, UiText};

fn render(text: UiText) -> String {
    let en: LanguageIdentifier = "en".parse().unwrap();
    text.render_plain(&en)
}

#[test]
fn system_results_render_english() {
    assert_eq!(
        render(UiText::new(ids::SYSTEM_RESULT_CONFIG_DIFFERS)),
        "In-memory configuration differs from disk version"
    );
    assert_eq!(
        render(UiText::new(ids::SYSTEM_RESULT_CONFIG_MATCHES)),
        "In-memory configuration matches disk version"
    );
    let imported = |count: i64| {
        UiText::new(ids::SYSTEM_RESULT_CONFIG_IMPORTED).arg("count", UiArg::Count(count))
    };
    assert_eq!(render(imported(1)), "Successfully imported 1 section");
    assert_eq!(render(imported(3)), "Successfully imported 3 sections");
    let warned = UiText::new(ids::SYSTEM_RESULT_CONFIG_IMPORTED_WITH_WARNINGS)
        .arg("count", UiArg::Count(2))
        .arg("warnings", UiArg::Count(1))
        .arg(
            "details",
            UiArg::Text("Failed to save to disk: denied".to_owned()),
        );
    assert_eq!(
        render(warned),
        "Imported 2 sections with 1 warning: Failed to save to disk: denied"
    );
}

#[test]
fn token_source_results_render_english() {
    let label = |id, name: &str| UiText::new(id).arg("label", UiArg::Text(name.to_owned()));
    assert_eq!(
        render(UiText::new(ids::TOKENS_RESULT_SOURCE_LIVE)),
        "Live market data"
    );
    assert_eq!(
        render(label(ids::TOKENS_RESULT_SOURCE_UNAVAILABLE, "DexScreener")),
        "DexScreener unavailable — retrying"
    );
    assert_eq!(
        render(label(ids::TOKENS_RESULT_SOURCE_NOT_LISTED, "GeckoTerminal")),
        "Not listed on GeckoTerminal"
    );
    assert_eq!(
        render(UiText::new(ids::TOKENS_RESULT_SECURITY_AVAILABLE)),
        "Security report available"
    );
    assert_eq!(
        render(UiText::new(ids::TOKENS_RESULT_SECURITY_MISSING)),
        "No Rugcheck report"
    );
    assert_eq!(
        render(UiText::new(ids::TOKENS_RESULT_CHART_AVAILABLE)),
        "Chart data available"
    );
    assert_eq!(
        render(UiText::new(ids::TOKENS_RESULT_CHART_MISSING)),
        "No chart data yet"
    );
}

#[test]
fn position_management_result_renders_english() {
    let text = UiText::new(ids::POSITIONS_RESULT_MANAGEMENT_SET)
        .arg("management", UiArg::Text("user_only".to_owned()));
    assert_eq!(render(text), "Position management set to user_only");
}
