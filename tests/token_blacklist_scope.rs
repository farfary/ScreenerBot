// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The token blacklist refuses new entries and nothing else: an owned token keeps its pool, no
//! automatic rule blacklists the chain's native or stablecoin quote assets, an automatic entry
//! lifts when its cause ends, and a token the user removed stays removed.

use crate::common;

use std::collections::HashSet;
use std::sync::Arc;

use screenerbot::chains::solana::constants::{USDC_MINT, USDT_MINT};
use screenerbot::chains::solana::pools::discovery::skips_blacklisted_token;
use screenerbot::chains::ChainId;
use screenerbot::tokens::cleanup::{
    blacklist_token, evaluate_blacklist, lift_automatic_quote_asset_entries,
    lift_renounced_authority_entries, lift_stale_route_entries, remove_blacklisted_token,
};
use screenerbot::tokens::database::ROUTE_ENTRY_LIFETIME_SECS;
use screenerbot::tokens::types::RugcheckData;
use screenerbot::tokens::{BlacklistSource, BlacklistWrite};
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

fn open_store(dir: &tempfile::TempDir) -> screenerbot::tokens::TokenDatabase {
    screenerbot::tokens::TokenDatabase::new(
        &dir.path().join("tokens.db").to_string_lossy(),
        ChainId::Solana,
    )
    .expect("open tokens database")
}

fn store_report(db: &screenerbot::tokens::TokenDatabase, mint: &str, report: &RugcheckData) {
    db.upsert_token(mint, None, None, Some(6))
        .expect("store the token");
    db.upsert_rugcheck_data(mint, report)
        .expect("store rugcheck data");
}

fn renounced() -> RugcheckData {
    let mut report = with_authorities("unused");
    report.mint_authority = None;
    report.freeze_authority = None;
    report
}

/// A no-route entry lifts once it is a full lifetime old, so the next quote retests the route.
/// Younger route entries, and every entry from another source, stay however old they are.
#[test]
fn a_route_entry_lifts_after_its_lifetime_and_no_other_entry_does() {
    let dir = common::isolated_env();
    let db = open_store(&dir);
    let route = "RouteMint111111111111111111111111111111111111";
    let loss = "LossMint1111111111111111111111111111111111111";
    let llm = "LlmMint11111111111111111111111111111111111111";
    let manual = "ManualMint1111111111111111111111111111111111";
    for (mint, source) in [
        (route, BlacklistSource::AutoSwapRoute),
        (loss, BlacklistSource::AutoLoss),
        (llm, BlacklistSource::AutoLlmAnalysis),
        (manual, BlacklistSource::Manual),
    ] {
        db.add_to_blacklist(mint, "reason", source.code())
            .expect("file the entry");
    }
    let filed = db
        .get_blacklist_entry(route)
        .expect("read")
        .expect("route entry")
        .added_at;

    let almost = lift_stale_route_entries(&db, filed + ROUTE_ENTRY_LIFETIME_SECS - 1)
        .expect("lift before the lifetime ends");
    assert!(almost.is_empty());
    assert!(db.is_blacklisted(route).expect("read"));

    let lifted = lift_stale_route_entries(&db, filed + 30 * ROUTE_ENTRY_LIFETIME_SECS)
        .expect("lift after the lifetime ends");
    assert_eq!(lifted.len(), 1);
    assert_eq!(lifted[0].mint, route);
    assert!(!db.is_blacklisted(route).expect("read"));
    for kept in [loss, llm, manual] {
        assert!(db.is_blacklisted(kept).expect("read"), "{kept} stays");
    }

    assert!(
        !db.is_blacklist_cleared(route).expect("read"),
        "a lift is not a removal by the user"
    );
    assert_eq!(
        blacklist_token(
            route,
            "no route",
            BlacklistSource::AutoSwapRoute.code(),
            &db
        )
        .expect("file again"),
        BlacklistWrite::Written,
        "the swap path may file a lifted token again"
    );
}

/// An authority entry lifts when the stored Rugcheck report shows both authorities renounced.
/// A report that still shows one, a token with no report, and any other source stay.
#[test]
fn an_authority_entry_lifts_only_when_a_report_shows_it_renounced() {
    let dir = common::isolated_env();
    let db = open_store(&dir);
    let renounced_mint = "RenouncedMint11111111111111111111111111111111";
    let held_mint = "HeldAuthorityMint11111111111111111111111111";
    let unreported = "UnreportedMint1111111111111111111111111111111";
    let loss = "LossAuthorityMint111111111111111111111111111";

    store_report(&db, renounced_mint, &renounced());
    store_report(&db, held_mint, &with_authorities("Issuer1111"));
    store_report(&db, loss, &renounced());
    for mint in [renounced_mint, held_mint, unreported] {
        db.add_to_blacklist(mint, "Mint authority present", "auto_cleanup")
            .expect("file the entry");
    }
    db.add_to_blacklist(loss, "Closed at a loss", BlacklistSource::AutoLoss.code())
        .expect("file the entry");

    let lifted = lift_renounced_authority_entries(&db).expect("lift");
    assert_eq!(lifted.len(), 1);
    assert_eq!(lifted[0].mint, renounced_mint);
    assert!(!db.is_blacklisted(renounced_mint).expect("read"));
    assert!(db.is_blacklisted(held_mint).expect("read"));
    assert!(
        db.is_blacklisted(unreported).expect("read"),
        "a missing report is not a renounced authority"
    );
    assert!(
        db.is_blacklisted(loss).expect("read"),
        "a loss entry stays when the authorities are renounced"
    );
}

/// Removing a token through the cleanup owner deletes its entry, remembers the removal, and
/// opens the entry gate; no automatic source files it again, and a manual blacklist does.
#[tokio::test]
async fn a_token_the_user_removed_stays_removed_until_blacklisted_by_hand() {
    let _cfg = common::config_guard();
    let dir = common::isolated_env();
    screenerbot::global::set_force_stopped(false, None);

    let db = Arc::new(open_store(&dir));
    db.add_to_blacklist(BLACKLISTED, "Mint authority present", "auto_cleanup")
        .expect("file the entry");
    screenerbot::tokens::install_database(db.clone());
    assert_eq!(
        check_entry_admission(BLACKLISTED, &[]).await,
        Err(EntryBlock::Blacklisted)
    );

    let removed = remove_blacklisted_token(BLACKLISTED, &db)
        .await
        .expect("remove the entry")
        .expect("the entry that was removed");
    assert_eq!(removed.source, "auto_cleanup");
    assert_eq!(check_entry_admission(BLACKLISTED, &[]).await, Ok(()));

    for source in [
        BlacklistSource::AutoCleanup,
        BlacklistSource::AutoLoss,
        BlacklistSource::AutoLlmAnalysis,
        BlacklistSource::AutoSwapRoute,
    ] {
        assert_eq!(
            blacklist_token(BLACKLISTED, "again", source.code(), &db).expect("write"),
            BlacklistWrite::RefusedUserRemoved,
            "{}",
            source.code()
        );
    }
    assert_eq!(check_entry_admission(BLACKLISTED, &[]).await, Ok(()));

    assert_eq!(
        blacklist_token(BLACKLISTED, "by hand", BlacklistSource::Manual.code(), &db)
            .expect("write"),
        BlacklistWrite::Written
    );
    assert_eq!(
        check_entry_admission(BLACKLISTED, &[]).await,
        Err(EntryBlock::Blacklisted)
    );

    assert!(
        remove_blacklisted_token(CLEAN, &db)
            .await
            .expect("remove nothing")
            .is_none(),
        "a token with no entry has nothing to remove"
    );

    screenerbot::tokens::uninstall_databases();
}
