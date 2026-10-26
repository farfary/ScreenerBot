// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests of the replacement refusals, the apply classification and the create-only marker.

use super::*;

const OLD: &str = "OldWalletAddress11111111111111111111111111";
const NEW: &str = "NewWalletAddress11111111111111111111111111";

fn allowed() -> StageChecks {
    StageChecks {
        blocker: None,
        stored_open_positions: Some(0),
        trading_wallet: Some(OLD.to_owned()),
        new_address: NEW.to_owned(),
        watched: false,
        pending: false,
    }
}

fn marker() -> Marker {
    Marker {
        chain: "solana".to_owned(),
        previous_address: OLD.to_owned(),
        new_address: NEW.to_owned(),
        backup_directory: "wallet-reset-20261026T120000Z-OldWalle".to_owned(),
        requested_at: Utc::now(),
    }
}

#[test]
fn an_idle_process_with_another_key_stages_and_names_the_replaced_wallet() {
    assert_eq!(check_stage(&allowed()).unwrap(), OLD);
}

#[test]
fn every_restart_blocker_refuses_with_its_reason() {
    for reason in [
        RestartBlocker::TradeInFlight,
        RestartBlocker::ToolInFlight,
        RestartBlocker::OpenPositions { count: 1 },
        RestartBlocker::PositionsLoading,
        RestartBlocker::PositionsUnreadable,
    ] {
        let checks = StageChecks {
            blocker: Some(reason),
            ..allowed()
        };
        assert!(
            matches!(check_stage(&checks), Err(Error::ReplacementBlocked { reason: r }) if r == reason),
            "{reason:?}"
        );
    }
}

#[test]
fn open_positions_in_the_store_refuse_even_when_none_are_loaded() {
    let checks = StageChecks {
        stored_open_positions: Some(3),
        ..allowed()
    };
    assert!(matches!(
        check_stage(&checks),
        Err(Error::ReplacementBlocked {
            reason: RestartBlocker::OpenPositions { count: 3 }
        })
    ));

    let unreadable = StageChecks {
        stored_open_positions: None,
        ..allowed()
    };
    assert!(matches!(
        check_stage(&unreadable),
        Err(Error::ReplacementBlocked {
            reason: RestartBlocker::PositionsUnreadable
        })
    ));
}

#[test]
fn the_same_wallet_a_watched_wallet_and_a_pending_marker_are_refused() {
    let same = StageChecks {
        new_address: OLD.to_owned(),
        ..allowed()
    };
    assert!(matches!(
        check_stage(&same),
        Err(Error::ReplacementSameWallet { .. })
    ));

    let watched = StageChecks {
        watched: true,
        ..allowed()
    };
    assert!(matches!(
        check_stage(&watched),
        Err(Error::ReplacementWalletIsWatched { .. })
    ));

    let pending = StageChecks {
        pending: true,
        ..allowed()
    };
    assert!(matches!(
        check_stage(&pending),
        Err(Error::ReplacementPending)
    ));

    let no_wallet = StageChecks {
        trading_wallet: None,
        ..allowed()
    };
    assert!(matches!(
        check_stage(&no_wallet),
        Err(Error::ReplacementNoTradingWallet)
    ));
}

#[test]
fn the_vault_decides_where_an_apply_resumes() {
    let marker = marker();
    assert_eq!(classify(&marker, Some(OLD), true), ApplyState::NotFlipped);
    assert_eq!(classify(&marker, Some(NEW), true), ApplyState::Flipped);
    assert_eq!(classify(&marker, Some(OLD), false), ApplyState::Stale);
    assert_eq!(classify(&marker, Some("Another"), true), ApplyState::Stale);
    assert_eq!(classify(&marker, None, true), ApplyState::Stale);
}

#[test]
fn a_marker_names_only_a_wallet_reset_directory_inside_backups() {
    assert!(is_backup_directory_name(&marker().backup_directory));
    for name in [
        "../outside",
        "wallet-reset-x/../../outside",
        "/absolute/wallet-reset-x",
        "upgrade-positions.db",
        "",
    ] {
        assert!(!is_backup_directory_name(name), "{name}");
    }
}

#[test]
fn the_marker_write_is_create_only() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("wallet_replacement.json");

    write_marker(&path, &marker()).unwrap();
    let written: Marker = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(written, marker());

    let second = Marker {
        new_address: "Another".to_owned(),
        ..marker()
    };
    assert!(matches!(
        write_marker(&path, &second),
        Err(Error::ReplacementPending)
    ));
    let kept: Marker = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(kept, marker(), "an existing marker is never replaced");
}
