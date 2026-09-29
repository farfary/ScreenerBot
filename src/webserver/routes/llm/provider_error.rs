//! Error envelope for a failed LLM provider call.

use crate::apis::llm::LlmError;
use crate::apis::Error as ApisError;
use crate::assistant::error::Error as AssistantError;
use crate::i18n::{ids, MessageId, UiText};
use crate::llm_analysis::error::Error as AnalysisError;
use crate::webserver::api_error::{ApiError, ApiErrorCode};

/// Builds the error for a failed provider call. Rejections that carry the
/// provider's own wording (bad key, refused request, malformed reply) show that
/// wording, since it is what the user needs to correct the provider settings.
/// Every other failure reports `failed` and keeps the technical text in
/// `details`. The status is always the internal-error status: the provider's
/// own status describes the upstream call, not this request's authorization.
pub(crate) fn provider_failure(failed: MessageId, error: &LlmError) -> ApiError {
    let wording = match error {
        LlmError::AuthError { message, .. }
        | LlmError::ApiError { message, .. }
        | LlmError::InvalidResponse { message, .. } => Some(message),
        LlmError::RateLimited { .. }
        | LlmError::Timeout { .. }
        | LlmError::NetworkError { .. }
        | LlmError::ParseError { .. }
        | LlmError::ProviderDisabled { .. }
        | LlmError::MissingApiKey { .. }
        | LlmError::AlreadyInitialized => None,
    };
    match wording {
        Some(reason) => ApiError::new(ApiErrorCode::Internal, ids::ERRORS_LLM_PROVIDER_REFUSED)
            .text_arg("reason", reason.clone()),
        None => ApiError::new(ApiErrorCode::Internal, failed),
    }
    .details(error.to_string())
}

/// Provider failure for a client error that may wrap an `LlmError`.
fn apis_failure(failed: MessageId, error: &ApisError) -> ApiError {
    match error {
        ApisError::Llm(llm) => provider_failure(failed, llm),
        ApisError::Network(_)
        | ApisError::Data(_)
        | ApisError::Internal(_)
        | ApisError::Disabled { .. }
        | ApisError::RateLimiter { .. }
        | ApisError::CreditsExhausted { .. }
        | ApisError::NotFound { .. }
        | ApisError::SourcesExhausted { .. } => {
            ApiError::new(ApiErrorCode::Internal, failed).details(error.to_string())
        }
    }
}

/// Failure of an assistant turn; a wrapped provider rejection shows the
/// provider's wording, everything else reports `failed`.
pub(crate) fn assistant_failure(failed: MessageId, error: &AssistantError) -> ApiError {
    match error {
        AssistantError::Apis(apis) => apis_failure(failed, apis),
        AssistantError::Database(_)
        | AssistantError::Internal(_)
        | AssistantError::Io(_)
        | AssistantError::InvalidParameters { .. }
        | AssistantError::ProviderNotConfigured { .. }
        | AssistantError::Timeout { .. }
        | AssistantError::UnknownScheduleType { .. }
        | AssistantError::TaskNotFound { .. }
        | AssistantError::RunRecord { .. } => {
            ApiError::new(ApiErrorCode::Internal, failed).details(error.to_string())
        }
    }
}

/// Catalog text and technical detail of a failed assistant turn, for the
/// streaming transport. Same wording selection as [`assistant_failure`].
pub(crate) fn assistant_failure_text(
    failed: MessageId,
    error: &AssistantError,
) -> (UiText, String) {
    let (text, details) = assistant_failure(failed, error).into_text_and_details();
    (text, details.unwrap_or_default())
}

/// Failure of a model analysis; a wrapped provider rejection shows the
/// provider's wording, everything else reports `failed`.
pub(crate) fn analysis_failure(failed: MessageId, error: &AnalysisError) -> ApiError {
    match error {
        AnalysisError::Apis(apis) => apis_failure(failed, apis),
        AnalysisError::Database(_)
        | AnalysisError::Internal(_)
        | AnalysisError::Data(_)
        | AnalysisError::Io(_)
        | AnalysisError::Disabled
        | AnalysisError::ProviderNotConfigured { .. }
        | AnalysisError::RateLimited { .. }
        | AnalysisError::Timeout { .. }
        | AnalysisError::InvalidParameters { .. } => {
            ApiError::new(ApiErrorCode::Internal, failed).details(error.to_string())
        }
    }
}
