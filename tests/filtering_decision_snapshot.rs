// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Recorded Solana filtering decisions.
//!
//! A fixed set of normalized tokens is evaluated under three filter configurations and the
//! outcome of every `(config, token)` pair is compared byte-for-byte with a recorded file.
//! Any change to a rule, a rejection code, a source label or the evaluation order shows up
//! as a differing line.
//!
//! # Inputs (`tests/fixtures/filtering/`)
//!
//! * `solana-token-set.jsonl` — one `{"age_seconds", "token"}` object per line.
//! * `solana-blocked-authorities.json` — the blocked-authority set seeded before evaluation.
//! * `solana-filter-configs.json` — the three filter configurations.
//! * `solana-decisions.json` — the recorded outcomes.
//!
//! Every address in the fixtures is replaced by `bs58(sha256("filtering-fixture:" + address))`;
//! the native asset addresses are kept. Only fields a filter reads are populated.
//!
//! # Offline guarantee
//!
//! Decimals come from the tokens themselves (asserted), no token database is installed, the
//! LLM stage is off, the cooldown reads the empty in-process position list and the authority
//! check reads the seeded in-memory set.
//!
//! # Recording
//!
//! The ignored recorder clones the owner's real database into a temp dir, so the live files
//! are never touched. It writes only when `SCREENERBOT_RECORD_FILTERING_FIXTURE=1`:
//!
//! ```text
//! SCREENERBOT_RECORD_FILTERING_FIXTURE=1 cargo test --test filtering_decision_snapshot \
//!     -- --ignored record_solana_decision_fixture --nocapture
//! ```
#![allow(clippy::await_holding_lock)]

mod common;

use chrono::{DateTime, Duration, TimeZone, Utc};
use screenerbot::chains::ChainId;
use screenerbot::config::FilteringConfig;
use screenerbot::filtering::evaluate_token;
use screenerbot::tokens::authority_cache::refresh_blocked_from_db;
use screenerbot::tokens::types::{SecurityRisk, Token, TokenHolder};
use screenerbot::tokens::Priority;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const CONFIG_ORDER: [&str; 4] = ["core_only", "default", "dex_only", "strict"];
const PER_GROUP: usize = 8;
const SCAM_AUTHORITY_CAP: usize = 16;
const TOTAL_CAP: usize = 300;
const SCAM_AUTHORITY_CODE: &str = "onchain_known_scam_authority";
const PASS: &str = "pass";

/// One fixture line: a token and how long before the evaluation it was first discovered.
#[derive(Serialize, Deserialize)]
struct Line {
    age_seconds: i64,
    token: Token,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("filtering")
}

fn pretty<T: Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("serialize fixture");
    text.push('\n');
    text
}

fn turn_llm_off() {
    common::set_config(|cfg| {
        cfg.llm.enabled = false;
        cfg.llm_analysis.filtering_enabled = false;
    });
}

/// `bs58(sha256("filtering-fixture:" + address))`; native asset addresses stay as they are.
fn pseudonymize(address: &str) -> String {
    if screenerbot::chains::adapter().is_native_asset(address) {
        return address.to_owned();
    }
    let digest = Sha256::digest(format!("filtering-fixture:{address}").as_bytes());
    bs58::encode(digest).into_string()
}

fn parse_lines(text: &str) -> Vec<Line> {
    text.lines()
        .map(|line| serde_json::from_str(line).expect("parse token set line"))
        .collect()
}

/// The token as evaluated: discovery time set `age_seconds` before now, decimals guaranteed.
fn aged(line: &Line) -> Token {
    let mut token = line.token.clone();
    token.first_discovered_at = Utc::now() - Duration::seconds(line.age_seconds);
    assert!(
        screenerbot::chains::adapter().is_native_asset(&token.mint)
            || token
                .decimals
                .is_some_and(screenerbot::tokens::decimals_are_valid),
        "fixture token {} would reach the decimals network fallback",
        token.mint
    );
    token
}

/// Outcome of one evaluation: the string `"pass"` or `{"reason", "source"}`.
async fn outcome(token: &Token, config: &FilteringConfig) -> serde_json::Value {
    match evaluate_token(&common::solana_filter_profile(), token, config).await {
        Ok(()) => serde_json::Value::String(PASS.to_owned()),
        Err(reason) => serde_json::json!({
            "reason": reason.label(),
            "source": reason.source().as_str(),
        }),
    }
}

fn outcome_code(value: &serde_json::Value) -> String {
    match value.as_str() {
        Some(code) => code.to_owned(),
        None => value["reason"].as_str().expect("reason").to_owned(),
    }
}

/// Evaluate every line under every config in fixed order; returns the serialized decisions.
async fn decisions_text(
    lines: &[Line],
    configs: &BTreeMap<String, FilteringConfig>,
) -> (
    String,
    BTreeMap<String, BTreeMap<String, serde_json::Value>>,
) {
    let tokens: Vec<Token> = lines.iter().map(aged).collect();
    let mut decisions = BTreeMap::new();
    for name in CONFIG_ORDER {
        let config = configs.get(name).expect("config present");
        let mut per_mint = BTreeMap::new();
        for token in &tokens {
            per_mint.insert(token.mint.clone(), outcome(token, config).await);
        }
        decisions.insert(name.to_owned(), per_mint);
    }
    (pretty(&decisions), decisions)
}

fn load_configs(text: &str) -> BTreeMap<String, FilteringConfig> {
    serde_json::from_str(text).expect("parse filter configs")
}

fn read_fixture(name: &str) -> String {
    let path = fixture_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn filter_configs() -> BTreeMap<String, FilteringConfig> {
    let strict = {
        let mut config = FilteringConfig::default();
        config.dexscreener.enabled = true;
        config.geckoterminal.enabled = true;
        config.rugcheck.enabled = true;
        config.onchain.enabled = true;
        config.onchain.combined_risk_enabled = true;
        config.age_enabled = true;
        config.cooldown_enabled = true;
        // Sub-checks that default to off, so this profile decides differently from `default`.
        config.dexscreener.fdv_enabled = true;
        config.dexscreener.volume_enabled = true;
        config.dexscreener.price_change_enabled = true;
        config.geckoterminal.market_cap_enabled = true;
        config.geckoterminal.volume_enabled = true;
        config.geckoterminal.price_change_enabled = true;
        config.geckoterminal.pool_metrics_enabled = true;
        config
    };
    // Every provider source off: only the core checks decide.
    let core_only = {
        let mut config = FilteringConfig::default();
        config.dexscreener.enabled = false;
        config.geckoterminal.enabled = false;
        config.rugcheck.enabled = false;
        config.onchain.enabled = false;
        config
    };
    BTreeMap::from([
        ("core_only".to_owned(), core_only),
        ("default".to_owned(), FilteringConfig::default()),
        ("dex_only".to_owned(), common::filters_default_dex_only()),
        ("strict".to_owned(), strict),
    ])
}

// ============================================================================
// SNAPSHOT
// ============================================================================

#[tokio::test]
async fn solana_decisions_match_the_recorded_snapshot() {
    let dir = tempfile::tempdir().expect("create temp data dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    common::ensure_config();
    turn_llm_off();

    let blocked: Vec<String> =
        serde_json::from_str(&read_fixture("solana-blocked-authorities.json"))
            .expect("parse blocked authorities");
    refresh_blocked_from_db(ChainId::Solana, blocked);

    let lines = parse_lines(&read_fixture("solana-token-set.jsonl"));
    let configs = load_configs(&read_fixture("solana-filter-configs.json"));
    let (actual_text, actual) = decisions_text(&lines, &configs).await;

    let expected_text = read_fixture("solana-decisions.json");
    if actual_text != expected_text {
        let expected: BTreeMap<String, BTreeMap<String, serde_json::Value>> =
            serde_json::from_str(&expected_text).expect("parse recorded decisions");
        for (config, per_mint) in &actual {
            for (mint, got) in per_mint {
                let want = expected.get(config).and_then(|m| m.get(mint));
                if want != Some(got) {
                    eprintln!("DIFF config={config} mint={mint} expected={want:?} actual={got}");
                }
            }
        }
        panic!("filtering decisions differ from the recorded snapshot");
    }
}

// ============================================================================
// RECORDER
// ============================================================================

/// Keeps only what a filter reads; everything else is reset. Addresses are pseudonymized and
/// every original address seen is added to `originals`.
fn normalize(source: &Token, originals: &mut BTreeSet<String>) -> Token {
    let epoch: DateTime<Utc> = Utc.timestamp_opt(1_700_000_000, 0).single().expect("epoch");
    let mut address = |value: &str| {
        originals.insert(value.to_owned());
        pseudonymize(value)
    };

    let mint = address(&source.mint);
    let mint_authority = source.mint_authority.as_deref().map(&mut address);
    let freeze_authority = source.freeze_authority.as_deref().map(&mut address);
    let update_authority = source.update_authority.as_deref().map(&mut address);
    let transfer_fee_authority = source.transfer_fee_authority.as_deref().map(&mut address);
    // Dropped fields that carry addresses are still recorded for the leak proof.
    for dropped in [
        source.pool_price_last_used_pool.as_deref(),
        source.top_holders.iter().find_map(|h| h.owner.as_deref()),
    ]
    .into_iter()
    .flatten()
    {
        address(dropped);
    }
    let top_holders = source
        .top_holders
        .iter()
        .map(|holder| {
            if let Some(owner) = holder.owner.as_deref() {
                address(owner);
            }
            TokenHolder {
                address: address(&holder.address),
                amount: holder.amount.clone(),
                pct: holder.pct,
                owner: None,
                insider: holder.insider,
            }
        })
        .collect();

    Token {
        mint,
        symbol: source.symbol.clone(),
        name: source.name.clone(),
        decimals: source.decimals,
        description: None,
        image_url: None,
        header_image_url: None,
        supply: None,
        data_source: source.data_source,
        first_discovered_at: epoch,
        blockchain_created_at: None,
        metadata_last_fetched_at: epoch,
        decimals_last_fetched_at: epoch,
        market_data_last_fetched_at: epoch,
        security_data_last_fetched_at: None,
        pool_price_last_calculated_at: epoch,
        pool_price_last_used_pool: None,
        price_usd: 0.0,
        price_sol: 0.0,
        price_native: String::new(),
        price_change_m5: source.price_change_m5,
        price_change_h1: source.price_change_h1,
        price_change_h6: source.price_change_h6,
        price_change_h24: source.price_change_h24,
        market_cap: source.market_cap,
        fdv: source.fdv,
        liquidity_usd: source.liquidity_usd,
        volume_m5: source.volume_m5,
        volume_h1: source.volume_h1,
        volume_h6: source.volume_h6,
        volume_h24: source.volume_h24,
        pool_count: source.pool_count,
        reserve_in_usd: source.reserve_in_usd,
        txns_m5_buys: source.txns_m5_buys,
        txns_m5_sells: source.txns_m5_sells,
        txns_h1_buys: source.txns_h1_buys,
        txns_h1_sells: source.txns_h1_sells,
        txns_h6_buys: source.txns_h6_buys,
        txns_h6_sells: source.txns_h6_sells,
        txns_h24_buys: source.txns_h24_buys,
        txns_h24_sells: source.txns_h24_sells,
        websites: Vec::new(),
        socials: Vec::new(),
        mint_authority,
        freeze_authority,
        update_authority,
        is_mutable: source.is_mutable,
        security_score: source.security_score,
        security_score_normalised: None,
        is_rugged: source.is_rugged,
        token_type: source.token_type.clone(),
        graph_insiders_detected: source.graph_insiders_detected,
        lp_provider_count: source.lp_provider_count,
        security_risks: source
            .security_risks
            .iter()
            .map(|risk| SecurityRisk {
                name: risk.name.clone(),
                value: risk.value.clone(),
                description: risk.description.clone(),
                score: risk.score,
                level: risk.level.clone(),
            })
            .collect(),
        total_holders: source.total_holders,
        top_holders,
        creator_balance_pct: source.creator_balance_pct,
        top_10_holders_pct: None,
        transfer_fee_pct: source.transfer_fee_pct,
        transfer_fee_max_amount: source.transfer_fee_max_amount,
        transfer_fee_authority,
        is_blacklisted: source.is_blacklisted,
        priority: Priority::Standard,
        last_rejection_reason: None,
        last_rejection_source: None,
        last_rejection_at: None,
    }
}

fn authorities_of(token: &Token) -> impl Iterator<Item = &str> {
    [
        token.mint_authority.as_deref(),
        token.freeze_authority.as_deref(),
        token.update_authority.as_deref(),
    ]
    .into_iter()
    .flatten()
}

#[tokio::test]
#[ignore = "records the filtering decision fixture from a cloned real database"]
async fn record_solana_decision_fixture() {
    if std::env::var("SCREENERBOT_RECORD_FILTERING_FIXTURE").as_deref() != Ok("1") {
        eprintln!("SKIP recorder: SCREENERBOT_RECORD_FILTERING_FIXTURE is not 1");
        return;
    }
    let Some(_dir) = common::real_db_env() else {
        return;
    };
    let db = common::init_real_token_db();
    turn_llm_off();

    let recorded_at = Utc::now();
    let source_tokens = screenerbot::tokens::get_all_tokens_for_filtering_async(db.chain())
        .await
        .expect("load tokens for filtering");
    let adapter = screenerbot::chains::adapter();

    // Normalize, pseudonymize and compute ages. Tokens that would reach the decimals
    // network fallback are left out.
    let mut originals = BTreeSet::new();
    let mut lines: Vec<Line> = Vec::new();
    for source in &source_tokens {
        let decimals_ok = adapter.is_native_asset(&source.mint)
            || source
                .decimals
                .is_some_and(screenerbot::tokens::decimals_are_valid);
        if !decimals_ok {
            continue;
        }
        let age_minutes = (recorded_at - source.first_discovered_at).num_minutes();
        lines.push(Line {
            age_seconds: age_minutes * 60 + 30,
            token: normalize(source, &mut originals),
        });
    }
    eprintln!(
        "recorder: {} of {} tokens kept",
        lines.len(),
        source_tokens.len()
    );

    // Blocked authorities: seed the full pseudonymized set while selecting.
    let real_blocked = db
        .load_blocked_authorities()
        .expect("load blocked authorities");
    originals.extend(real_blocked.iter().cloned());
    let blocked_all: Vec<String> = real_blocked.iter().map(|a| pseudonymize(a)).collect();
    refresh_blocked_from_db(ChainId::Solana, blocked_all);

    let configs = filter_configs();

    // Evaluate everything and group by (config, outcome code).
    let mut groups: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    let mut scam: BTreeSet<String> = BTreeSet::new();
    for line in &lines {
        let token = aged(line);
        for name in CONFIG_ORDER {
            let value = outcome(&token, &configs[name]).await;
            let code = outcome_code(&value);
            if code == SCAM_AUTHORITY_CODE {
                scam.insert(token.mint.clone());
            }
            groups
                .entry((name.to_owned(), code))
                .or_default()
                .push(token.mint.clone());
        }
    }

    // Selection: scam-authority rejections first, then the rarest groups, up to the cap.
    let mut selected: BTreeSet<String> = BTreeSet::new();
    selected.extend(scam.iter().take(SCAM_AUTHORITY_CAP).cloned());
    let mut ordered: Vec<(&(String, String), &Vec<String>)> = groups.iter().collect();
    ordered.sort_by(|a, b| a.1.len().cmp(&b.1.len()).then_with(|| a.0.cmp(b.0)));
    for (_, mints) in ordered {
        let mut sorted = mints.clone();
        sorted.sort();
        for mint in sorted.into_iter().take(PER_GROUP) {
            if selected.len() >= TOTAL_CAP {
                break;
            }
            selected.insert(mint);
        }
    }

    let mut chosen: Vec<Line> = lines
        .into_iter()
        .filter(|line| selected.contains(&line.token.mint))
        .collect();
    chosen.sort_by(|a, b| a.token.mint.cmp(&b.token.mint));

    let present: BTreeSet<&str> = chosen
        .iter()
        .flat_map(|line| authorities_of(&line.token))
        .collect();
    let blocked_selected: Vec<String> = real_blocked
        .iter()
        .map(|a| pseudonymize(a))
        .filter(|a| present.contains(a.as_str()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    // Round-trip through the fixture text, then evaluate exactly as the snapshot test does.
    let mut jsonl = String::new();
    for line in &chosen {
        jsonl.push_str(&serde_json::to_string(line).expect("serialize line"));
        jsonl.push('\n');
    }
    let reparsed = parse_lines(&jsonl);
    refresh_blocked_from_db(ChainId::Solana, blocked_selected.clone());
    let configs_text = pretty(&configs);
    let configs = load_configs(&configs_text);
    let (decisions_out, decisions) = decisions_text(&reparsed, &configs).await;
    let blocked_text = pretty(&blocked_selected);

    // Leak proof: no original address appears in any written file.
    let files = [
        ("solana-token-set.jsonl", &jsonl),
        ("solana-blocked-authorities.json", &blocked_text),
        ("solana-filter-configs.json", &configs_text),
        ("solana-decisions.json", &decisions_out),
    ];
    let mut checked = 0usize;
    for original in originals.iter().filter(|a| !adapter.is_native_asset(a)) {
        checked += 1;
        for (name, text) in &files {
            assert!(
                !text.contains(original.as_str()),
                "original address survives in {name}"
            );
        }
    }
    eprintln!("recorder: leak proof checked {checked} original addresses");

    let dir = fixture_dir();
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    for (name, text) in &files {
        std::fs::write(dir.join(name), text).expect("write fixture");
        eprintln!("recorder: wrote {name} ({} bytes)", text.len());
    }

    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (config, per_mint) in &decisions {
        for value in per_mint.values() {
            *counts
                .entry((config.clone(), outcome_code(value)))
                .or_default() += 1;
        }
    }
    eprintln!("recorder: {} tokens selected", reparsed.len());
    for ((config, code), count) in counts {
        eprintln!("recorder: {config:<9} {code:<40} {count}");
    }
}
