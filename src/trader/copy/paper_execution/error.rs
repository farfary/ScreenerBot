// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Queue admission failures for paper execution.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueError {
    Full,
    Duplicate,
    Unavailable,
}
