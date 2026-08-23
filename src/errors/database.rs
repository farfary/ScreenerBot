//! Database-related error classifications.
//!
//! Note: Keep errors `Clone` by storing messages as strings (do not store raw rusqlite errors).

#[derive(Debug, Clone, thiserror::Error)]
pub enum DatabaseError {
    #[error("database connection error: {message}")]
    Connection { message: String },
    #[error("sqlite error: {message}")]
    Sqlite { message: String },
    #[error("database query error (op={operation}): {message}")]
    Query { operation: String, message: String },
}

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
