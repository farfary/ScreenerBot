//! Billboard route handlers — endpoint implementations.

use super::cache;
use super::cards::build_cards;
use super::types::*;
use crate::tokens;
use crate::webserver::utils::success_response;
use axum::response::{IntoResponse, Response};

/// Ensure billboard tokens are tracked in our database (for future updates).
/// This runs in the background and doesn't block the response.
fn ensure_tokens_tracked(tokens: &[BillboardToken]) {
    let mints: Vec<String> = tokens.iter().map(|t| t.mint.clone()).collect();
    let names: Vec<Option<String>> = tokens.iter().map(|t| Some(t.name.clone())).collect();
    let symbols: Vec<Option<String>> = tokens.iter().map(|t| Some(t.symbol.clone())).collect();

    tokio::spawn(async move {
        let db = match tokens::get_global_database() {
            Some(db) => db,
            None => return,
        };

        for (i, mint) in mints.iter().enumerate() {
            let name = names.get(i).and_then(|n| n.as_deref());
            let symbol = symbols.get(i).and_then(|s| s.as_deref());

            // upsert_token will create tracking entry if token doesn't exist
            let _ = db.upsert_token(mint, symbol, name, None);
        }
    });
}

/// GET /api/billboard — Get featured tokens
pub(super) async fn get_billboard_handler() -> Response {
    match cache::get_billboard().await {
        Ok(tokens) => {
            // Ensure tokens are tracked for future updates
            ensure_tokens_tracked(&tokens);

            let cards = build_cards(tokens).await;
            let count = cards.len();

            success_response(serde_json::json!({
                "tokens": cards,
                "count": count
            }))
        }
        Err(e) => axum::Json(serde_json::json!({
            "success": false,
            "error": e,
            "tokens": [],
            "count": 0
        }))
        .into_response(),
    }
}

/// GET /api/billboard/all — Get all billboard categories
pub(super) async fn get_billboard_all_handler() -> Response {
    // Fetch all sources concurrently
    let (featured_result, jupiter_organic, jupiter_traded, dexscreener_trending) = tokio::join!(
        cache::get_billboard(),
        cache::get_jupiter_organic(),
        cache::get_jupiter_traded(),
        cache::get_dexscreener_trending()
    );

    let featured = featured_result.unwrap_or_default();

    // Ensure featured tokens are tracked for future updates
    ensure_tokens_tracked(&featured);

    // Every category is enriched the same way — the frontend renders one card shape.
    let (featured, jupiter_organic, jupiter_traded, dexscreener_trending) = tokio::join!(
        build_cards(featured),
        build_cards(jupiter_organic),
        build_cards(jupiter_traded),
        build_cards(dexscreener_trending),
    );

    success_response(serde_json::json!({
        "success": true,
        "featured": featured,
        "jupiter_organic": jupiter_organic,
        "jupiter_traded": jupiter_traded,
        "dexscreener_trending": dexscreener_trending
    }))
}

/// GET /api/billboard/jupiter/organic — Get Jupiter top organic tokens
pub(super) async fn get_jupiter_organic_handler() -> Response {
    let cards = build_cards(cache::get_jupiter_organic().await).await;
    success_response(serde_json::json!({
        "success": true,
        "tokens": cards,
        "count": cards.len()
    }))
}

/// GET /api/billboard/jupiter/traded — Get Jupiter top traded tokens
pub(super) async fn get_jupiter_traded_handler() -> Response {
    let cards = build_cards(cache::get_jupiter_traded().await).await;
    success_response(serde_json::json!({
        "success": true,
        "tokens": cards,
        "count": cards.len()
    }))
}

/// GET /api/billboard/dexscreener/trending — Get DexScreener trending tokens
pub(super) async fn get_dexscreener_trending_handler() -> Response {
    let cards = build_cards(cache::get_dexscreener_trending().await).await;
    success_response(serde_json::json!({
        "success": true,
        "tokens": cards,
        "count": cards.len()
    }))
}
