//! Tool favorites handlers

use axum::{extract::Path, extract::Query, http::StatusCode, response::Response, Json};

use crate::logger::{self, LogTag};
use crate::tools::database::{
    get_tool_favorites, increment_tool_favorite_use, remove_tool_favorite,
    update_tool_favorite as db_update_tool_favorite, upsert_tool_favorite,
};
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// =============================================================================
// Tool Favorites Handlers
// =============================================================================

/// Get all tool favorites (optionally filtered by tool_type query param)
pub async fn get_favorites_list(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let tool_type = params.get("tool_type").map(String::as_str);

    match get_tool_favorites(tool_type) {
        Ok(favorites) => {
            let total = favorites.len();
            success_response(ToolFavoritesListResponse { favorites, total })
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to get favorites",
            Some(&e),
        ),
    }
}

/// Add a new tool favorite
pub async fn add_favorite(Json(request): Json<AddToolFavoriteRequest>) -> Response {
    // Validate tool_type
    let valid_types = ["buy_multi", "sell_multi", "token_watch"];
    if !valid_types.contains(&request.tool_type.as_str()) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_TOOL_TYPE",
            "Invalid tool type",
            Some(&format!("Must be one of: {:?}", valid_types)),
        );
    }

    match upsert_tool_favorite(
        &request.mint,
        request.symbol.as_deref(),
        request.name.as_deref(),
        request.logo_url.as_deref(),
        &request.tool_type,
        request.config_json.as_deref(),
        request.label.as_deref(),
        request.notes.as_deref(),
    ) {
        Ok(id) => {
            logger::info(
                LogTag::Tools,
                &format!(
                    "Added tool favorite: {} for {}",
                    request.mint, request.tool_type
                ),
            );
            success_response(serde_json::json!({ "id": id, "success": true }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to add favorite",
            Some(&e),
        ),
    }
}

/// Update a tool favorite
pub async fn update_favorite(
    Path(id): Path<i64>,
    Json(request): Json<UpdateToolFavoriteRequest>,
) -> Response {
    match db_update_tool_favorite(
        id,
        request.config_json.as_deref(),
        request.label.as_deref(),
        request.notes.as_deref(),
    ) {
        Ok(true) => success_response(serde_json::json!({ "success": true })),
        Ok(false) => error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Favorite not found",
            None,
        ),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to update favorite",
            Some(&e),
        ),
    }
}

/// Delete a tool favorite
pub async fn delete_favorite(Path(id): Path<i64>) -> Response {
    match remove_tool_favorite(id) {
        Ok(true) => {
            logger::info(LogTag::Tools, &format!("Removed tool favorite: {id}"));
            success_response(serde_json::json!({ "success": true }))
        }
        Ok(false) => error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Favorite not found",
            None,
        ),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to delete favorite",
            Some(&e),
        ),
    }
}

/// Mark a favorite as used (increment counter)
pub async fn mark_favorite_used(Path(id): Path<i64>) -> Response {
    match increment_tool_favorite_use(id) {
        Ok(()) => success_response(serde_json::json!({ "success": true })),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to update use count",
            Some(&e),
        ),
    }
}
