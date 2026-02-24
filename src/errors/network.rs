#[derive(Debug, Clone)]
pub enum NetworkError {
    ConnectionTimeout {
        endpoint: String,
        timeout_ms: u64,
    },
    ConnectionRefused {
        endpoint: String,
        reason: String,
    },
    HttpStatusError {
        endpoint: String,
        status: u16,
        body: Option<String>,
    },
    DnsResolutionFailed {
        hostname: String,
        error: String,
    },
    TlsHandshakeFailed {
        endpoint: String,
        error: String,
    },
    Generic {
        message: String,
    },
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkError::ConnectionTimeout {
                endpoint,
                timeout_ms,
            } => {
                write!(
                    f,
                    "Connection timeout to {} after {}ms",
                    endpoint, timeout_ms
                )
            }
            NetworkError::HttpStatusError {
                endpoint,
                status,
                body,
            } => {
                write!(
                    f,
                    "HTTP {} from {}: {}",
                    status,
                    endpoint,
                    body.as_deref().unwrap_or("No body")
                )
            }
            NetworkError::Generic { message } => write!(f, "{message}"),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl std::error::Error for NetworkError {}
