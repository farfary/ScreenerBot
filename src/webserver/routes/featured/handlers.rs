//! Featured route handlers — endpoint implementations.

use super::cache;
use super::cards::build_cards;
use super::types::FeaturedCard;
use crate::tokens;
use crate::webserver::routes::boosts;
use crate::webserver::utils::success_response;
use axum::response::Response;

/// Start tracking the boosted tokens in our database so they gain market data.
///
/// A boost is bought for a mint the app may never have seen; without this the card
/// would stay identity-only forever and the token would never appear in the tokens
/// table the user can act on. Runs in the background and never blocks the response.
///
/// Only OUR boosted tokens are tracked. The discovery boards are third-party
/// rankings that change every couple of minutes — tracking those would grow the
/// database by hundreds of tokens the user never asked about.
fn ensure_boosted_tokens_tracked(cards: &[FeaturedCard]) {
    let tracked: Vec<(String, Option<String>, Option<String>)> = cards
        .iter()
        .map(|card| {
            (
                card.mint.clone(),
                Some(card.symbol.clone()).filter(|s| !s.is_empty()),
                Some(card.name.clone()).filter(|s| !s.is_empty()),
            )
        })
        .collect();

    if tracked.is_empty() {
        return;
    }

    tokio::spawn(async move {
        let Some(db) = tokens::get_global_database() else {
            return;
        };
        for (mint, symbol, name) in &tracked {
            // upsert_token creates the tracking entry when the token is unknown.
            let _ = db.upsert_token(mint, symbol.as_deref(), name.as_deref(), None);
        }
    });
}

/// Our boosted tokens, ranked and enriched.
async fn boosted_cards() -> Vec<FeaturedCard> {
    let cards = build_cards(boosts::active_boosts().await).await;
    ensure_boosted_tokens_tracked(&cards);
    cards
}

/// GET /api/featured — the tokens boosted on screenerbot.io
pub(super) async fn get_featured_handler() -> Response {
    let cards = boosted_cards().await;
    success_response(serde_json::json!({
        "success": true,
        "count": cards.len(),
        "tokens": cards,
    }))
}

/// GET /api/featured/all — every featured category in one pass
pub(super) async fn get_featured_all_handler() -> Response {
    // Fetch all sources concurrently
    let (boosted, jupiter_organic, jupiter_traded, dexscreener_trending) = tokio::join!(
        boosted_cards(),
        cache::get_jupiter_organic(),
        cache::get_jupiter_traded(),
        cache::get_dexscreener_trending()
    );

    // Every category is enriched the same way — the frontend renders one card shape.
    let (jupiter_organic, jupiter_traded, dexscreener_trending) = tokio::join!(
        build_cards(jupiter_organic),
        build_cards(jupiter_traded),
        build_cards(dexscreener_trending),
    );

    success_response(serde_json::json!({
        "success": true,
        "boosted": boosted,
        "jupiter_organic": jupiter_organic,
        "jupiter_traded": jupiter_traded,
        "dexscreener_trending": dexscreener_trending
    }))
}

/// GET /api/featured/jupiter/organic — Jupiter top organic tokens
pub(super) async fn get_jupiter_organic_handler() -> Response {
    let cards = build_cards(cache::get_jupiter_organic().await).await;
    success_response(serde_json::json!({
        "success": true,
        "count": cards.len(),
        "tokens": cards,
    }))
}

/// GET /api/featured/jupiter/traded — Jupiter top traded tokens
pub(super) async fn get_jupiter_traded_handler() -> Response {
    let cards = build_cards(cache::get_jupiter_traded().await).await;
    success_response(serde_json::json!({
        "success": true,
        "count": cards.len(),
        "tokens": cards,
    }))
}

/// GET /api/featured/dexscreener/trending — DexScreener trending tokens
pub(super) async fn get_dexscreener_trending_handler() -> Response {
    let cards = build_cards(cache::get_dexscreener_trending().await).await;
    success_response(serde_json::json!({
        "success": true,
        "count": cards.len(),
        "tokens": cards,
    }))
}
