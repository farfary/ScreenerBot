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

    match crate::tokens::get_favorites_async().await {
        Ok(favorites) => {
            let total = favorites.len();
            logger::info(LogTag::Webserver, &format!("Fetched {} favorites", total));
            Ok(Json(FavoritesListResponse { favorites, total }))
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
                logger::info(
                    LogTag::Webserver,
                    &format!("Removed favorite: mint={mint}"),
                );
                Ok(Json(FavoriteResponse {
                    success: true,
                    favorite: None,
                    message: Some("Favorite removed".to_string()),
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
            logger::info(
                LogTag::Webserver,
                &format!("Updated favorite: mint={mint}"),
            );
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
