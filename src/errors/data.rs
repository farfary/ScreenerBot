#[derive(Debug, Clone)]
pub enum DataError {
    ParseError {
        data_type: String,
        error: String,
    },
    ValidationError {
        field: String,
        value: String,
        reason: String,
    },
    InvalidFormat {
        expected: String,
        received: String,
    },
    InvalidAmount {
        amount: String,
        reason: String,
    },
    Generic {
        message: String,
    },
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataError::ParseError { data_type, error } => {
                write!(f, "Failed to parse {}: {}", data_type, error)
            }
            DataError::InvalidAmount { amount, reason } => {
                write!(f, "Invalid amount '{}': {}", amount, reason)
            }
            DataError::Generic { message } => write!(f, "{}", message),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl std::error::Error for DataError {}
