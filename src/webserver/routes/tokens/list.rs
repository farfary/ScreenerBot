//! Token listing, stats, filtering, and search handlers

use axum::{extract::Query, http::StatusCode, Json};
use std::collections::HashMap;

use super::types::*;
use crate::{
    filtering::{self, FilteringView, SnapshotState},
    logger::{self, LogTag},
};

const MAX_PAGE_SIZE: usize = 200;

/// GET /api/tokens/list
///
/// Query: view, search, sort_by, sort_dir, cursor, limit, page, page_size,
/// has_pool_price, has_open_position
pub(crate) async fn get_tokens_list(
    Query(query): Query<TokenListQuery>,
) -> Json<TokenListResponse> {
    let max_page_size = MAX_PAGE_SIZE;
    let request_view = query.view.clone();
    let filtering_query = query.into_filtering_query(max_page_size);
    let view = FilteringView::from_str(&request_view);

    // Return promotional fixtures only for owner-initiated media capture. The real
    // list is whatever this machine's database happens to hold, which on a fresh
    // install is a handful of rows and several empty views.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return Json(crate::webserver::promo::get_promo_tokens_list(
            &filtering_query,
        ));
    }

    match filtering::query_tokens(filtering_query).await {
        Ok(result) => {
            logger::debug(
                LogTag::Webserver,
                &format!(
                    "view={} page={}/{} items={}/{}",
                    request_view,
                    result.page,
                    result.total_pages,
                    result.items.len(),
                    result.total
                ),
            );

            Json(build_token_list_response(result, view))
        }
        Err(err) => {
            logger::info(
                LogTag::Webserver,
                &format!("Failed to load tokens list via filtering service: {err}"),
            );

            Json(TokenListResponse {
                items: vec![],
                page: 1,
                page_size: max_page_size,
                total: 0,
                total_pages: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
                cursor: Some(0),
                next_cursor: None,
                prev_cursor: None,
                priced_total: 0,
                positions_total: 0,
                blacklisted_total: 0,
                rejection_reasons: HashMap::new(),
                available_rejection_reasons: Vec::new(),
                blacklist_reasons: HashMap::new(),
            })
        }
    }
}

/// GET /api/tokens/stats
///
/// Get token statistics from the filtering service
///
/// Non-blocking: serves the snapshot only if one already exists. Waiting for the first
/// build here stalled the tokens tab for up to 30 seconds on a fresh launch. While it is
/// building the counts are reported absent with `snapshot_state: "building"`, never as
/// zeros — the tab polls, and the real counts land on the next tick.
pub async fn get_tokens_stats() -> Result<Json<TokenStatsResponse>, StatusCode> {
    // Return promotional fixtures only for owner-initiated media capture. The
    // snapshot is reported Ready because the promo universe is always built.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        let (total, priced, positions, blacklisted, with_ohlcv) =
            crate::webserver::promo::get_promo_tokens_stats();
        return Ok(Json(TokenStatsResponse {
            snapshot_state: SnapshotState::Ready,
            total_tokens_in_database: Some(total),
            total_tokens: Some(total),
            with_pool_price: Some(priced),
            open_positions: Some(positions),
            blacklisted: Some(blacklisted),
            with_ohlcv: Some(with_ohlcv),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }));
    }

    let snapshot = filtering::try_fetch_stats().await;

    if let Some(snapshot) = snapshot.as_ref() {
        logger::info(
            LogTag::Webserver,
            &format!(
                "db_total={} snapshot={} pool={} open={} blacklist={}",
                snapshot.total_tokens_in_database,
                snapshot.total_tokens,
                snapshot.with_pool_price,
                snapshot.open_positions,
                snapshot.blacklisted
            ),
        );
    }

    Ok(Json(TokenStatsResponse {
        snapshot_state: SnapshotState::of(&snapshot),
        total_tokens_in_database: snapshot.as_ref().map(|s| s.total_tokens_in_database),
        total_tokens: snapshot.as_ref().map(|s| s.total_tokens),
        with_pool_price: snapshot.as_ref().map(|s| s.with_pool_price),
        open_positions: snapshot.as_ref().map(|s| s.open_positions),
        blacklisted: snapshot.as_ref().map(|s| s.blacklisted),
        with_ohlcv: snapshot.as_ref().map(|s| s.with_ohlcv),
        timestamp: snapshot
            .as_ref()
            .map(|s| s.updated_at.to_rfc3339())
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    }))
}

/// POST /api/tokens/filter
///
/// Filter tokens with advanced criteria
pub async fn filter_tokens(
    Json(filter): Json<FilterRequest>,
) -> Result<Json<TokenListResponse>, StatusCode> {
    logger::info(
        LogTag::Webserver,
        &format!("view={} search='{}'", filter.view, filter.search),
    );

    let max_page_size = MAX_PAGE_SIZE;
    let view = FilteringView::from_str(&filter.view);
    let filtering_query = filter.into_filtering_query(max_page_size);

    match filtering::query_tokens(filtering_query).await {
        Ok(result) => Ok(Json(build_token_list_response(result, view))),
        Err(err) => {
            logger::info(LogTag::Webserver, &format!("Filtering query failed: {err}"));
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/tokens/search
///
/// Search for tokens by name, symbol, or mint address.
/// Uses DexScreener search API for name/symbol queries, and direct lookup for mint addresses.
///
/// Query: q (required), limit (optional, default 20, max 50)
pub async fn search_tokens(
    Query(query): Query<TokenSearchQuery>,
) -> Result<Json<TokenSearchResponse>, (StatusCode, Json<serde_json::Value>)> {
    let search_query = query.q.trim();

    if search_query.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
              "success": false,
              "error": "Search query 'q' is required"
            })),
        ));
    }

    logger::debug(
        LogTag::Webserver,
        &format!(
            "Token search: q='{}', limit={:?}",
            search_query, query.limit
        ),
    );

    match crate::tokens::search_tokens(search_query, query.limit).await {
        Ok(results) => {
            logger::info(
                LogTag::Webserver,
                &format!(
                    "Token search completed: q='{}', results={}",
                    search_query, results.total
                ),
            );

            Ok(Json(TokenSearchResponse {
                results: results.results,
                query: results.query,
                total: results.total,
            }))
        }
        Err(err) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Token search failed: q='{search_query}', error={err}"),
            );

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "error": err
                })),
            ))
        }
    }
}
