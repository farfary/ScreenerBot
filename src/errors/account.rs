// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! ScreenerBot account errors — signing in to screenerbot.io from the app.
//!
//! Distinct from `NetworkError` on purpose. "The server refused your password"
//! and "the server could not be reached" call for different words to the user
//! and different behaviour from the caller: one is final until they change
//! something, the other is worth retrying on its own.
//!
//! `Refused` carries the SERVER'S sentence rather than one composed here, so
//! the app and the website say the same thing about the same problem.
//!
//! `Refused` and `Unavailable` are the two halves of a failed token call, and
//! the distinction decides whether a session survives: a refusal is the
//! server's verdict, an unavailable answer (rate limit, server error, a body
//! that is not an OAuth error) says nothing about the session at all.

#[derive(Debug, Clone, thiserror::Error)]
pub enum AccountError {
    /// The server declined the sign-in and explained why.
    ///
    /// `code` is the RFC 6749 section 5.2 error code (`invalid_grant`,
    /// `authorization_pending`, ...). It is present only on a 400 answer, the
    /// one status at which the token endpoint defines it, and absent for a
    /// refusal decided locally.
    #[error("{message}")]
    Refused {
        code: Option<String>,
        message: String,
    },

    /// The server did not decide anything: rate limited (429), a server or
    /// gateway error (5xx), or a body that is not an OAuth error. Retryable,
    /// and never a reason to end a session.
    #[error("screenerbot.io could not answer (HTTP {status}): {detail}")]
    Unavailable { status: u16, detail: String },

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
