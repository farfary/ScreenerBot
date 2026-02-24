//! Token metadata for dashboard — resolves token names and symbols for display.

use futures::stream::{self, StreamExt};
use std::collections::{HashMap, HashSet};

use crate::logger::{self, LogTag};

use super::{clamp_token_limit, short_mint_label, TOKEN_METADATA_CONCURRENCY};
use super::super::types::{SnapshotTokenBalance, WalletTokenOverview};

async fn fetch_token_metadata_batch(
    mints: &[String],
) -> HashMap<String, crate::tokens::types::Token> {
    if mints.is_empty() {
        return HashMap::new();
    }

    stream::iter(mints.iter().cloned())
        .map(|mint| async move {
            match crate::tokens::get_full_token_async(&mint).await {
                Ok(Some(token)) => Some((mint, token)),
                Ok(None) => None,
                Err(err) => {
                    logger::debug(
                        LogTag::Wallet,
                        &format!("Failed to load token metadata for {mint}: {err}"),
                    );
                    None
                }
            }
        })
        .buffer_unordered(TOKEN_METADATA_CONCURRENCY)
        .filter_map(|entry| async move { entry })
        .collect()
        .await
}

pub(super) async fn enrich_token_overview(
    balances: Vec<SnapshotTokenBalance>,
    max_tokens: usize,
) -> Vec<WalletTokenOverview> {
    let mut rows = Vec::with_capacity(balances.len());

    let mut unique_mints: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for balance in &balances {
        if seen.insert(balance.mint.clone()) {
            unique_mints.push(balance.mint.clone());
        }
    }

    let metadata_map: HashMap<String, crate::tokens::types::Token> =
        fetch_token_metadata_batch(&unique_mints).await;

    for balance in balances {
        let token_meta = metadata_map.get(&balance.mint);

        let (
            symbol,
            name,
            image_url,
            price_sol,
            price_usd,
            liquidity_usd,
            volume_24h,
            last_updated,
            dex_id,
        ) = if let Some(meta) = token_meta {
            let price_sol = if meta.price_sol > 0.0 {
                Some(meta.price_sol)
            } else {
                None
            };
            let price_usd = if meta.price_usd > 0.0 {
                Some(meta.price_usd)
            } else {
                None
            };
            let liquidity_usd = meta.liquidity_usd;
            let volume_24h = meta.volume_h24;
            let last_updated = Some(meta.market_data_last_fetched_at.to_rfc3339());
            let dex_id = Some(meta.data_source.as_str().to_owned());

            let symbol = if meta.symbol.trim().is_empty() {
                short_mint_label(&balance.mint)
            } else {
                meta.symbol.clone()
            };

            (
                symbol,
                Some(meta.name.clone()),
                meta.image_url.clone(),
                price_sol,
                price_usd,
                liquidity_usd,
                volume_24h,
                last_updated,
                dex_id,
            )
        } else {
            (
                short_mint_label(&balance.mint),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
        };

        let value_sol = price_sol.map(|price| price * balance.balance_ui);

        rows.push(WalletTokenOverview {
            mint: balance.mint.clone(),
            symbol,
            name,
            image_url,
            balance_ui: balance.balance_ui,
            balance_raw: balance.balance,
            decimals: balance.decimals,
            is_token_2022: balance.is_token_2022,
            price_sol,
            price_usd,
            value_sol,
            liquidity_usd,
            volume_24h,
            last_updated,
            dex_id,
        });
    }

    rows.sort_by(|a, b| {
        let a_key = a.value_sol.unwrap_or(a.balance_ui);
        let b_key = b.value_sol.unwrap_or(b.balance_ui);
        b_key
            .partial_cmp(&a_key)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let max_tokens = clamp_token_limit(max_tokens);
    if rows.len() > max_tokens {
        rows.truncate(max_tokens);
    }

    rows
}
