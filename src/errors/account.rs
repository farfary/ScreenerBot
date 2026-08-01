//! ScreenerBot account errors — signing in to screenerbot.io from the app.
//!
//! Distinct from `NetworkError` on purpose. "The server refused your password"
//! and "the server could not be reached" call for different words to the user
//! and different behaviour from the caller: one is final until they change
//! something, the other is worth retrying on its own.
//!
//! `Refused` carries the SERVER'S sentence rather than one composed here, so
//! the app and the website say the same thing about the same problem.

#[derive(Debug, Clone)]
pub enum AccountError {
    /// The server declined the sign-in and explained why.
    Refused {
        message: String,
    },

    /// An operation needed an account and there is none.
    NotSignedIn,

    /// The stored session no longer works — revoked, expired, or rotated out
    /// from under us. The user must sign in again; nothing is retryable.
    SessionEnded,

    /// A response arrived but did not mean anything we understand. Almost
    /// always an old binary against a newer server, or a captive portal.
    UnexpectedResponse {
        message: String,
    },

    /// Local storage of the session failed.
    Storage {
        message: String,
    },

    Generic {
        message: String,
    },
}

impl std::fmt::Display for AccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountError::Refused { message } => write!(f, "{message}"),
            AccountError::NotSignedIn => {
                write!(f, "Not signed in to a ScreenerBot account")
            }
            AccountError::SessionEnded => write!(
                f,
                "Your ScreenerBot session has ended. Sign in again from Settings."
            ),
            AccountError::UnexpectedResponse { message } => {
                write!(f, "Unexpected response from screenerbot.io: {message}")
            }
            AccountError::Storage { message } => {
                write!(f, "Could not save the ScreenerBot session: {message}")
            }
            AccountError::Generic { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for AccountError {}
