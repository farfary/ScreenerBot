#[derive(Debug, Clone)]
pub enum RateLimitError {
    ExceededLimit {
        limit_type: String,
        current_rate: f64,
        limit: f64,
    },
    TemporaryThrottle {
        duration_seconds: u64,
    },
    Generic {
        message: String,
    },
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::ExceededLimit {
                limit_type,
                current_rate,
                limit,
            } => {
                write!(
                    f,
                    "Rate limit exceeded for {}: {}/s > {}/s",
                    limit_type, current_rate, limit
                )
            }
            RateLimitError::Generic { message } => write!(f, "{}", message),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl std::error::Error for RateLimitError {}

