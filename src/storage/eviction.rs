// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Rebuildable rows as evictable units: what each evicting store offers with its estimated size and last use, and how a planned eviction reaches the store's own domain sweep through its open pool.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::DateTime;

use crate::chains::ChainId;
use crate::database::catalog::{OHLCVS, TOKENS};
use crate::database::handles::StoreHandle;
use crate::errors::{DatabaseError, StorageError};
use crate::tokens::retention::ProtectedMints;

use super::report::unix_seconds;
use super::types::{Evictable, EvictionUnit, Protection, StoreUsage};

const DAY_SECS: u64 = 24 * 60 * 60;

/// Token age bands eviction steps through, in days since last use, oldest first.
/// Bands newer than the protection floor are never offered.
pub const TOKEN_AGE_BANDS_DAYS: [u64; 9] = [365, 180, 90, 60, 30, 14, 7, 3, 1];

/// The chain an open store file belongs to.
fn chain_of(handle: &StoreHandle) -> Option<ChainId> {
    ChainId::ALL
        .iter()
        .copied()
        .find(|chain| handle.spec.path(*chain) == handle.path)
}

fn store_failed(handle: &StoreHandle) -> impl Fn(DatabaseError) -> StorageError + '_ {
    |source| StorageError::Store {
        store: handle.spec.id.to_owned(),
        source,
    }
}

/// `part` of `bytes`, out of `whole`.
fn share(bytes: u64, part: u64, whole: u64) -> u64 {
    if whole == 0 {
        return 0;
    }
    u64::try_from(u128::from(bytes) * u128::from(part.min(whole)) / u128::from(whole))
        .unwrap_or(u64::MAX)
}

/// Every evictable unit the open stores offer. `protected` lists the tokens in use;
/// the tokens whose charts are monitored now are added from the chart stores. Data
/// last used within `floor` is protected as recently used: token age bands start at
/// the floor, and a chart active within it is marked so. A store whose units cannot
/// be listed offers none and is reported in the result's failures.
pub fn collect(
    stores: &[StoreHandle],
    usages: &[StoreUsage],
    protected: &ProtectedMints,
    floor: Duration,
    now: SystemTime,
) -> (Vec<Evictable>, Vec<StorageError>) {
    let mut protected = protected.clone();
    let mut failures = Vec::new();
    for handle in stores.iter().filter(|handle| handle.spec.id == OHLCVS.id) {
        let Some(chain) = chain_of(handle) else {
            continue;
        };
        let active = handle
            .pool
            .get()
            .map_err(DatabaseError::from)
            .and_then(|conn| crate::ohlcvs::retention::active_chart_mints(&conn, chain));
        match active {
            Ok(mints) => {
                for mint in mints {
                    protected.insert(mint, Protection::Watched);
                }
            }
            Err(error) => {
                failures.push(store_failed(handle)(error));
                return (Vec::new(), failures);
            }
        }
    }

    let keep = Arc::new(protected.mints());
    let mut units = Vec::new();
    for handle in stores
        .iter()
        .filter(|handle| handle.spec.retention.evicts())
    {
        let (Some(chain), Some(usage)) = (
            chain_of(handle),
            usages.iter().find(|usage| usage.path == handle.path),
        ) else {
            continue;
        };
        let offered = if handle.spec.id == TOKENS.id {
            token_units(handle, chain, usage, &keep, floor, now)
        } else if handle.spec.id == OHLCVS.id {
            chart_units(handle, chain, usage, &protected, floor, now)
        } else {
            Ok(Vec::new())
        };
        match offered {
            Ok(offered) => units.extend(offered),
            Err(error) => failures.push(error),
        }
    }
    (units, failures)
}

/// One unit per age band: every token last used before the band's cutoff. A band's
/// size is estimated from the tokens discovered between its cutoff and the next older
/// one, at the store's average live bytes per token; tokens refreshed since their
/// discovery are kept by the sweep, so the estimate only orders the bands.
fn token_units(
    handle: &StoreHandle,
    chain: ChainId,
    usage: &StoreUsage,
    keep: &Arc<HashSet<String>>,
    floor: Duration,
    now: SystemTime,
) -> Result<Vec<Evictable>, StorageError> {
    let cutoffs: Vec<SystemTime> = TOKEN_AGE_BANDS_DAYS
        .into_iter()
        .map(|days| Duration::from_secs(days * DAY_SECS))
        .filter(|age| *age >= floor)
        .filter_map(|age| now.checked_sub(age))
        .collect();
    let seconds: Vec<i64> = cutoffs.iter().copied().map(unix_seconds).collect();
    let conn = handle
        .pool
        .get()
        .map_err(|error| store_failed(handle)(DatabaseError::from(error)))?;
    let (total, before) =
        crate::tokens::database::retention::discovery_counts(&conn, chain, &seconds)
            .map_err(store_failed(handle))?;
    let mut older = 0;
    let mut units = Vec::new();
    for ((cutoff, cutoff_seconds), count) in cutoffs.into_iter().zip(seconds).zip(before) {
        let band = count.saturating_sub(older);
        older = count.max(older);
        if band == 0 {
            continue;
        }
        units.push(Evictable {
            store: handle.spec.id,
            path: handle.path.clone(),
            unit: EvictionUnit::TokensUnusedBefore {
                cutoff: cutoff_seconds,
                keep: Arc::clone(keep),
            },
            bytes: share(usage.live_bytes(), band, total),
            last_used: cutoff,
            protection: None,
        });
    }
    Ok(units)
}

/// One unit per token whose chart is no longer monitored, sized by its share of the
/// store's candles. A chart of a token in use, or active within the floor, is
/// protected.
fn chart_units(
    handle: &StoreHandle,
    chain: ChainId,
    usage: &StoreUsage,
    protected: &ProtectedMints,
    floor: Duration,
    now: SystemTime,
) -> Result<Vec<Evictable>, StorageError> {
    let conn = handle
        .pool
        .get()
        .map_err(|error| store_failed(handle)(DatabaseError::from(error)))?;
    let (total, charts) =
        crate::ohlcvs::retention::inactive_charts(&conn, chain).map_err(store_failed(handle))?;
    Ok(charts
        .into_iter()
        .map(|chart| {
            let last_used = DateTime::parse_from_rfc3339(&chart.last_activity)
                .ok()
                .and_then(|time| u64::try_from(time.timestamp()).ok())
                .map_or(UNIX_EPOCH, |seconds| {
                    UNIX_EPOCH + Duration::from_secs(seconds)
                });
            let recent =
                now.duration_since(last_used).is_ok_and(|idle| idle < floor) || last_used > now;
            let protection = protected
                .protection(&chart.mint)
                .or(recent.then_some(Protection::RecentlyViewed));
            Evictable {
                store: handle.spec.id,
                path: handle.path.clone(),
                bytes: share(usage.live_bytes(), chart.candles, total),
                unit: EvictionUnit::ChartHistory {
                    mint: chart.mint,
                    last_activity: chart.last_activity,
                },
                last_used,
                protection,
            }
        })
        .collect())
}

/// Carry out one eviction through the store's own sweep. Returns the rows deleted.
pub fn evict(
    handle: &StoreHandle,
    unit: &EvictionUnit,
    stop: &dyn Fn() -> bool,
) -> Result<u64, StorageError> {
    let mismatch = || StorageError::Eviction {
        store: handle.spec.id.to_owned(),
        unit: unit.to_string(),
    };
    let chain = chain_of(handle).ok_or_else(mismatch)?;
    let mut conn = handle
        .pool
        .get()
        .map_err(|error| store_failed(handle)(DatabaseError::from(error)))?;
    match unit {
        EvictionUnit::TokensUnusedBefore { cutoff, keep } if handle.spec.id == TOKENS.id => {
            crate::tokens::database::retention::evict_tokens_unused_before(
                &mut conn, chain, *cutoff, keep, stop,
            )
            .map(|eviction| eviction.rows)
            .map_err(store_failed(handle))
        }
        EvictionUnit::ChartHistory {
            mint,
            last_activity,
        } if handle.spec.id == OHLCVS.id => crate::ohlcvs::retention::evict_chart_history(
            &mut conn,
            chain,
            mint,
            last_activity,
            stop,
        )
        .map(|eviction| eviction.rows)
        .map_err(store_failed(handle)),
        _ => Err(mismatch()),
    }
}
