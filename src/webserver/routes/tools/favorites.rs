// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tool favorites handlers.

use axum::{extract::Path, extract::Query, response::Response, Json};
use std::collections::HashMap;

use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::tools::database::{
    get_tool_favorites, increment_tool_favorite_use, remove_tool_favorite,
    update_tool_favorite as db_update_tool_favorite, upsert_tool_favorite,
};
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;
use axum::response::IntoResponse as _;

use super::types::*;

// =============================================================================
// Tool Favorites Handlers
// =============================================================================

/// Get all tool favorites (optionally filtered by tool_type query param)
pub async fn get_favorites_list(Query(params): Query<HashMap<String, String>>) -> Response {
    let tool_type = params.get("tool_type").map(String::as_str);

    match get_tool_favorites(tool_type) {
        Ok(favorites) => {
            let total = favorites.len();
            success_response(ToolFavoritesListResponse { favorites, total })
        }
        Err(e) => ApiError::new(
            ApiErrorCode::DatabaseError,
            ids::ERRORS_TOOLS_FAVORITES_LIST_FAILED,
        )
        .details(e.to_string())
        .into_response(),
    }
}

/// Add a new tool favorite
pub async fn add_favorite(Json(request): Json<AddToolFavoriteRequest>) -> Response {
    // Validate tool_type
    let valid_types = ["buy_multi", "sell_multi", "token_watch"];
    if !valid_types.contains(&request.tool_type.as_str()) {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_TOOLS_FAVORITE_TYPE_INVALID,
        )
        .text_arg("types", valid_types.join(", "))
        .into_response();
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
        Err(e) => ApiError::new(
            ApiErrorCode::DatabaseError,
            ids::ERRORS_TOOLS_FAVORITE_ADD_FAILED,
        )
        .details(e.to_string())
        .into_response(),
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
        Ok(false) => ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_TOOLS_FAVORITE_NOT_FOUND)
            .into_response(),
        Err(e) => ApiError::new(
            ApiErrorCode::DatabaseError,
            ids::ERRORS_TOOLS_FAVORITE_UPDATE_FAILED,
        )
        .details(e.to_string())
        .into_response(),
    }
}

/// Delete a tool favorite
pub async fn delete_favorite(Path(id): Path<i64>) -> Response {
    match remove_tool_favorite(id) {
        Ok(true) => {
            logger::info(LogTag::Tools, &format!("Removed tool favorite: {id}"));
            success_response(serde_json::json!({ "success": true }))
        }
        Ok(false) => ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_TOOLS_FAVORITE_NOT_FOUND)
            .into_response(),
        Err(e) => ApiError::new(
            ApiErrorCode::DatabaseError,
            ids::ERRORS_TOOLS_FAVORITE_DELETE_FAILED,
        )
        .details(e.to_string())
        .into_response(),
    }
}

/// Mark a favorite as used (increment counter)
pub async fn mark_favorite_used(Path(id): Path<i64>) -> Response {
    match increment_tool_favorite_use(id) {
        Ok(()) => success_response(serde_json::json!({ "success": true })),
        Err(e) => ApiError::new(
            ApiErrorCode::DatabaseError,
            ids::ERRORS_TOOLS_FAVORITE_USE_FAILED,
        )
        .details(e.to_string())
        .into_response(),
    }
}
