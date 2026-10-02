// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The watch loop's runtime-failure vocabulary: the last problem the observation loop hit for
//! a target, shown in its status.

use crate::i18n::{ids, UiText};

/// The last problem the observation loop hit for a target, shown in its status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchRuntimeError {
    ProviderUnavailable,
    ProviderRepeatedFailure,
    ProcessingRepeatedFailure,
    PositionUnreadable,
    ProviderCheckFailed,
    DecodeFailed,
    ProcessingFailed,
    PositionSaveFailed,
}

impl WatchRuntimeError {
    pub fn ui_text(self) -> UiText {
        UiText::new(match self {
            Self::ProviderUnavailable => ids::WALLETS_WATCH_ERROR_PROVIDER_UNAVAILABLE,
            Self::ProviderRepeatedFailure => ids::WALLETS_WATCH_ERROR_PROVIDER_REPEATED_FAILURE,
            Self::ProcessingRepeatedFailure => ids::WALLETS_WATCH_ERROR_PROCESSING_REPEATED_FAILURE,
            Self::PositionUnreadable => ids::WALLETS_WATCH_ERROR_POSITION_UNREADABLE,
            Self::ProviderCheckFailed => ids::WALLETS_WATCH_ERROR_PROVIDER_CHECK_FAILED,
            Self::DecodeFailed => ids::WALLETS_WATCH_ERROR_DECODE_FAILED,
            Self::ProcessingFailed => ids::WALLETS_WATCH_ERROR_PROCESSING_FAILED,
            Self::PositionSaveFailed => ids::WALLETS_WATCH_ERROR_POSITION_SAVE_FAILED,
        })
    }
}
