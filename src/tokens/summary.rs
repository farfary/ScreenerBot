use std::collections::{HashMap, HashSet};

use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};

use crate::{
    logger::{log, LogTag},
    pools, positions,
    tokens::{blacklist, security_db::SecurityDatabase, types::Token},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSummary {
    pub mint: String,
    pub symbol: String,
    pub name: Option<String>,
    pub logo_url: Option<String>,
    pub price_sol: Option<f64>,
    pub price_updated_at: Option<i64>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,
    pub price_change_h1: Option<f64>,
    pub price_change_h24: Option<f64>,
    pub security_score: Option<i32>,
    pub rugged: Option<bool>,
    pub total_holders: Option<i32>,
    pub has_pool_price: bool,
    pub has_ohlcv: bool,
    pub has_open_position: bool,
    pub blacklisted: bool,
}

#[derive(Debug, Default)]
pub struct TokenSummaryContext {
    security: HashMap<String, SecuritySnapshot>,
    ohlcv: HashSet<String>,
    open_positions: HashSet<String>,
    blacklisted: HashSet<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct SecuritySnapshot {
    pub score: i32,
    pub rugged: bool,
    pub total_holders: Option<i32>,
}

impl TokenSummaryContext {
    pub async fn build(mints: &[String]) -> Self {
        let unique_mints: HashSet<String> = mints.iter().cloned().collect();

        let open_positions = positions::get_open_positions()
            .await
            .into_iter()
            .map(|pos| pos.mint)
            .collect::<HashSet<_>>();

        let blacklisted = blacklist::get_blacklisted_mints()
            .into_iter()
            .collect::<HashSet<_>>();

        let security = load_security_snapshots(&unique_mints);
        let ohlcv = load_ohlcv_flags(&unique_mints).await;

        Self {
            security,
            ohlcv,
            open_positions,
            blacklisted,
        }
    }

    pub fn security_snapshot(&self, mint: &str) -> Option<&SecuritySnapshot> {
        self.security.get(mint)
    }

    pub fn has_ohlcv(&self, mint: &str) -> bool {
        self.ohlcv.contains(mint)
    }

    pub fn has_open_position(&self, mint: &str) -> bool {
        self.open_positions.contains(mint)
    }

    pub fn is_blacklisted(&self, mint: &str) -> bool {
        self.blacklisted.contains(mint)
    }
}

pub fn token_to_summary(token: &Token, caches: &TokenSummaryContext) -> TokenSummary {
    let (price_sol, price_updated_at) =
        if let Some(price_result) = pools::get_pool_price(&token.mint) {
            let age_secs = price_result.timestamp.elapsed().as_secs();
            let now_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            (
                Some(price_result.price_sol),
                Some(now_unix - (age_secs as i64)),
            )
        } else {
            (None, None)
        };

    let has_pool_price = price_sol.is_some();
    let has_ohlcv = caches.has_ohlcv(&token.mint);
    let has_open_position = caches.has_open_position(&token.mint);
    let blacklisted = caches.is_blacklisted(&token.mint);

    let (security_score, rugged, total_holders) = caches
        .security_snapshot(&token.mint)
        .map(|snapshot| {
            (
                Some(snapshot.score),
                Some(snapshot.rugged),
                snapshot.total_holders,
            )
        })
        .unwrap_or((None, None, None));

    TokenSummary {
        mint: token.mint.clone(),
        symbol: token.symbol.clone(),
        name: Some(token.name.clone()),
        logo_url: token.logo_url.clone(),
        price_sol,
        price_updated_at,
        liquidity_usd: token.liquidity.as_ref().and_then(|l| l.usd),
        volume_24h: token.volume.as_ref().and_then(|v| v.h24),
        fdv: token.fdv,
        market_cap: token.market_cap,
        price_change_h1: token.price_change.as_ref().and_then(|p| p.h1),
        price_change_h24: token.price_change.as_ref().and_then(|p| p.h24),
        security_score,
        rugged,
        total_holders,
        has_pool_price,
        has_ohlcv,
        has_open_position,
        blacklisted,
    }
}

pub async fn summarize_tokens(tokens: &[Token]) -> Vec<TokenSummary> {
    if tokens.is_empty() {
        return Vec::new();
    }

    let mints: Vec<String> = tokens.iter().map(|token| token.mint.clone()).collect();
    let caches = TokenSummaryContext::build(&mints).await;

    tokens
        .iter()
        .map(|token| token_to_summary(token, &caches))
        .collect()
}

fn load_security_snapshots(mints: &HashSet<String>) -> HashMap<String, SecuritySnapshot> {
    let mut snapshots = HashMap::new();

    if mints.is_empty() {
        return snapshots;
    }

    if let Ok(db) = SecurityDatabase::new("data/security.db") {
        for mint in mints {
            if let Ok(Some(sec)) = db.get_security_info(mint) {
                snapshots.insert(
                    mint.clone(),
                    SecuritySnapshot {
                        score: sec.score,
                        rugged: sec.rugged,
                        total_holders: if sec.total_holders > 0 {
                            Some(sec.total_holders)
                        } else {
                            None
                        },
                    },
                );
            }
        }
    }

    snapshots
}

async fn load_ohlcv_flags(mints: &HashSet<String>) -> HashSet<String> {
    if mints.is_empty() {
        return HashSet::new();
    }

    let mint_list: Vec<String> = mints.iter().cloned().collect();

    match crate::ohlcvs::get_mints_with_data(&mint_list).await {
        Ok(set) => set,
        Err(err) => {
            log(
                LogTag::Ohlcv,
                "BULK_HAS_DATA_FALLBACK",
                &format!("Failed bulk OHLCV presence check: {}", err),
            );
            fallback_load_ohlcv_flags(mints).await
        }
    }
}

async fn fallback_load_ohlcv_flags(mints: &HashSet<String>) -> HashSet<String> {
    stream::iter(mints.iter().cloned())
        .map(|mint| async move {
            let has_data = crate::ohlcvs::has_data(&mint).await.unwrap_or(false);
            (mint, has_data)
        })
        .buffer_unordered(8)
        .filter_map(|(mint, has_data)| async move {
            if has_data {
                Some(mint)
            } else {
                None
            }
        })
        .collect::<HashSet<_>>()
        .await
}
