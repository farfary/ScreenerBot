//! ScreenerBot account errors — signing in to screenerbot.io from the app.
//!
//! Distinct from `NetworkError` on purpose. "The server refused your password"
//! and "the server could not be reached" call for different words to the user
//! and different behaviour from the caller: one is final until they change
//! something, the other is worth retrying on its own.
//!
//! `Refused` carries the SERVER'S sentence rather than one composed here, so
//! the app and the website say the same thing about the same problem.

#[derive(Debug, Clone, thiserror::Error)]
pub enum AccountError {
    /// The server declined the sign-in and explained why.
    #[error("{message}")]
    Refused { message: String },

    /// An operation needed an account and there is none.
    #[error("not signed in to a ScreenerBot account")]
    NotSignedIn,

    /// The stored session no longer works — revoked, expired, or rotated out
    /// from under us. The user must sign in again; nothing is retryable.
    #[error("your ScreenerBot session has ended. Sign in again from Settings.")]
    SessionEnded,

    /// A response arrived but did not mean anything we understand. Almost
    /// always an old binary against a newer server, or a captive portal.
    #[error("unexpected response from screenerbot.io: {message}")]
    UnexpectedResponse { message: String },

    /// Local storage of the session failed.
    #[error("could not save the ScreenerBot session: {message}")]
    Storage { message: String },

    #[error("{message}")]
    Generic { message: String },
}
