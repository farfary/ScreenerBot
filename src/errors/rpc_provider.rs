use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum RpcProviderError {
    ProviderDown {
        provider_name: String,
        since: DateTime<Utc>,
    },
    RateLimitExceeded {
        provider_name: String,
        limit_type: String,
        reset_at: DateTime<Utc>,
    },
    MalformedResponse {
        provider_name: String,
        endpoint: String,
        response_body: String,
    },
    ApiKeyInvalid {
        provider_name: String,
    },
    Generic {
        provider_name: String,
        message: String,
    },
}

impl std::fmt::Display for RpcProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcProviderError::ProviderDown {
                provider_name,
                since,
            } => {
                write!(f, "Provider {provider_name} down since {since}")
            }
            RpcProviderError::Generic {
                provider_name,
                message,
            } => {
                write!(f, "Provider {provider_name} error: {message}")
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

impl std::error::Error for RpcProviderError {}
