// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Maps a rejected multi-wallet tool configuration onto an API error.

use crate::i18n::ids;
use crate::tools::Error;
use crate::webserver::api_error::{ApiError, ApiErrorCode};

/// A configuration check failure is always a 400. The check's own wording is
/// shown as the reason; every other error kind reports the operation only,
/// with the error text in `details`.
pub(super) fn invalid_config(error: &Error) -> ApiError {
    match error {
        Error::InvalidConfig { detail } => {
            ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_TOOLS_CONFIG_INVALID)
                .text_arg("reason", detail.as_str())
        }
        Error::Database(_)
        | Error::Wallets(_)
        | Error::RowDecode { .. }
        | Error::Migration { .. }
        | Error::MainWalletUnavailable { .. }
        | Error::Search { .. }
        | Error::AtaCache { .. }
        | Error::Dependency { .. } => ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_TOOLS_CONFIG_REJECTED,
        )
        .details(error.to_string()),
    }
}
