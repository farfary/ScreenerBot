// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Blacklist management handlers.

use crate::{
    i18n::ids,
    webserver::api_error::{ApiError, ApiErrorCode},
};
use axum::{body::Bytes, extract::Path, Json};
use std::sync::Arc;

use super::types::*;
use crate::{
    logger::{self, LogTag},
    tokens::{
        cleanup,
        database::{database, TokenDatabase},
        BlacklistSource,
    },
};

/// The token database that owns `mint`, or the typed error naming `failed` as the operation
/// that could not start.
fn database_for(
    mint: &str,
    failed: crate::i18n::MessageId,
) -> Result<Arc<TokenDatabase>, ApiError> {
    let chain = crate::chains::chain_for_address(mint).map_err(|e| {
        logger::warning(
            LogTag::Webserver,
            &format!("Blacklist request refused for mint={mint}: {e}"),
        );
        ApiError::new(ApiErrorCode::InvalidInput, failed).details(e.to_string())
    })?;
    database(chain).ok_or_else(|| {
        ApiError::new(
            ApiErrorCode::ServiceUnavailable,
            ids::ERRORS_TOKENS_DATABASE_UNAVAILABLE,
        )
    })
}

/// The blacklist reason a request body names, or the default reason when it has no body.
///
/// The body is optional and no `Content-Type` is required: the dashboard's context menu
/// and scripted callers post a bare request.
fn reason_from_body(body: &Bytes) -> Result<String, ApiError> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(default_blacklist_reason());
    }
    serde_json::from_slice::<AddBlacklistRequest>(body)
        .map(|request| request.reason)
        .map_err(|e| {
            ApiError::new(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_TOKENS_BLACKLIST_FAILED,
            )
            .details(e.to_string())
        })
}

/// POST /api/tokens/:mint/blacklist
///
/// Add a token to the blacklist as a manual decision. The body is optional JSON
/// (`{"reason": "..."}`); a manual blacklist also forgets an earlier removal of the token.
pub async fn add_to_blacklist(
    Path(mint): Path<String>,
    body: Bytes,
) -> Result<Json<BlacklistResponse>, ApiError> {
    let reason = reason_from_body(&body)?;

    logger::debug(
        LogTag::Webserver,
        &format!("Adding to blacklist: mint={mint}, reason={reason}"),
    );

    let db = database_for(&mint, ids::ERRORS_TOKENS_BLACKLIST_FAILED)?;

    let mint_clone = mint.clone();
    let reason_clone = reason.clone();
    match tokio::task::spawn_blocking(move || {
        cleanup::blacklist_token(
            &mint_clone,
            &reason_clone,
            BlacklistSource::Manual.code(),
            &db,
        )
    })
    .await
    {
        Ok(Ok(_)) => {
            logger::info(
                LogTag::Webserver,
                &format!("Token blacklisted: mint={mint}, reason={reason}"),
            );
            Ok(Json(BlacklistResponse::flag(mint, true)))
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
/// Remove a token from the blacklist at the user's request. No automatic source files it
/// again; its pool and account blacklist rows go too (see `cleanup::remove_blacklisted_token`).
pub async fn remove_from_blacklist(
    Path(mint): Path<String>,
) -> Result<Json<BlacklistResponse>, ApiError> {
    logger::debug(
        LogTag::Webserver,
        &format!("Removing from blacklist: mint={mint}"),
    );

    let db = database_for(&mint, ids::ERRORS_TOKENS_UNBLACKLIST_FAILED)?;

    match cleanup::remove_blacklisted_token(&mint, &db).await {
        Ok(_) => {
            logger::info(
                LogTag::Webserver,
                &format!("Token removed from blacklist: mint={mint}"),
            );
            Ok(Json(BlacklistResponse::flag(mint, false)))
        }
        Err(e) => {
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
    }
}

/// GET /api/tokens/:mint/blacklist
///
/// Blacklist status of a token: the flag, and for a blacklisted token the reason, who
/// filed it (`source`) and when (`added_at`, unix seconds).
pub async fn get_blacklist_status(
    Path(mint): Path<String>,
) -> Result<Json<BlacklistResponse>, ApiError> {
    logger::debug(
        LogTag::Webserver,
        &format!("Checking blacklist status: mint={mint}"),
    );

    let db = database_for(&mint, ids::ERRORS_TOKENS_BLACKLIST_STATUS_FAILED)?;

    let mint_clone = mint.clone();
    match tokio::task::spawn_blocking(move || db.get_blacklist_entry(&mint_clone)).await {
        Ok(Ok(entry)) => {
            logger::debug(
                LogTag::Webserver,
                &format!(
                    "Blacklist status checked: mint={mint} blacklisted={}",
                    entry.is_some()
                ),
            );
            Ok(Json(match entry {
                Some(entry) => BlacklistResponse {
                    success: true,
                    mint,
                    is_blacklisted: true,
                    reason: Some(entry.reason),
                    source: Some(entry.source),
                    added_at: Some(entry.added_at),
                },
                None => BlacklistResponse::flag(mint, false),
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
    use axum::{
        body::Body,
        http::{header, Request, StatusCode},
        response::IntoResponse,
        routing::post,
        Router,
    };
    use tower::ServiceExt;

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
        let add = add_to_blacklist(Path(NOT_AN_ADDRESS.to_owned()), Bytes::new()).await;
        assert_eq!(status_of(add), StatusCode::BAD_REQUEST);

        let remove = remove_from_blacklist(Path(NOT_AN_ADDRESS.to_owned())).await;
        assert_eq!(status_of(remove), StatusCode::BAD_REQUEST);

        let status = get_blacklist_status(Path(NOT_AN_ADDRESS.to_owned())).await;
        assert_eq!(status_of(status), StatusCode::BAD_REQUEST);
    }

    /// The router, not the handler, rejects a request whose body does not fit the extractor:
    /// a `Json` extractor answers 415 to a bodiless POST with no `Content-Type`. Posting with
    /// no body, a bare reason, an empty object, a full object and no header must all reach
    /// the handler, which answers 400 for the mint that is not an address.
    #[tokio::test]
    async fn the_post_reaches_the_handler_with_or_without_a_body_or_content_type() {
        let cases: [(Option<&str>, Option<&str>); 5] = [
            (None, None),
            (Some("application/json"), Some("{}")),
            (
                Some("application/json"),
                Some(r#"{"reason":"manual note"}"#),
            ),
            (None, Some(r#"{"reason":"no content type"}"#)),
            (
                Some("application/json"),
                Some(r#"{"mint":"x","reason":"extra field"}"#),
            ),
        ];
        for (content_type, body) in cases {
            let router = Router::new().route("/tokens/{mint}/blacklist", post(add_to_blacklist));
            let mut request = Request::post(format!("/tokens/{NOT_AN_ADDRESS}/blacklist"));
            if let Some(content_type) = content_type {
                request = request.header(header::CONTENT_TYPE, content_type);
            }
            let response = router
                .oneshot(
                    request
                        .body(body.map_or_else(Body::empty, |text| Body::from(text.to_owned())))
                        .expect("request"),
                )
                .await
                .expect("response");
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "content type {content_type:?}, body {body:?}"
            );
        }
    }

    /// A body that is not the documented JSON is client input too, never a 415 or a 500.
    #[tokio::test]
    async fn a_malformed_body_is_rejected_as_input() {
        let add = add_to_blacklist(Path(NOT_AN_ADDRESS.to_owned()), Bytes::from("{not json")).await;
        assert_eq!(status_of(add), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn the_reason_defaults_only_when_the_body_is_empty() {
        assert_eq!(
            reason_from_body(&Bytes::new()).expect("empty body"),
            default_blacklist_reason()
        );
        assert_eq!(
            reason_from_body(&Bytes::from("  \n")).expect("blank body"),
            default_blacklist_reason()
        );
        assert_eq!(
            reason_from_body(&Bytes::from(r#"{"reason":"scam"}"#)).expect("reason"),
            "scam"
        );
        assert_eq!(
            reason_from_body(&Bytes::from("{}")).expect("empty object"),
            default_blacklist_reason()
        );
    }
}
