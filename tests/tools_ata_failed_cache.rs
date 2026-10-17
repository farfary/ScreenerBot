// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The failed-ATA cache must not park an empty account forever.
//!
//! `close_ata` records every close failure, including a transient RPC error or the
//! race where the account was already gone, and `scan_closeable_atas` then hides every
//! cached row from later cleanup cycles. A cached transient failure therefore has to
//! carry the time it becomes eligible again, or the rent of that account is never
//! reclaimed by the periodic cleanup.

use crate::common;

use chrono::{Duration, Utc};
use screenerbot::tools::database::{
    get_excluded_atas_for_wallet, get_failed_atas_for_wallet, upsert_failed_ata,
};

const WALLET: &str = "Wallet1111111111111111111111111111111111111";
const ATA: &str = "Ata11111111111111111111111111111111111111111";
const PERMANENT_ATA: &str = "PermAta111111111111111111111111111111111111";
const MINT: &str = "TokenMint111111111111111111111111111111111";

#[test]
fn a_transient_close_failure_is_cached_with_a_retry_time() {
    common::run_isolated(
        module_path!(),
        "a_transient_close_failure_is_cached_with_a_retry_time",
        || async {
            let _dir = common::isolated_env();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            screenerbot::tools::init_tools_db().expect("open the tools store");

            upsert_failed_ata(
                ATA,
                Some(MINT),
                WALLET,
                "rpc request timed out while sending the close transaction",
                false,
            )
            .expect("record the failed close");

            let rows = get_failed_atas_for_wallet(WALLET).expect("read the failed ATAs");
            assert_eq!(rows.len(), 1, "the failed ATA is cached once");
            let row = &rows[0];
            assert!(
                !row.is_permanent_failure,
                "a transient failure is not permanent"
            );
            assert!(
                row.next_retry_at.is_some(),
                "a non-permanent failure must record when the ATA is retried, \
                 otherwise the scan skips it for good: {row:?}"
            );
        },
    );
}

#[test]
fn a_cached_failure_is_excluded_only_until_its_retry_time() {
    common::run_isolated(
        module_path!(),
        "a_cached_failure_is_excluded_only_until_its_retry_time",
        || async {
            let _dir = common::isolated_env();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            screenerbot::tools::init_tools_db().expect("open the tools store");

            upsert_failed_ata(ATA, Some(MINT), WALLET, "rpc request timed out", false)
                .expect("record the transient failure");
            upsert_failed_ata(PERMANENT_ATA, Some(MINT), WALLET, "owner mismatch", true)
                .expect("record the permanent failure");

            let now = Utc::now();
            let parked = get_excluded_atas_for_wallet(WALLET, now).expect("read exclusions");
            assert!(
                parked.contains(ATA),
                "a transient failure is parked until its retry time"
            );
            assert!(parked.contains(PERMANENT_ATA));

            let later = get_excluded_atas_for_wallet(WALLET, now + Duration::hours(25))
                .expect("read exclusions after the longest backoff");
            assert!(
                !later.contains(ATA),
                "a transient failure whose retry time has passed is attempted again"
            );
            assert!(
                later.contains(PERMANENT_ATA),
                "a permanent failure is never attempted again"
            );
        },
    );
}
