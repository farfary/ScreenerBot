// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The blocked-authority set never blocks the address a renounced authority holds.

use screenerbot::chains::solana::constants::SYSTEM_PROGRAM_ID;
use screenerbot::chains::ChainId;
use screenerbot::tokens::authority_cache::{is_blocked_authority, refresh_blocked_from_db};

const BLOCKED: &str = "BlockedAuthority1111111111111111111111111111";

/// The System Program is the update, freeze or mint authority of every token whose authority
/// was given up; a set (or a stored manual row) that names it must not reject those tokens,
/// while any other blocked address stays blocked.
#[test]
fn the_renounced_authority_is_never_blocked() {
    refresh_blocked_from_db(
        ChainId::Solana,
        vec![SYSTEM_PROGRAM_ID.to_owned(), BLOCKED.to_owned()],
    );

    assert!(!is_blocked_authority(ChainId::Solana, SYSTEM_PROGRAM_ID));
    assert!(is_blocked_authority(ChainId::Solana, BLOCKED));
}
