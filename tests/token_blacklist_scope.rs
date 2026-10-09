// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The token blacklist refuses new entries and nothing else: an owned token keeps its pool.

mod common;

use std::collections::HashSet;
use std::sync::Arc;

use screenerbot::chains::solana::pools::discovery::skips_blacklisted_token;
use screenerbot::chains::ChainId;
use screenerbot::trader::admission::{check_entry_admission, EntryBlock};

const BLACKLISTED: &str = "BlacklistedMint11111111111111111111111111111";
const CLEAN: &str = "CleanMint1111111111111111111111111111111111";

/// A blacklisted mint keeps pool discovery while it is a position, held or paper-held token
/// (the owned set), loses it when nobody owns it, and is refused by entry admission either way.
/// The database is a fresh file installed as the Solana token database, so the admission gate
/// reads the same blacklist row discovery does.
#[tokio::test]
async fn a_blacklisted_token_keeps_its_pool_while_owned_and_never_passes_entry() {
    let _cfg = common::config_guard();
    let dir = common::isolated_env();
    screenerbot::global::set_force_stopped(false, None);

    let db = Arc::new(
        screenerbot::tokens::TokenDatabase::new(
            &dir.path().join("tokens.db").to_string_lossy(),
            ChainId::Solana,
        )
        .expect("open tokens database"),
    );
    db.add_to_blacklist(BLACKLISTED, "Mint authority present", "auto_cleanup")
        .expect("blacklist the mint");
    screenerbot::tokens::install_database(db.clone());

    let nobody_owns: HashSet<String> = HashSet::new();
    let owned: HashSet<String> = [BLACKLISTED.to_owned()].into_iter().collect();

    assert!(
        skips_blacklisted_token(&db, BLACKLISTED, &nobody_owns),
        "an unowned blacklisted token gets no pool"
    );
    assert!(
        !skips_blacklisted_token(&db, BLACKLISTED, &owned),
        "an owned blacklisted token keeps its pool price"
    );
    assert!(!skips_blacklisted_token(&db, CLEAN, &nobody_owns));

    assert_eq!(
        check_entry_admission(BLACKLISTED, &[]).await,
        Err(EntryBlock::Blacklisted),
        "owning a blacklisted token must not open the entry gate"
    );
    assert_eq!(check_entry_admission(CLEAN, &[]).await, Ok(()));

    screenerbot::tokens::uninstall_databases();
}
