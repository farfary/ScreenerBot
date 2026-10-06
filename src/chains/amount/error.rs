// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Why a canonical raw amount could not be parsed.

/// A canonical decimal amount could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AmountParseError {
    /// The input contained no digits.
    #[error("amount cannot be empty")]
    Empty,
    /// The input was not a canonical unsigned decimal integer.
    #[error("amount must be a canonical unsigned decimal integer")]
    InvalidFormat,
    /// The input exceeds the range of `u128`.
    #[error("amount exceeds the u128 range")]
    Overflow,
}
