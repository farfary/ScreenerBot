// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana rent: the live rent-exempt minimum and recognition of rent deposits in recorded transactions.
//!
//! The network sets the rent-exempt minimum and has changed it: an SPL token
//! account held 2,039,280 lamports at 6,960 lamports per byte and holds
//! 1,488,440 at 5,080. Live decisions therefore read the minimum from the chain,
//! and what a closed account returns is the lamports that account holds, never
//! a constant. Recorded transactions keep the rate of their own slot, so
//! recognition accepts every rate the network has charged.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use crate::chains::solana::access::chain_access;

/// Data length of an SPL token account without extensions.
pub const TOKEN_ACCOUNT_LEN: usize = 165;

/// Bytes the runtime bills for on top of an account's data.
const ACCOUNT_STORAGE_OVERHEAD: u64 = 128;

/// Rent-exempt lamports per byte the network has charged, oldest first.
const LAMPORTS_PER_BYTE_HISTORY: [u64; 2] = [6_960, 5_080];

/// Largest account data length recognised as a rent deposit. Covers system,
/// mint, token, Token-2022 with extensions, multisig and metadata accounts
/// while keeping ordinary amounts above ~0.008 SOL out of the match.
const MAX_RECOGNISED_DATA_LEN: u64 = 1_024;

/// How long a minimum read from the chain is reused. The rate changes only
/// through a feature activation at an epoch boundary.
const MINIMUM_CACHE_TTL: Duration = Duration::from_secs(3_600);

static MINIMUM_CACHE: LazyLock<Mutex<HashMap<usize, (u64, Instant)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The rent-exempt minimum, in lamports, for an account of `data_len` bytes at
/// the network's current rate.
pub async fn minimum_balance(data_len: usize) -> crate::chains::solana::Result<u64> {
    if let Ok(cache) = MINIMUM_CACHE.lock() {
        if let Some((lamports, read_at)) = cache.get(&data_len) {
            if read_at.elapsed() < MINIMUM_CACHE_TTL {
                return Ok(*lamports);
            }
        }
    }
    let lamports = chain_access().rent_minimum(data_len).await.map_err(|e| {
        crate::chains::solana::Error::Rpc {
            operation: "getMinimumBalanceForRentExemption",
            detail: e.to_string(),
        }
    })?;
    if let Ok(mut cache) = MINIMUM_CACHE.lock() {
        cache.insert(data_len, (lamports, Instant::now()));
    }
    Ok(lamports)
}

/// The rent-exempt minimum of an account holding `data_len` bytes at
/// `lamports_per_byte`.
pub fn rent_minimum_from_rate(lamports_per_byte: u64, data_len: usize) -> u64 {
    (ACCOUNT_STORAGE_OVERHEAD + data_len as u64).saturating_mul(lamports_per_byte)
}

/// Whether `lamports` is exactly the rent-exempt deposit of an account at any
/// rate the network has charged.
pub fn is_rent_deposit(lamports: u64) -> bool {
    LAMPORTS_PER_BYTE_HISTORY.iter().any(|rate| {
        lamports % rate == 0
            && (ACCOUNT_STORAGE_OVERHEAD..=ACCOUNT_STORAGE_OVERHEAD + MAX_RECOGNISED_DATA_LEN)
                .contains(&(lamports / rate))
    })
}

#[cfg(test)]
mod tests {
    use super::{is_rent_deposit, rent_minimum_from_rate, TOKEN_ACCOUNT_LEN};

    /// A token account's rent from a per-byte rate equals the network's
    /// rent-exempt minimum for its 165 bytes at that rate.
    #[test]
    fn rent_from_the_per_byte_rate_matches_the_network_minimum() {
        assert_eq!(rent_minimum_from_rate(5_080, TOKEN_ACCOUNT_LEN), 1_488_440);
        assert_eq!(rent_minimum_from_rate(6_960, TOKEN_ACCOUNT_LEN), 2_039_280);
    }

    #[test]
    fn deposits_at_the_former_and_the_current_rate_are_recognised() {
        for lamports in [
            2_039_280, // token account, 6,960 per byte
            1_461_600, // mint, 6,960 per byte
            890_880,   // system account, 6,960 per byte
            1_488_440, // token account, 5,080 per byte
            1_066_800, // mint, 5,080 per byte
            1_513_840, // Token-2022 account with the immutable-owner extension
            1_574_800, // Token-2022 account with the transfer-fee extension
            650_240,   // system account, 5,080 per byte
            4_099_560, // metadata account, 5,080 per byte
        ] {
            assert!(is_rent_deposit(lamports), "{lamports} is a rent deposit");
        }
    }

    #[test]
    fn ordinary_amounts_are_not_rent() {
        for lamports in [
            0,
            5_000,
            1_500_000,
            2_000_000,
            10_000_000,
            100_000_000,
            1_000_000_000,
            // A multiple of the rate above the largest recognised account.
            5_080 * 10_000,
        ] {
            assert!(!is_rent_deposit(lamports), "{lamports} is not rent");
        }
    }
}
