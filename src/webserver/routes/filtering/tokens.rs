// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Filtering tokens route — lists tokens with their current filter evaluation results.

use axum::{
    body::Body,
    extract::Query,
    response::{IntoResponse as _, Response},
};
use chrono::{DateTime, Utc};

use crate::{
    chains::ChainScope,
    filtering::sources::rejection_text,
    i18n::{ids, source_locale, LanguageIdentifier},
    logger::{self, LogTag},
    tokens::{get_rejected_tokens_async, get_token_info_batch_async},
    webserver::{
        api_error::{ApiError, ApiErrorCode},
        utils::success_response,
    },
};

use super::types::{RejectedTokenEntry, RejectedTokensQuery};

/// GET /api/filtering/rejected-tokens
/// Get list of rejected tokens with pagination and filtering
pub async fn get_rejected_tokens_handler(Query(params): Query<RejectedTokensQuery>) -> Response {
    let limit = params.limit.unwrap_or(50).min(100); // Max 100 per page
    let offset = params.offset.unwrap_or_default();

    // The rejection tables are per chain; a route without a chain reads the only one.
    let rejected = match ChainScope::All.sole_chain() {
        Ok(chain) => get_rejected_tokens_async(
            chain,
            params.reason,
            params.source,
            params.search,
            limit,
            offset,
        )
        .await
        .map(|tokens| (chain, tokens)),
        Err(err) => Err(err.into()),
    };
    match rejected {
        Ok((chain, tokens)) => {
            // Collect mints for batch token info lookup
            let mints: Vec<String> = tokens.iter().map(|(mint, _, _, _)| mint.clone()).collect();

            // Fetch token info (symbol, name, image) in a single batch query
            let token_info = get_token_info_batch_async(chain, mints)
                .await
                .unwrap_or_default();

            let entries: Vec<RejectedTokenEntry> = tokens
                .into_iter()
                .map(|(mint, reason, source, ts)| {
                    let (symbol, name, image_url) =
                        token_info.get(&mint).cloned().unwrap_or((None, None, None));

                    RejectedTokenEntry {
                        mint,
                        symbol,
                        name,
                        image_url,
                        reason_text: rejection_text(&reason),
                        reason,
                        source,
                        rejected_at: DateTime::from_timestamp(ts, 0)
                            .unwrap_or_else(|| Utc::now())
                            .to_rfc3339(),
                    }
                })
                .collect();

            success_response(entries)
        }
        Err(err) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Failed to fetch rejected tokens: {:?}", err),
            );
            ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_FILTERING_REJECTED_TOKENS_FAILED,
            )
            .details(format!("{err:?}"))
            .into_response()
        }
    }
}

/// GET /api/filtering/export-rejected-tokens
/// Export rejected tokens to CSV
pub async fn export_rejected_tokens(Query(params): Query<RejectedTokensQuery>) -> Response {
    // Fetch up to 100,000 tokens for export
    let limit = 100000;
    let offset = 0;

    let rejected = match ChainScope::All.sole_chain() {
        Ok(chain) => {
            get_rejected_tokens_async(
                chain,
                params.reason,
                params.source,
                params.search,
                limit,
                offset,
            )
            .await
        }
        Err(err) => Err(err.into()),
    };
    match rejected {
        Ok(tokens) => {
            let mut wtr = csv::Writer::from_writer(vec![]);
            // Write header
            if let Err(e) =
                wtr.write_record(&["Mint", "Reason", "Display Label", "Source", "Rejected At"])
            {
                return ApiError::new(
                    ApiErrorCode::Internal,
                    ids::ERRORS_FILTERING_CSV_HEADER_FAILED,
                )
                .details(e.to_string())
                .into_response();
            }

            // Write records
            let locale: LanguageIdentifier = source_locale().parse().unwrap_or_default();
            for (mint, reason, source, ts) in tokens {
                let dt = DateTime::from_timestamp(ts, 0)
                    .unwrap_or_else(|| Utc::now())
                    .to_rfc3339();
                let reason_label = rejection_text(&reason).render_plain(&locale);

                if let Err(e) = wtr.write_record(&[mint, reason, reason_label, source, dt]) {
                    return ApiError::new(
                        ApiErrorCode::Internal,
                        ids::ERRORS_FILTERING_CSV_RECORD_FAILED,
                    )
                    .details(e.to_string())
                    .into_response();
                }
            }

            match wtr.into_inner() {
                Ok(data) => {
                    let filename =
                        format!("rejected_tokens_{}.csv", Utc::now().format("%Y%m%d_%H%M%S"));

                    Response::builder()
                        .header("Content-Type", "text/csv")
                        .header(
                            "Content-Disposition",
                            format!("attachment; filename=\"{}\"", filename),
                        )
                        .body(Body::from(data))
                        .unwrap_or_else(|_| {
                            ApiError::new(
                                ApiErrorCode::Internal,
                                ids::ERRORS_FILTERING_EXPORT_RESPONSE_FAILED,
                            )
                            .into_response()
                        })
                }
                Err(e) => ApiError::new(
                    ApiErrorCode::Internal,
                    ids::ERRORS_FILTERING_CSV_FINALIZE_FAILED,
                )
                .details(e.to_string())
                .into_response(),
            }
        }
        Err(err) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Failed to fetch rejected tokens for export: {:?}", err),
            );
            ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_FILTERING_REJECTED_TOKENS_FAILED,
            )
            .details(format!("{err:?}"))
            .into_response()
        }
    }
}
