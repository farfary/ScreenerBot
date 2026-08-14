//! Favorites management handlers

use axum::{extract::Path, http::StatusCode, Json};

use super::types::*;
use crate::{
    logger::{self, LogTag},
    tokens::favorites::{AddFavoriteRequest, UpdateFavoriteRequest},
};

/// GET /api/tokens/favorites
///
/// Get all favorite tokens ordered by creation date (newest first)
pub async fn get_favorites(
) -> Result<Json<FavoritesListResponse>, (StatusCode, Json<serde_json::Value>)> {
    logger::debug(LogTag::Webserver, "Fetching token favorites");

    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        let favorites = crate::webserver::promo::get_promo_favorites();
        let total = favorites.len();
        return Ok(Json(FavoritesListResponse { favorites, total }));
    }

    match crate::tokens::get_favorites_async().await {
        Ok(favorites) => {
            let total = favorites.len();
            // Enrich each favorite with the FULL assembled token (market data,
            // price, txns, etc.) so the favorites subtab renders the exact same
            // columns as the all/passed token lists. Fall back to the favorite's
            // stored metadata when the token isn't in the store (columns then show
            // "—" for the missing market fields). Merge in trading-state flags and
            // the favorite extras so row actions and the details dialog work.
            let mut rows = Vec::with_capacity(total);
            for favorite in favorites {
                let has_open_position =
                    crate::positions::state::is_open_position(&favorite.mint).await;
                let blacklisted = crate::trader::safety::is_blacklisted(&favorite.mint).await;

                let mut row = match crate::tokens::get_full_token_async(&favorite.mint).await {
                    Ok(Some(token)) => {
                        serde_json::to_value(&token).unwrap_or_else(|_| serde_json::json!({}))
                    }
                    _ => serde_json::json!({
                        "mint": favorite.mint,
                        "symbol": favorite.symbol,
                        "name": favorite.name,
                        "logo_url": favorite.logo_url,
                    }),
                };

                if let serde_json::Value::Object(map) = &mut row {
                    map.insert("mint".to_owned(), serde_json::json!(favorite.mint));
                    map.insert("is_favorite".to_owned(), serde_json::json!(true));
                    map.insert("notes".to_owned(), serde_json::json!(favorite.notes));
                    map.insert(
                        "favorite_created_at".to_owned(),
                        serde_json::json!(favorite.created_at),
                    );
                    map.insert(
                        "has_open_position".to_owned(),
                        serde_json::json!(has_open_position),
                    );
                    map.insert("blacklisted".to_owned(), serde_json::json!(blacklisted));
                }

                rows.push(row);
            }
            logger::info(LogTag::Webserver, &format!("Fetched {total} favorites"));
            Ok(Json(FavoritesListResponse {
                favorites: rows,
                total,
            }))
        }
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to fetch favorites: {e}"),
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "error": format!("Failed to fetch favorites: {e}")
                })),
            ))
        }
    }
}

/// POST /api/tokens/favorites
///
/// Add a token to favorites
pub async fn add_favorite(
    Json(request): Json<AddFavoriteRequest>,
) -> Result<Json<FavoriteResponse>, (StatusCode, Json<serde_json::Value>)> {
    logger::debug(
        LogTag::Webserver,
        &format!("Adding favorite: mint={}", request.mint),
    );

    match crate::tokens::add_favorite_async(request.clone()).await {
        Ok(favorite) => {
            logger::info(
                LogTag::Webserver,
                &format!(
                    "Added favorite: mint={} symbol={:?}",
                    favorite.mint, favorite.symbol
                ),
            );
            Ok(Json(FavoriteResponse {
                success: true,
                favorite: Some(favorite),
                message: None,
            }))
        }
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to add favorite mint={}: {}", request.mint, e),
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "error": format!("Failed to add favorite: {e}")
                })),
            ))
        }
    }
}

/// DELETE /api/tokens/favorites/:mint
///
/// Remove a token from favorites
pub async fn remove_favorite(
    Path(mint): Path<String>,
) -> Result<Json<FavoriteResponse>, (StatusCode, Json<serde_json::Value>)> {
    logger::debug(
        LogTag::Webserver,
        &format!("Removing favorite: mint={mint}"),
    );

    match crate::tokens::remove_favorite_async(mint.clone()).await {
        Ok(removed) => {
            if removed {
                logger::info(LogTag::Webserver, &format!("Removed favorite: mint={mint}"));
                Ok(Json(FavoriteResponse {
                    success: true,
                    favorite: None,
                    message: Some("Favorite removed".to_owned()),
                }))
            } else {
                logger::debug(
                    LogTag::Webserver,
                    &format!("Favorite not found: mint={mint}"),
                );
                Err((
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                      "success": false,
                      "error": "Favorite not found"
                    })),
                ))
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to remove favorite mint={mint}: {e}"),
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "error": format!("Failed to remove favorite: {e}")
                })),
            ))
        }
    }
}

/// PATCH /api/tokens/favorites/:mint
///
/// Update a favorite's notes or metadata
pub async fn update_favorite(
    Path(mint): Path<String>,
    Json(request): Json<UpdateFavoriteRequest>,
) -> Result<Json<FavoriteResponse>, (StatusCode, Json<serde_json::Value>)> {
    logger::debug(
        LogTag::Webserver,
        &format!("Updating favorite: mint={mint}"),
    );

    match crate::tokens::update_favorite_async(mint.clone(), request).await {
        Ok(Some(favorite)) => {
            logger::info(LogTag::Webserver, &format!("Updated favorite: mint={mint}"));
            Ok(Json(FavoriteResponse {
                success: true,
                favorite: Some(favorite),
                message: None,
            }))
        }
        Ok(None) => {
            logger::debug(
                LogTag::Webserver,
                &format!("Favorite not found for update: mint={mint}"),
            );
            Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                  "success": false,
                  "error": "Favorite not found"
                })),
            ))
        }
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to update favorite mint={mint}: {e}"),
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "error": format!("Failed to update favorite: {e}")
                })),
            ))
        }
    }
}
