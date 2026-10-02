// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategies route utilities — helper functions for strategy response formatting.

use axum::response::{IntoResponse, Response};
use std::fmt::Display;

use crate::i18n::MessageId;
use crate::webserver::api_error::{ApiError, ApiErrorCode};

/// Error response for a failed strategy operation.
pub fn err(code: ApiErrorCode, id: MessageId) -> Response {
    ApiError::new(code, id).into_response()
}

/// Error response whose technical cause travels in `details`.
pub fn err_cause(code: ApiErrorCode, id: MessageId, cause: &impl Display) -> Response {
    ApiError::new(code, id)
        .details(cause.to_string())
        .into_response()
}
