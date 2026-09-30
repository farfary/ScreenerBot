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
fn token_route_errors_render_english() {
    let cases = [
        (
            ids::ERRORS_TOKENS_DATABASE_UNAVAILABLE,
            "Token database not available",
        ),
        (
            ids::ERRORS_TOKENS_BLACKLIST_FAILED,
            "Failed to blacklist token",
        ),
        (
            ids::ERRORS_TOKENS_BLACKLIST_INTERNAL,
            "Internal error during blacklist operation",
        ),
        (
            ids::ERRORS_TOKENS_UNBLACKLIST_FAILED,
            "Failed to remove from blacklist",
        ),
        (
            ids::ERRORS_TOKENS_UNBLACKLIST_INTERNAL,
            "Internal error during unblacklist operation",
        ),
        (
            ids::ERRORS_TOKENS_BLACKLIST_STATUS_FAILED,
            "Failed to check blacklist status",
        ),
        (
            ids::ERRORS_TOKENS_BLACKLIST_STATUS_INTERNAL,
            "Internal error during blacklist status check",
        ),
        (
            ids::ERRORS_TOKENS_FAVORITES_FETCH_FAILED,
            "Failed to fetch favorites",
        ),
        (
            ids::ERRORS_TOKENS_FAVORITE_ADD_FAILED,
            "Failed to add favorite",
        ),
        (
            ids::ERRORS_TOKENS_FAVORITE_REMOVE_FAILED,
            "Failed to remove favorite",
        ),
        (
            ids::ERRORS_TOKENS_FAVORITE_UPDATE_FAILED,
            "Failed to update favorite",
        ),
        (
            ids::ERRORS_TOKENS_DETAIL_NOT_FOUND,
            "Token not found in database or external sources",
        ),
        (ids::ERRORS_TOKENS_FETCH_FAILED, "Failed to fetch token"),
        (
            ids::ERRORS_TOKENS_REFRESH_ALL_FAILED,
            "All data sources failed",
        ),
        (ids::ERRORS_TOKENS_REFRESH_FAILED, "Failed to refresh token"),
        (
            ids::ERRORS_TOKENS_SEARCH_QUERY_REQUIRED,
            "Search query 'q' is required",
        ),
        (ids::ERRORS_TOKENS_SEARCH_FAILED, "Token search failed"),
    ];
    for (id, expected) in cases {
        assert_eq!(render(UiText::new(id)), expected);
    }
}

#[test]
fn action_and_service_lookup_errors_render_english() {
    assert_eq!(
        render(
            UiText::new(ids::ERRORS_ACTIONS_NOT_FOUND).arg("id", UiArg::Text("act-1".to_owned()))
        ),
        "Action act-1 not found"
    );
    assert_eq!(
        render(
            UiText::new(ids::ERRORS_SERVICES_NOT_FOUND).arg("name", UiArg::Text("rpc".to_owned()))
        ),
        "Service 'rpc' not found"
    );
}
