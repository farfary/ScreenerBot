// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Errors produced by the shared paper fill model and the live-arming gate.

use crate::errors::{ErrorClass, Severity};

/// Why a quote-based paper fill did not settle. Each caller maps it onto its own
/// persisted skip or error value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PaperFillError {
    /// The request is not an exact-input request of a positive size.
    #[error("a paper fill needs an exact-input request of a positive size")]
    InvalidSize,
    /// A sale was requested of an empty holding.
    #[error("a paper sale needs a positive holding")]
    EmptySell,
    /// The router returned no quote for the request.
    #[error("no router quote is available for the paper fill")]
    QuoteUnavailable,
    /// A quote changed identity between samples, lacks a network fee estimate or
    /// produces a non-positive figure.
    #[error("the router quotes cannot settle a paper fill")]
    QuoteUnfillable,
    /// The landing quote's output is below the initial quote's minimum output.
    #[error("the landing quote is below the initial minimum output")]
    MinimumOutputNotMet,
}

impl ErrorClass for PaperFillError {
    fn is_retryable(&self) -> bool {
        match self {
            PaperFillError::InvalidSize | PaperFillError::EmptySell => false,
            PaperFillError::QuoteUnavailable
            | PaperFillError::QuoteUnfillable
            | PaperFillError::MinimumOutputNotMet => true,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            PaperFillError::InvalidSize | PaperFillError::EmptySell => Severity::Warning,
            PaperFillError::QuoteUnavailable
            | PaperFillError::QuoteUnfillable
            | PaperFillError::MinimumOutputNotMet => Severity::Info,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            PaperFillError::InvalidSize | PaperFillError::EmptySell => 400,
            PaperFillError::QuoteUnavailable
            | PaperFillError::QuoteUnfillable
            | PaperFillError::MinimumOutputNotMet => 503,
        }
    }
}

/// Why a requested trading-mode transition was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PaperModeError {
    /// Arming live needs the exact confirmation phrase.
    #[error("arming live trading needs the exact confirmation phrase")]
    LiveConfirmationRequired,
}

impl ErrorClass for PaperModeError {
    fn is_retryable(&self) -> bool {
        match self {
            PaperModeError::LiveConfirmationRequired => false,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            PaperModeError::LiveConfirmationRequired => Severity::Info,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            PaperModeError::LiveConfirmationRequired => 400,
        }
    }
}
