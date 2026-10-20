// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Creator reputation from the data service: wallets reported as serial liquidity removers
//! become blocked addresses.
//!
//! The service owns the judgement; this module only validates its list, stores it as the
//! `creator_history` rows of `authority_reputation` and reloads the in-memory blocked set the
//! on-chain filter reads. Signed out, offline or against a service that has no list yet, nothing
//! is requested or replaced: the stored rows stay until they are older than [`EXPIRY_SECS`].

use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::chains::{adapter_for, ChainId};
use crate::data_server::{get_json, is_usable, Surface};
use crate::errors::InternalError;
use crate::logger::{self, LogTag};
use crate::tokens::authority_cache;
use crate::tokens::database::{CreatorReputationRow, TokenDatabase};
use crate::tokens::types::TokenResult;
use crate::tokens::Error;
use crate::utils::{check_shutdown_or_delay, run_or_shutdown};

/// Delay before the first pass after start.
const FIRST_PASS_DELAY_SECS: u64 = 30;
/// Time between passes.
const PASS_INTERVAL_SECS: u64 = 15 * 60;
/// A stored creator row that no answer has refreshed for this long is deleted.
const EXPIRY_SECS: i64 = 72 * 3_600;
/// Data-service path of the creator list.
const CREATORS_PATH: &str = "/v1/dna/creators";

/// The data service's creator list.
#[derive(Debug, Clone, Deserialize)]
pub struct CreatorsResponse {
    /// False until the service has computed a list since its start.
    pub ready: bool,
    pub creators: Vec<ServiceCreator>,
}

/// One wallet of the list: `rugged` of its `tokens` ended with the liquidity removed.
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceCreator {
    pub address: String,
    pub tokens: u32,
    pub rugged: u32,
}

/// The rows of `creators` that may be stored, and how many were refused.
///
/// A wallet is refused when it cannot be a personal wallet on `chain` (a program, a derived
/// address, the renounced-authority address or a malformed string), when it reports no tokens,
/// or when it reports more removals than tokens.
pub(super) fn accepted_rows(
    chain: ChainId,
    creators: &[ServiceCreator],
) -> (Vec<CreatorReputationRow>, usize) {
    let adapter = adapter_for(chain);
    let rows: Vec<CreatorReputationRow> = creators
        .iter()
        .filter(|creator| {
            creator.tokens > 0
                && creator.rugged <= creator.tokens
                && adapter.is_personal_wallet(&creator.address)
        })
        .map(|creator| CreatorReputationRow {
            address: creator.address.clone(),
            tokens: creator.tokens,
            rugged: creator.rugged,
        })
        .collect();
    let refused = creators.len() - rows.len();
    (rows, refused)
}

/// Apply one answer to the store and reload the chain's blocked set from it.
///
/// A ready list replaces the stored creator rows. No answer, or a list that is not ready, only
/// expires rows older than [`EXPIRY_SECS`]. Returns the number of blocked addresses afterwards.
pub fn apply_answer(
    db: &TokenDatabase,
    answer: Option<&CreatorsResponse>,
    now: i64,
) -> TokenResult<usize> {
    match answer {
        Some(list) if list.ready => {
            let (rows, refused) = accepted_rows(db.chain(), &list.creators);
            if refused > 0 {
                logger::warning(
                    LogTag::Tokens,
                    &format!("[CREATORS] Refused {refused} invalid entries of the creator list"),
                );
            }
            db.replace_creator_reputation(&rows, now)?;
        }
        _ => {
            db.expire_creator_reputation(now - EXPIRY_SECS)?;
        }
    }
    let blocked = db.load_blocked_authorities()?;
    let count = blocked.len();
    authority_cache::refresh_blocked_from_db(db.chain(), blocked);
    Ok(count)
}

/// One pass: ask the service for the list when it is usable, then apply the answer.
async fn sync_pass(db: &Arc<TokenDatabase>) -> TokenResult<usize> {
    let chain = db.chain();
    let answer = if is_usable(Surface::Tokens) {
        get_json::<CreatorsResponse>(Surface::Tokens, chain, CREATORS_PATH, &[]).await
    } else {
        None
    };
    let now = chrono::Utc::now().timestamp();
    let db = db.clone();
    tokio::task::spawn_blocking(move || apply_answer(&db, answer.as_ref(), now))
        .await
        .map_err(|error| Error::Internal(InternalError::from(error)))?
}

/// Keep the database's chain's creator list current from the data service until shutdown.
pub fn start_sync_loop(db: Arc<TokenDatabase>, shutdown: Arc<Notify>) -> JoinHandle<()> {
    tokio::spawn(async move {
        if !adapter_for(db.chain()).has_creator_reputation() {
            return;
        }
        let mut delay = FIRST_PASS_DELAY_SECS;
        loop {
            if check_shutdown_or_delay(&shutdown, Duration::from_secs(delay)).await {
                break;
            }
            delay = PASS_INTERVAL_SECS;
            match run_or_shutdown(&shutdown, sync_pass(&db)).await {
                None => break,
                Some(Ok(_)) => {}
                Some(Err(error)) => logger::warning(
                    LogTag::Tokens,
                    &format!("[CREATORS] Creator list sync failed: {error}"),
                ),
            }
        }
    })
}
