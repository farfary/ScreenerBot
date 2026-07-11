//! Billboard route handlers — endpoint implementations and token enrichment.

use super::cache;
use super::logos::fill_missing_logos;
use super::types::*;
use crate::tokens;
use crate::webserver::utils::success_response;
use axum::response::{IntoResponse, Response};

/// Enrich a billboard token with data from local token database
async fn enrich_billboard_token(token: BillboardToken) -> EnrichedBillboardToken {
    let mint = token.mint.clone();

    // Try to get full token data from our database
    match tokens::get_full_token_async(&mint).await {
        Ok(Some(db_token)) => EnrichedBillboardToken {
            base: token,
            price_sol: Some(db_token.price_sol),
            price_usd: Some(db_token.price_usd),
            market_cap_usd: db_token.market_cap,
            fdv_usd: db_token.fdv,
            liquidity_usd: db_token.liquidity_usd,
            volume_24h: db_token.volume_h24,
            price_change_1h: db_token.price_change_h1,
            price_change_24h: db_token.price_change_h24,
            holder_count: db_token.total_holders,
            security_score: db_token.security_score,
            security_score_normalised: db_token.security_score_normalised,
            is_in_database: true,
            data_source: Some(db_token.data_source.as_str().to_owned()),
        },
        Ok(None) | Err(_) => {
            // Token not in database - return base with no enrichment
            EnrichedBillboardToken {
                base: token,
                price_sol: None,
                price_usd: None,
                market_cap_usd: None,
                fdv_usd: None,
                liquidity_usd: None,
                volume_24h: None,
                price_change_1h: None,
                price_change_24h: None,
                holder_count: None,
                security_score: None,
                security_score_normalised: None,
                is_in_database: false,
                data_source: None,
            }
        }
    }
}

/// Enrich multiple billboard tokens concurrently
async fn enrich_billboard_tokens(tokens: Vec<BillboardToken>) -> Vec<EnrichedBillboardToken> {
    let futures: Vec<_> = tokens.into_iter().map(enrich_billboard_token).collect();
    futures::future::join_all(futures).await
}

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

/// GET /api/billboard — Get featured tokens with enrichment
pub(super) async fn get_billboard_handler() -> Response {
    match cache::get_billboard().await {
        Ok(mut tokens) => {
            // Ensure tokens are tracked for future updates
            ensure_tokens_tracked(&tokens);

            // Resolve any logo the source did not supply (DB, then DexScreener)
            fill_missing_logos(&mut tokens).await;

            // Enrich with local database data
            let enriched = enrich_billboard_tokens(tokens).await;
            let count = enriched.len();

            success_response(serde_json::json!({
                "tokens": enriched,
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

/// GET /api/billboard/all — Get all billboard categories with enriched featured tokens
pub(super) async fn get_billboard_all_handler() -> Response {
    // Fetch all sources concurrently
    let (featured_result, mut jupiter_organic, mut jupiter_traded, mut dexscreener_trending) = tokio::join!(
        cache::get_billboard(),
        cache::get_jupiter_organic(),
        cache::get_jupiter_traded(),
        cache::get_dexscreener_trending()
    );

    let mut featured = featured_result.unwrap_or_default();

    // Ensure featured tokens are tracked for future updates
    ensure_tokens_tracked(&featured);

    // Resolve any logo the sources did not supply (DB, then DexScreener). Every
    // category needs this, not just featured: Jupiter and DexScreener both ship
    // tokens with a missing or malformed icon.
    tokio::join!(
        fill_missing_logos(&mut featured),
        fill_missing_logos(&mut jupiter_organic),
        fill_missing_logos(&mut jupiter_traded),
        fill_missing_logos(&mut dexscreener_trending),
    );

    // Enrich featured tokens with local database data
    let enriched_featured = enrich_billboard_tokens(featured).await;

    success_response(serde_json::json!({
        "success": true,
        "featured": enriched_featured,
        "jupiter_organic": jupiter_organic,
        "jupiter_traded": jupiter_traded,
        "dexscreener_trending": dexscreener_trending
    }))
}

/// GET /api/billboard/jupiter/organic — Get Jupiter top organic tokens
pub(super) async fn get_jupiter_organic_handler() -> Response {
    let mut tokens = cache::get_jupiter_organic().await;
    fill_missing_logos(&mut tokens).await;
    success_response(serde_json::json!({
        "success": true,
        "tokens": tokens,
        "count": tokens.len()
    }))
}

/// GET /api/billboard/jupiter/traded — Get Jupiter top traded tokens
pub(super) async fn get_jupiter_traded_handler() -> Response {
    let mut tokens = cache::get_jupiter_traded().await;
    fill_missing_logos(&mut tokens).await;
    success_response(serde_json::json!({
        "success": true,
        "tokens": tokens,
        "count": tokens.len()
    }))
}

/// GET /api/billboard/dexscreener/trending — Get DexScreener trending tokens
pub(super) async fn get_dexscreener_trending_handler() -> Response {
    let mut tokens = cache::get_dexscreener_trending().await;
    fill_missing_logos(&mut tokens).await;
    success_response(serde_json::json!({
        "success": true,
        "tokens": tokens,
        "count": tokens.len()
    }))
}
