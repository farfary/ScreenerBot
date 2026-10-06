// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Blacklist management handlers.

use crate::{
    i18n::ids,
    webserver::api_error::{ApiError, ApiErrorCode},
};
use axum::{extract::Path, Json};

use super::types::*;
use crate::{
    logger::{self, LogTag},
    tokens::{cleanup, database::database},
};

/// POST /api/tokens/:mint/blacklist
///
/// Add a token to the blacklist
pub async fn add_to_blacklist(
    Path(mint): Path<String>,
    Json(request): Json<Option<AddBlacklistRequest>>,
) -> Result<Json<BlacklistResponse>, ApiError> {
    let reason = request
        .map(|r| r.reason)
        .unwrap_or_else(|| "Manual blacklist via UI".to_owned());

    logger::debug(
        LogTag::Webserver,
        &format!("Adding to blacklist: mint={mint}, reason={reason}"),
    );

    let chain = match crate::chains::chain_for_address(&mint) {
        Ok(chain) => chain,
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to blacklist token mint={mint}: {e}"),
            );
            return Err(ApiError::new(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_TOKENS_BLACKLIST_FAILED,
            )
            .details(e.to_string()));
        }
    };
    let db = match database(chain) {
        Some(db) => db,
        None => {
            return Err(ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_TOKENS_DATABASE_UNAVAILABLE,
            ));
        }
    };

    // Use spawn_blocking for sync database operation
    let mint_clone = mint.clone();
    let reason_clone = reason.clone();
    match tokio::task::spawn_blocking(move || {
        cleanup::blacklist_token(&mint_clone, &reason_clone, "manual", &db)
    })
    .await
    {
        Ok(Ok(())) => {
            logger::info(
                LogTag::Webserver,
                &format!("Token blacklisted: mint={mint}, reason={reason}"),
            );
            Ok(Json(BlacklistResponse {
                success: true,
                mint,
                is_blacklisted: true,
            }))
        }
        Ok(Err(e)) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to blacklist token mint={mint}: {e}"),
            );
            Err(
                ApiError::new(ApiErrorCode::Internal, ids::ERRORS_TOKENS_BLACKLIST_FAILED)
                    .details(e.to_string()),
            )
        }
        Err(join_err) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Join error blacklisting token mint={mint}: {join_err}"),
            );
            Err(ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_TOKENS_BLACKLIST_INTERNAL,
            )
            .details(join_err.to_string()))
        }
    }
}

/// DELETE /api/tokens/:mint/blacklist
///
/// Remove a token from the blacklist
pub async fn remove_from_blacklist(
    Path(mint): Path<String>,
) -> Result<Json<BlacklistResponse>, ApiError> {
    logger::debug(
        LogTag::Webserver,
        &format!("Removing from blacklist: mint={mint}"),
    );

    let chain = match crate::chains::chain_for_address(&mint) {
        Ok(chain) => chain,
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to remove from blacklist mint={mint}: {e}"),
            );
            return Err(ApiError::new(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_TOKENS_UNBLACKLIST_FAILED,
            )
            .details(e.to_string()));
        }
    };
    let db = match database(chain) {
        Some(db) => db,
        None => {
            return Err(ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_TOKENS_DATABASE_UNAVAILABLE,
            ));
        }
    };

    // Use spawn_blocking for sync database operation
    let mint_clone = mint.clone();
    match tokio::task::spawn_blocking(move || cleanup::unblacklist_token(&mint_clone, &db)).await {
        Ok(Ok(())) => {
            logger::info(
                LogTag::Webserver,
                &format!("Token removed from blacklist: mint={mint}"),
            );
            Ok(Json(BlacklistResponse {
                success: true,
                mint,
                is_blacklisted: false,
            }))
        }
        Ok(Err(e)) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to remove from blacklist mint={mint}: {e}"),
            );
            Err(ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_TOKENS_UNBLACKLIST_FAILED,
            )
            .details(e.to_string()))
        }
        Err(join_err) => {
            logger::warning(
                LogTag::Webserver,
                &format!(
                    "Join error removing from blacklist mint={}: {}",
                    mint, join_err
                ),
            );
            Err(ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_TOKENS_UNBLACKLIST_INTERNAL,
            )
            .details(join_err.to_string()))
        }
    }
}

/// GET /api/tokens/:mint/blacklist
///
/// Get blacklist status for a token
pub async fn get_blacklist_status(
    Path(mint): Path<String>,
) -> Result<Json<BlacklistResponse>, ApiError> {
    logger::debug(
        LogTag::Webserver,
        &format!("Checking blacklist status: mint={mint}"),
    );

    let chain = match crate::chains::chain_for_address(&mint) {
        Ok(chain) => chain,
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to check blacklist status mint={mint}: {e}"),
            );
            return Err(ApiError::new(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_TOKENS_BLACKLIST_STATUS_FAILED,
            )
            .details(e.to_string()));
        }
    };
    let db = match database(chain) {
        Some(db) => db,
        None => {
            return Err(ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_TOKENS_DATABASE_UNAVAILABLE,
            ));
        }
    };

    // Use spawn_blocking for sync database operation
    let mint_clone = mint.clone();
    match tokio::task::spawn_blocking(move || db.is_blacklisted(&mint_clone)).await {
        Ok(Ok(is_blacklisted)) => {
            logger::debug(
                LogTag::Webserver,
                &format!(
                    "Blacklist status checked: mint={} blacklisted={}",
                    mint, is_blacklisted
                ),
            );
            Ok(Json(BlacklistResponse {
                success: true,
                mint,
                is_blacklisted,
            }))
        }
        Ok(Err(e)) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to check blacklist status mint={mint}: {e}"),
            );
            Err(ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_TOKENS_BLACKLIST_STATUS_FAILED,
            )
            .details(e.to_string()))
        }
        Err(join_err) => {
            logger::warning(
                LogTag::Webserver,
                &format!(
                    "Join error checking blacklist status mint={}: {}",
                    mint, join_err
                ),
            );
            Err(ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_TOKENS_BLACKLIST_STATUS_INTERNAL,
            )
            .details(join_err.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, response::IntoResponse};

    const NOT_AN_ADDRESS: &str = "not-a-mint-address";

    fn status_of(result: Result<Json<BlacklistResponse>, ApiError>) -> StatusCode {
        match result {
            Ok(_) => StatusCode::OK,
            Err(error) => error.into_response().status(),
        }
    }

    /// A mint that no enabled chain accepts is client input, so every
    /// blacklist route answers 400 before it reaches the token database.
    #[tokio::test]
    async fn a_mint_that_is_not_an_address_is_rejected_as_input() {
        let add = add_to_blacklist(Path(NOT_AN_ADDRESS.to_owned()), Json(None)).await;
        assert_eq!(status_of(add), StatusCode::BAD_REQUEST);

        let remove = remove_from_blacklist(Path(NOT_AN_ADDRESS.to_owned())).await;
        assert_eq!(status_of(remove), StatusCode::BAD_REQUEST);

        let status = get_blacklist_status(Path(NOT_AN_ADDRESS.to_owned())).await;
        assert_eq!(status_of(status), StatusCode::BAD_REQUEST);
    }
}
