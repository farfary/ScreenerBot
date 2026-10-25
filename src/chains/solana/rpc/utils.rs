// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! RPC utility functions
//!
//! Common utilities for RPC operations.

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Parse a pubkey from string safely
///
/// Wrapper around `Pubkey::from_str` with better error messages.
pub fn parse_pubkey_string(s: &str) -> crate::chains::solana::Result<Pubkey> {
    Pubkey::from_str(s).map_err(|_| crate::chains::solana::Error::InvalidAddress {
        kind: "pubkey",
        value: s.to_owned(),
    })
}
