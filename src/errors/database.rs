//! Database-related error classifications.
//!
//! Note: Keep errors `Clone` by storing messages as strings (do not store raw rusqlite errors).

#[derive(Debug, Clone)]
pub enum DatabaseError {
    Connection { message: String },
    Sqlite { message: String },
    Migration { message: String },
    Query { operation: String, message: String },
    Generic { message: String },
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::Connection { message } => {
                write!(f, "Database connection error: {message}")
            }
            DatabaseError::Sqlite { message } => write!(f, "SQLite error: {message}"),
            DatabaseError::Migration { message } => {
                write!(f, "Database migration error: {message}")
            }
            DatabaseError::Query { operation, message } => {
                write!(f, "Database query error (op={operation}): {message}")
            }
            DatabaseError::Generic { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for DatabaseError {}

impl From<rusqlite::Error> for DatabaseError {
    fn from(err: rusqlite::Error) -> Self {
        DatabaseError::Sqlite {
            message: err.to_string(),
        }
    }
}

impl From<r2d2::Error> for DatabaseError {
    fn from(err: r2d2::Error) -> Self {
        DatabaseError::Connection {
            message: err.to_string(),
        }
    }
}
