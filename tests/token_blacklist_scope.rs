// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The token blacklist refuses new entries and nothing else: an owned token keeps its pool, and
//! no automatic rule blacklists the chain's native or stablecoin quote assets.

use crate::common;

use std::collections::HashSet;
use std::sync::Arc;

use screenerbot::chains::solana::constants::{USDC_MINT, USDT_MINT};
use screenerbot::chains::solana::pools::discovery::skips_blacklisted_token;
use screenerbot::chains::ChainId;
use screenerbot::tokens::cleanup::{evaluate_blacklist, lift_automatic_quote_asset_entries};
use screenerbot::tokens::types::RugcheckData;
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

fn with_authorities(mint_authority: &str) -> RugcheckData {
    let now = chrono::Utc::now();
    RugcheckData {
        token_type: None,
        token_decimals: Some(6),
        score: None,
        score_normalised: None,
        score_description: None,
        mint_authority: Some(mint_authority.to_owned()),
        freeze_authority: Some(mint_authority.to_owned()),
        update_authority: None,
        creator: None,
        is_mutable: None,
        top_10_holders_pct: None,
        total_holders: None,
        total_lp_providers: None,
        graph_insiders_detected: None,
        total_market_liquidity: None,
        total_stable_liquidity: None,
        total_supply: None,
        creator_balance_pct: None,
        transfer_fee_pct: None,
        transfer_fee_max_amount: None,
        transfer_fee_authority: None,
        rugged: false,
        risks: vec![],
        top_holders: vec![],
        markets: None,
        security_data_last_fetched_at: now,
        security_data_first_fetched_at: now,
    }
}

/// A stablecoin issuer holds mint and freeze authority by design. The authority rule blacklists
/// an ordinary token that keeps them, never a quote asset; the cleanup start lifts an automatic
/// entry an earlier build filed on one and keeps an entry the owner made by hand.
#[tokio::test]
async fn the_authority_rule_never_blacklists_a_quote_asset() {
    let dir = common::isolated_env();
    let db = screenerbot::tokens::TokenDatabase::new(
        &dir.path().join("tokens.db").to_string_lossy(),
        ChainId::Solana,
    )
    .expect("open tokens database");
    let issuer = "BJE5MMbqXjVwjAF7oxwPYXnTXDyspzZyt4vwenNw5ruG";
    for mint in [USDC_MINT, CLEAN] {
        db.upsert_token(mint, None, None, Some(6))
            .expect("store the token");
        db.upsert_rugcheck_data(mint, &with_authorities(issuer))
            .expect("store rugcheck data");
    }

    assert_eq!(
        evaluate_blacklist(USDC_MINT, &db).await.expect("evaluate"),
        None
    );
    assert_eq!(
        evaluate_blacklist(CLEAN, &db).await.expect("evaluate"),
        Some(format!("Mint authority present: {issuer}"))
    );

    db.add_to_blacklist(USDC_MINT, "Mint authority present", "auto_cleanup")
        .expect("file the earlier automatic entry");
    db.add_to_blacklist(USDT_MINT, "Owner choice", "manual")
        .expect("file a manual entry");
    lift_automatic_quote_asset_entries(&db).expect("lift the automatic entries");

    assert!(
        !db.is_blacklisted(USDC_MINT).expect("read"),
        "the automatic entry is lifted"
    );
    assert!(
        db.is_blacklisted(USDT_MINT).expect("read"),
        "the manual entry stays"
    );
}
