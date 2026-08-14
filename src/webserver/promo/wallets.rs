//! Promo generator for the **Wallets page's wallet records** (`/api/wallets`) and
//! its watch targets (`/api/wallets/watch/*`).
//!
//! Distinct from `wallet.rs`, which fixtures the single *trading* wallet the header
//! and Main Wallet tab report (`/api/wallet/*`). This module owns the multi-wallet
//! table behind the Secondaries and Archive tabs and the observation list behind the
//! Watched tab.
//!
//! Without fixtures those three tabs are the only place in the app that puts the
//! operator's own key material on screen: they list the real local wallet records,
//! addresses included, and the Watched tab renders "Watched addresses could not be
//! loaded" whenever the watch database has not been opened. Neither is publishable,
//! so the promo session substitutes a plausible desk: one main wallet, two
//! secondaries with distinct jobs, two retired wallets, and four watched addresses
//! whose subscriptions report cleanly.

use chrono::{Duration, Utc};

use crate::wallets::watch::{WatchSource, WatchStatus, WatchTarget};
use crate::wallets::{Wallet, WalletRole, WalletType};

use super::data::PROMO_WALLET_ADDRESS;

/// One wallet record: (id, name, address, role, type, age days, last-used hours,
/// notes). The main row reuses `PROMO_WALLET_ADDRESS` so the Wallets table and the
/// header describe the same wallet.
type PromoWallet = (
    i64,
    &'static str,
    &'static str,
    WalletRole,
    WalletType,
    i64,
    Option<i64>,
    Option<&'static str>,
);

fn promo_wallets() -> Vec<PromoWallet> {
    vec![
        (
            1,
            "Main Trading",
            PROMO_WALLET_ADDRESS,
            WalletRole::Main,
            WalletType::Generated,
            168,
            Some(0),
            Some("Auto-trading wallet. Funded from cold storage weekly."),
        ),
        (
            2,
            "Launch Sniper",
            "7Np41oeYqPefeNQEHSv1UDhYrehxin3NStELsSKCT4K2",
            WalletRole::Secondary,
            WalletType::Generated,
            96,
            Some(3),
            Some("Small size, new-pair entries only."),
        ),
        (
            3,
            "Manual Desk",
            "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
            WalletRole::Secondary,
            WalletType::Imported,
            72,
            Some(19),
            Some("Manual swaps and position tests."),
        ),
        (
            4,
            "Q1 Archive",
            "5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9",
            WalletRole::Archive,
            WalletType::Imported,
            310,
            Some(2_160),
            Some("Retired after the Q1 rotation. Drained."),
        ),
        (
            5,
            "Cold Backup",
            "GDfnEsia2WLAW5t8yx2X5j2mkfA74i5kY9dGZZ2q5wG7",
            WalletRole::Archive,
            WalletType::Imported,
            420,
            None,
            Some("Offline backup key. Never used for trading."),
        ),
    ]
}

/// Generate the wallet records behind the Wallets page.
///
/// `include_inactive` mirrors the real handler's query: archived wallets are
/// inactive, so the Secondaries tab asks without it and the Archive tab asks with
/// it. Returning the archive rows unconditionally would leave the Secondaries table
/// listing wallets it is meant to exclude.
pub fn get_promo_wallets(include_inactive: bool) -> Vec<Wallet> {
    let now = Utc::now();

    promo_wallets()
        .into_iter()
        .map(
            |(id, name, address, role, wallet_type, age_days, used_hours, notes)| {
                let is_active = role != WalletRole::Archive;
                Wallet {
                    id,
                    name: name.to_owned(),
                    address: address.to_owned(),
                    role,
                    wallet_type,
                    created_at: now - Duration::days(age_days),
                    last_used_at: used_hours.map(|hours| now - Duration::hours(hours)),
                    notes: notes.map(str::to_owned),
                    is_active,
                }
            },
        )
        .filter(|wallet| include_inactive || wallet.is_active)
        .collect()
}

/// One watch target: (id, label, address, copy task id, enabled, age days,
/// last-activity minutes, last signature).
///
/// The first three addresses are exactly the wallets the Copy Trading fixture
/// copies, carrying the matching `WatchSource::Copy` — a copy task consumes its
/// target's activity through this list, so a Watched tab that did not contain them
/// would contradict the Auto Trader tab. The fourth is alert-only, which is what
/// makes the two source kinds visible side by side.
type PromoTarget = (
    i64,
    &'static str,
    &'static str,
    Option<i64>,
    bool,
    i64,
    Option<i64>,
    Option<&'static str>,
);

const PROMO_TARGETS: &[PromoTarget] = &[
    (
        1,
        "Whale · early rotations",
        "GDfnEsia2WLAW5t8yx2X5j2mkfA74i5kY9dGZZ2q5wG7",
        Some(1),
        true,
        11,
        Some(4),
        Some("4vJ9JU1bJJE96FbKdjWTnPjPCLu3B1Kt3Zi9Cf8mLpKzKxWQXsAoUqRnMH4kSXtGDb1kZ3aQGvSbXbNQAaCu7T2m"),
    ),
    (
        2,
        "Launch sniper",
        "5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9",
        Some(2),
        true,
        9,
        Some(17),
        Some("2sBqQ4rE1WjKPBd8Nz6nAmVhX7uFyTgLcR3JdWvKp9SxHnQaZmU5oYt6DcEbLfGiRw8PkVnT1MzXyAbCdEfGh3Jk"),
    ),
    (
        3,
        "Momentum desk",
        "3nMFwZXwY1s1M5s8vYAHqd4wGs4iSxXE4LRoUMMYqEgF",
        Some(3),
        true,
        21,
        Some(38),
        Some("5hKpTnQ2XcVbNm8RwEyUiOpAsDfGhJkLzXcVbNm4QwErTyUiOpAsDfGhJkL7ZxCvBnM2QwErTyUiOpAsDfGh6JkL"),
    ),
    (
        4,
        "Treasury outflows (alert only)",
        "BXP2gNKuqZBt4YFtGrkkPQ8sLxwvfgSyzqjfGYRnAoLp",
        None,
        false,
        30,
        Some(1_440),
        None,
    ),
];

fn target(entry: &PromoTarget) -> WatchTarget {
    let (id, label, address, copy_task, enabled, age_days, ..) = *entry;
    let created_at = Utc::now() - Duration::days(age_days);
    let mut sources = vec![WatchSource::Alert { rule_id: id }];
    if let Some(task_id) = copy_task {
        sources.insert(0, WatchSource::Copy { task_id });
    }
    WatchTarget {
        id: Some(id),
        address: address.to_owned(),
        label: Some(label.to_owned()),
        sources,
        enabled,
        created_at,
        updated_at: created_at + Duration::hours(1),
    }
}

/// Generate the Watched tab's target list.
pub fn get_promo_watch_targets() -> Vec<WatchTarget> {
    PROMO_TARGETS.iter().map(target).collect()
}

/// Generate one target's status row.
///
/// A disabled target is never subscribed and reports no error — the tab reads a
/// missing subscription on an enabled target as a fault, so the two flags have to
/// agree or the row contradicts its own toggle.
pub fn get_promo_watch_status(id: i64) -> Option<WatchStatus> {
    let entry = PROMO_TARGETS.iter().find(|entry| entry.0 == id)?;
    let (.., enabled, _age_days, activity_minutes, signature) = *entry;

    Some(WatchStatus {
        target: target(entry),
        subscribed: enabled,
        last_activity_at: activity_minutes.map(|mins| Utc::now() - Duration::minutes(mins)),
        last_signature: signature.map(str::to_owned),
        last_error: None,
    })
}
