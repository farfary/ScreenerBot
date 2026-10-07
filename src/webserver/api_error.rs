// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Typed API error envelope.
//!
//! `{"error":{"code","message","text","details","timestamp"}}` where `code` is a
//! stable machine category that fixes the HTTP status, `text` is the catalog
//! message the dashboard renders in the viewer's language, `message` is that
//! same catalog entry rendered in the source locale, and `details` carries
//! untranslated technical context.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::borrow::Cow;

use crate::i18n::{MessageId, UiArg, UiText};

/// Machine category of an API error. The category alone determines the HTTP
/// status, so a client can act on `code` without reading `message`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiErrorCode {
    InvalidInput,
    InvalidPassword,
    InvalidTotp,
    AuthDisabled,
    PasswordNotSet,
    CurrentPasswordRequired,
    Unauthorized,
    AuthenticationRequired,
    Forbidden,
    InvalidLocalRequest,
    InvalidToken,
    MissingToken,
    NotFound,
    Conflict,
    /// The request refers to state that changed since the client read it.
    StaleRequest,
    NoUpdateAvailable,
    SetupValidationRequired,
    TelegramDisabled,
    NotConfigured,
    ConfigError,
    DatabaseError,
    Internal,
    BrowserOpenFailed,
    ServiceUnavailable,
    InitializationRequired,
    UpstreamError,
    IntegrityFailed,
    NotImplemented,
    RateLimited,
    UpstreamTimeout,
    AgentControlDisabled,
    LiveConfirmationRequired,
    InvalidTask,
    TaskLimit,
    WatchRejected,
    LiveUnavailable,
    TaskLive,
    OpenPositions,
    CopyError,
    /// An uploaded body exceeds the size the endpoint accepts.
    PayloadTooLarge,
    /// No enabled swap route offers this shape of trade.
    RouteNotOffered,
}

impl ApiErrorCode {
    /// Wire form, identical to the serde representation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::InvalidPassword => "INVALID_PASSWORD",
            Self::InvalidTotp => "INVALID_TOTP",
            Self::AuthDisabled => "AUTH_DISABLED",
            Self::PasswordNotSet => "PASSWORD_NOT_SET",
            Self::CurrentPasswordRequired => "CURRENT_PASSWORD_REQUIRED",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::AuthenticationRequired => "AUTHENTICATION_REQUIRED",
            Self::Forbidden => "FORBIDDEN",
            Self::InvalidLocalRequest => "INVALID_LOCAL_REQUEST",
            Self::InvalidToken => "INVALID_TOKEN",
            Self::MissingToken => "MISSING_TOKEN",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::StaleRequest => "STALE_REQUEST",
            Self::NoUpdateAvailable => "NO_UPDATE_AVAILABLE",
            Self::SetupValidationRequired => "SETUP_VALIDATION_REQUIRED",
            Self::TelegramDisabled => "TELEGRAM_DISABLED",
            Self::NotConfigured => "NOT_CONFIGURED",
            Self::ConfigError => "CONFIG_ERROR",
            Self::DatabaseError => "DATABASE_ERROR",
            Self::Internal => "INTERNAL",
            Self::BrowserOpenFailed => "BROWSER_OPEN_FAILED",
            Self::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            Self::InitializationRequired => "INITIALIZATION_REQUIRED",
            Self::UpstreamError => "UPSTREAM_ERROR",
            Self::IntegrityFailed => "INTEGRITY_FAILED",
            Self::NotImplemented => "NOT_IMPLEMENTED",
            Self::RateLimited => "RATE_LIMITED",
            Self::UpstreamTimeout => "UPSTREAM_TIMEOUT",
            Self::AgentControlDisabled => "AGENT_CONTROL_DISABLED",
            Self::LiveConfirmationRequired => "LIVE_CONFIRMATION_REQUIRED",
            Self::InvalidTask => "INVALID_TASK",
            Self::TaskLimit => "TASK_LIMIT",
            Self::WatchRejected => "WATCH_REJECTED",
            Self::LiveUnavailable => "LIVE_UNAVAILABLE",
            Self::TaskLive => "TASK_LIVE",
            Self::OpenPositions => "OPEN_POSITIONS",
            Self::CopyError => "COPY_ERROR",
            Self::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
            Self::RouteNotOffered => "ROUTE_NOT_OFFERED",
        }
    }

    /// Category for an HTTP status a typed domain error already carries, so the
    /// response keeps that status. Statuses outside the table map to `Internal`.
    pub const fn for_status(status: u16) -> Self {
        match status {
            400 => Self::InvalidInput,
            401 => Self::Unauthorized,
            403 => Self::Forbidden,
            404 => Self::NotFound,
            409 => Self::Conflict,
            422 => Self::IntegrityFailed,
            429 => Self::RateLimited,
            502 => Self::UpstreamError,
            503 => Self::ServiceUnavailable,
            504 => Self::UpstreamTimeout,
            _ => Self::Internal,
        }
    }

    /// HTTP status carried by every error of this category.
    pub const fn status(self) -> StatusCode {
        match self {
            Self::InvalidInput
            | Self::AuthDisabled
            | Self::PasswordNotSet
            | Self::CurrentPasswordRequired
            | Self::StaleRequest
            | Self::NoUpdateAvailable
            | Self::SetupValidationRequired
            | Self::TelegramDisabled
            | Self::NotConfigured
            | Self::LiveConfirmationRequired
            | Self::InvalidTask
            | Self::TaskLimit
            | Self::WatchRejected => StatusCode::BAD_REQUEST,
            Self::InvalidPassword
            | Self::InvalidTotp
            | Self::Unauthorized
            | Self::AuthenticationRequired => StatusCode::UNAUTHORIZED,
            Self::Forbidden
            | Self::InvalidLocalRequest
            | Self::InvalidToken
            | Self::MissingToken
            | Self::AgentControlDisabled => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Conflict | Self::LiveUnavailable | Self::TaskLive | Self::OpenPositions => {
                StatusCode::CONFLICT
            }
            Self::IntegrityFailed | Self::RouteNotOffered => StatusCode::UNPROCESSABLE_ENTITY,
            Self::ConfigError
            | Self::DatabaseError
            | Self::Internal
            | Self::BrowserOpenFailed
            | Self::CopyError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotImplemented => StatusCode::NOT_IMPLEMENTED,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::UpstreamTimeout => StatusCode::GATEWAY_TIMEOUT,
            Self::UpstreamError => StatusCode::BAD_GATEWAY,
            Self::ServiceUnavailable | Self::InitializationRequired => {
                StatusCode::SERVICE_UNAVAILABLE
            }
        }
    }
}

/// A failed API call: category, localizable text and technical details.
#[derive(Debug, Clone)]
pub struct ApiError {
    code: ApiErrorCode,
    text: UiText,
    details: Option<String>,
}

impl ApiError {
    pub fn new(code: ApiErrorCode, id: MessageId) -> Self {
        Self {
            code,
            text: UiText::new(id),
            details: None,
        }
    }

    /// Error whose message is an already-built [`UiText`], for a typed domain
    /// error that owns its own catalog mapping.
    pub fn with_text(code: ApiErrorCode, text: UiText) -> Self {
        Self {
            code,
            text,
            details: None,
        }
    }

    /// The message and technical details, for a transport that carries them
    /// outside the HTTP error envelope.
    pub fn into_text_and_details(self) -> (UiText, Option<String>) {
        (self.text, self.details)
    }

    pub fn arg(mut self, name: impl Into<Cow<'static, str>>, value: UiArg) -> Self {
        self.text = self.text.arg(name, value);
        self
    }

    pub fn text_arg(self, name: impl Into<Cow<'static, str>>, value: impl Into<String>) -> Self {
        self.arg(name, UiArg::Text(value.into()))
    }

    pub fn count_arg(self, name: impl Into<Cow<'static, str>>, value: i64) -> Self {
        self.arg(name, UiArg::Count(value))
    }

    pub fn details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// The JSON envelope sent as the response body.
    pub fn body(&self) -> Value {
        json!({
            "error": {
                "code": self.code.as_str(),
                "message": self.text.render_source_plain(),
                "text": self.text,
                "details": self.details,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        })
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.code.status(), Json(self.body())).into_response()
    }
}

#[cfg(test)]
mod tests;
