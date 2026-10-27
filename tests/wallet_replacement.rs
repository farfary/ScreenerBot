// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Replacing the trading wallet: a stage turns trading off or changes nothing, a staged replacement applies at the next start, resumes from every interruption without overwriting the backup, is abandoned only before its store move while the previous wallet has open positions or the vault changed, and never lets an unstaged key change pass the wallet check.
//!
//! Every test runs in a process of its own with a data directory of its own: the wallet
//! manager, the store pools and the restart hold are process-wide.

use crate::common;

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use screenerbot::chains::solana::solana_sdk::signature::Keypair;
use screenerbot::chains::ChainId;
use screenerbot::database::catalog;
use screenerbot::wallets::replacement::{
    abandoned_replacement, apply_pending, stage, AbandonReason, StagedKey,
};
use screenerbot::wallets::validation::{WalletValidationResult, WalletValidator};
use screenerbot::wallets::WalletRole;

/// A trading wallet with stores, and a staged replacement by another key.
struct Scenario {
    _dir: tempfile::TempDir,
    old: String,
    new: String,
}

/// A fresh key: its address and its encrypted form, as setup produces them.
fn new_key() -> (String, String, String) {
    let keypair = Keypair::new();
    let secret = bs58::encode(keypair.to_bytes()).into_string();
    let encrypted =
        screenerbot::secure_storage::encrypt_private_key(&secret).expect("encrypt the key");
    let address = screenerbot::chains::solana::accounts::keypair_to_address(&keypair);
    (address, encrypted.ciphertext, encrypted.nonce)
}

/// A data directory with `config.toml` and a vault whose trading wallet is a fresh key.
async fn trading_wallet() -> (tempfile::TempDir, String) {
    let dir = common::isolated_env();
    screenerbot::paths::ensure_all_directories().expect("create data directories");
    let old = common::configure_trading_wallet();
    screenerbot::config::save_config(None).expect("write config.toml");
    screenerbot::wallets::initialize()
        .await
        .expect("initialize the wallet manager");
    (dir, old)
}

/// Write each wallet-scoped store as `wallet`'s, with a positions table holding
/// `open_positions` open positions.
fn seed_wallet_stores(wallet: &str, open_positions: usize) {
    for (spec, scope) in catalog::wallet_scoped_stores() {
        for path in spec.paths() {
            let connection = Connection::open(&path).expect("create the store");
            connection
                .execute_batch(&format!(
                    "CREATE TABLE IF NOT EXISTS {} (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
                    scope.metadata_table
                ))
                .expect("create the metadata table");
            connection
                .execute(
                    &format!(
                        "INSERT OR REPLACE INTO {} (key, value) VALUES ('current_wallet', ?1)",
                        scope.metadata_table
                    ),
                    params![wallet],
                )
                .expect("record the wallet");
        }
    }
    add_open_positions(wallet, open_positions);
}

/// Add `count` open positions of `wallet` to the positions store.
fn add_open_positions(wallet: &str, count: usize) {
    let connection = Connection::open(screenerbot::paths::get_positions_db_path())
        .expect("open the positions store");
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS positions (
                wallet_address TEXT NOT NULL,
                chain_id TEXT NOT NULL,
                archived INTEGER NOT NULL DEFAULT 0,
                exit_time TEXT,
                transaction_exit_verified INTEGER NOT NULL DEFAULT 0
             );",
        )
        .expect("create the positions table");
    for _ in 0..count {
        connection
            .execute(
                "INSERT INTO positions (wallet_address, chain_id) VALUES (?1, 'solana')",
                params![wallet],
            )
            .expect("insert an open position");
    }
}

/// The new key `address` as the route stages it, keeping the configured RPC endpoints.
fn staged_key(address: &str, ciphertext: String, nonce: String) -> StagedKey {
    StagedKey {
        chain: ChainId::Solana,
        address: address.to_owned(),
        ciphertext,
        nonce,
        rpc_urls: Vec::new(),
    }
}

/// Switch the Auto Trader and Copy Trading on, in memory and on disk.
fn switch_trading_on() {
    screenerbot::config::update_config_section(
        |cfg| {
            cfg.trader.enabled = true;
            cfg.copy_trading.enabled = true;
        },
        true,
    )
    .expect("switch trading on");
}

/// Whether the Auto Trader and Copy Trading are on.
fn trading_switches() -> (bool, bool) {
    screenerbot::config::with_config(|cfg| (cfg.trader.enabled, cfg.copy_trading.enabled))
}

/// The trading wallet with stores and a replacement staged by a fresh key, staged while
/// trading was on.
async fn staged() -> Scenario {
    let (dir, old) = trading_wallet().await;
    seed_wallet_stores(&old, 0);
    switch_trading_on();
    let (new, ciphertext, nonce) = new_key();
    let staged = stage(staged_key(&new, ciphertext, nonce))
        .await
        .expect("stage the replacement");
    assert_eq!(staged.previous_wallet, old);
    assert_eq!(staged.new_wallet, new);
    assert_eq!(
        trading_switches(),
        (false, false),
        "the stage turns the Auto Trader and Copy Trading off"
    );
    assert!(marker_path().exists(), "the stage writes the marker");
    assert!(
        screenerbot::global::try_hold_activity_for_restart().is_none(),
        "the stage commits the restart hold"
    );
    Scenario {
        _dir: dir,
        old,
        new,
    }
}

fn marker_path() -> PathBuf {
    screenerbot::paths::get_wallet_replacement_marker_path()
}

fn marker() -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(marker_path()).expect("read the marker"))
        .expect("decode the marker")
}

fn backup_directory_of(marker: &serde_json::Value) -> PathBuf {
    screenerbot::paths::get_backups_directory().join(
        marker["backup_directory"]
            .as_str()
            .expect("the marker names its backup directory"),
    )
}

fn wallet_store_paths() -> Vec<PathBuf> {
    catalog::wallet_scoped_stores()
        .flat_map(|(spec, _)| spec.paths())
        .collect()
}

fn wallet_reset_directories() -> Vec<PathBuf> {
    std::fs::read_dir(screenerbot::paths::get_backups_directory())
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("wallet-reset-"))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn config_key_address() -> String {
    let (ciphertext, nonce) = screenerbot::config::with_config(|cfg| {
        (cfg.wallet_encrypted.clone(), cfg.wallet_nonce.clone())
    });
    screenerbot::chains::solana::accounts::address_from_encrypted_key(&ciphertext, &nonce)
        .expect("the config key decrypts")
}

/// The state every applied replacement ends in, whatever interrupted it.
async fn assert_applied(scenario: &Scenario) {
    assert!(!marker_path().exists(), "the marker is removed");
    for path in wallet_store_paths() {
        assert!(!path.exists(), "{} was moved", path.display());
    }

    let backups = wallet_reset_directories();
    assert_eq!(backups.len(), 1, "one wallet-reset backup: {backups:?}");
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(backups[0].join("manifest.json")).expect("read the manifest"),
    )
    .expect("decode the manifest");
    assert_eq!(manifest["reason"], "wallet_replaced");
    assert_eq!(manifest["previousWallet"], scenario.old.as_str());
    assert_eq!(manifest["newWallet"], scenario.new.as_str());
    let mut expected: Vec<String> = wallet_store_paths()
        .iter()
        .filter_map(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect();
    expected.sort();
    let files: Vec<String> = manifest["files"]
        .as_array()
        .expect("the manifest lists its files")
        .iter()
        .filter_map(|file| file.as_str().map(str::to_owned))
        .collect();
    assert_eq!(files, expected);
    for file in &files {
        assert!(backups[0].join(file).exists(), "{file} is in the backup");
    }

    let wallets = screenerbot::wallets::list_wallets(true)
        .await
        .expect("list the wallets");
    let mains: Vec<_> = wallets
        .iter()
        .filter(|wallet| wallet.role == WalletRole::Main && wallet.is_active)
        .collect();
    assert_eq!(mains.len(), 1, "exactly one active main wallet");
    assert_eq!(mains[0].address, scenario.new);
    let previous = wallets
        .iter()
        .find(|wallet| wallet.address == scenario.old)
        .expect("the previous wallet stays in the vault");
    assert_eq!(previous.role, WalletRole::Secondary);
    assert!(previous.is_active, "the previous wallet stays active");

    assert_eq!(config_key_address(), scenario.new);
    let (trader, copy_trading) =
        screenerbot::config::with_config(|cfg| (cfg.trader.enabled, cfg.copy_trading.enabled));
    assert!(!trader, "the Auto Trader is off");
    assert!(!copy_trading, "Copy Trading is off");
}

fn move_into(path: &Path, directory: &Path) {
    std::fs::create_dir_all(directory).expect("create the backup directory");
    std::fs::rename(path, directory.join(path.file_name().expect("file name")))
        .expect("move the store file");
}

#[test]
fn a_staged_replacement_applies_at_the_next_boot() {
    common::run_isolated(
        module_path!(),
        "a_staged_replacement_applies_at_the_next_boot",
        || async {
            let scenario = staged().await;

            let applied = apply_pending()
                .expect("apply the replacement")
                .expect("a replacement was pending");
            assert_eq!(applied.previous_wallet, scenario.old);
            assert_eq!(applied.new_wallet, scenario.new);
            assert_applied(&scenario).await;

            let check = WalletValidator::validate_wallet_consistency()
                .await
                .expect("check the wallet");
            assert!(
                matches!(check, WalletValidationResult::FirstRun),
                "the new wallet starts clean: {check:?}"
            );

            // The stores the boot opens record the new wallet, never the replaced one.
            screenerbot::database::open::open_all();
            let check = WalletValidator::validate_wallet_consistency()
                .await
                .expect("check the wallet");
            assert!(
                matches!(check, WalletValidationResult::Valid),
                "the reopened stores belong to the new wallet: {check:?}"
            );
        },
    );
}

#[test]
fn every_interrupted_apply_resumes_to_the_same_state() {
    common::run_isolated_cases(
        module_path!(),
        "every_interrupted_apply_resumes_to_the_same_state",
        &[
            "not_started",
            "one_store_moved",
            "every_store_moved",
            "vault_flipped",
            "config_written",
        ],
        |case| async move {
            let scenario = staged().await;
            let staged_marker = std::fs::read(marker_path()).expect("read the marker");
            let backup = backup_directory_of(&marker());

            match case.as_str() {
                "not_started" => {}
                "one_store_moved" => {
                    move_into(&screenerbot::paths::get_positions_db_path(), &backup);
                }
                "every_store_moved" => {
                    for path in wallet_store_paths() {
                        move_into(&path, &backup);
                    }
                }
                "vault_flipped" => {
                    // Apply, then restore what the config save and the marker removal
                    // changed: the vault names the new wallet, the config the old one.
                    let old_key = screenerbot::config::with_config(|cfg| {
                        (cfg.wallet_encrypted.clone(), cfg.wallet_nonce.clone())
                    });
                    apply_pending().expect("apply").expect("applied");
                    screenerbot::config::update_config_section(
                        |cfg| {
                            cfg.wallet_encrypted = old_key.0;
                            cfg.wallet_nonce = old_key.1;
                            cfg.trader.enabled = true;
                            cfg.copy_trading.enabled = true;
                        },
                        true,
                    )
                    .expect("restore the old config key");
                    std::fs::write(marker_path(), &staged_marker).expect("restore the marker");
                }
                "config_written" => {
                    apply_pending().expect("apply").expect("applied");
                    std::fs::write(marker_path(), &staged_marker).expect("restore the marker");
                }
                other => panic!("unknown case {other}"),
            }

            apply_pending()
                .expect("resume the replacement")
                .expect("the resumed replacement applies");
            assert_applied(&scenario).await;
        },
    );
}

#[test]
fn an_open_position_abandons_the_replacement_at_boot() {
    common::run_isolated(
        module_path!(),
        "an_open_position_abandons_the_replacement_at_boot",
        || async {
            let scenario = staged().await;
            // A position opened between the request and the restart.
            add_open_positions(&scenario.old, 1);

            assert_eq!(apply_pending().expect("apply"), None);

            assert!(!marker_path().exists(), "the abandoned marker is removed");
            for path in wallet_store_paths() {
                assert!(path.exists(), "{} stays in place", path.display());
            }
            assert!(wallet_reset_directories().is_empty(), "nothing was moved");
            let wallets = screenerbot::wallets::list_wallets(true)
                .await
                .expect("list the wallets");
            let main = wallets
                .iter()
                .find(|wallet| wallet.role == WalletRole::Main)
                .expect("a main wallet");
            assert_eq!(
                main.address, scenario.old,
                "the trading wallet is unchanged"
            );
            let other = wallets
                .iter()
                .find(|wallet| wallet.address == scenario.new)
                .expect("the new key stays in the vault");
            assert_eq!(other.role, WalletRole::Secondary);

            let abandoned = abandoned_replacement().expect("the abandonment is recorded");
            assert_eq!(abandoned.previous_wallet, scenario.old);
            assert_eq!(abandoned.new_wallet, scenario.new);
            assert_eq!(abandoned.reason, AbandonReason::OpenPositions { count: 1 });
            screenerbot::wallets::replacement::dismiss_abandoned_replacement()
                .expect("dismiss the record");
            assert_eq!(abandoned_replacement(), None, "a dismissed record is gone");
        },
    );
}

#[test]
fn a_stale_marker_is_dropped() {
    common::run_isolated(module_path!(), "a_stale_marker_is_dropped", || async {
        let (_dir, old) = trading_wallet().await;
        seed_wallet_stores(&old, 0);
        let marker = serde_json::json!({
            "chain": "solana",
            "previous_address": "AnotherWallet1111111111111111111111111111111",
            "new_address": "YetAnotherWallet111111111111111111111111111",
            "backup_directory": "wallet-reset-20261026T120000Z-AnotherW",
            "requested_at": "2026-10-26T12:00:00Z",
        });
        std::fs::write(marker_path(), serde_json::to_vec(&marker).unwrap())
            .expect("write the marker");

        assert_eq!(apply_pending().expect("apply"), None);

        assert!(!marker_path().exists(), "the stale marker is removed");
        for path in wallet_store_paths() {
            assert!(path.exists(), "{} stays in place", path.display());
        }
        assert!(wallet_reset_directories().is_empty(), "nothing was moved");
        assert_eq!(config_key_address(), old);
    });
}

#[test]
fn the_replaced_key_stays_exportable() {
    common::run_isolated(
        module_path!(),
        "the_replaced_key_stays_exportable",
        || async {
            let scenario = staged().await;
            apply_pending().expect("apply").expect("applied");

            let wallets = screenerbot::wallets::list_wallets(true)
                .await
                .expect("list the wallets");
            let previous = wallets
                .iter()
                .find(|wallet| wallet.address == scenario.old)
                .expect("the previous wallet stays in the vault");
            let export = screenerbot::wallets::export_wallet(previous.id)
                .await
                .expect("export the previous wallet");
            assert_eq!(export.address, scenario.old);
            assert!(!export.private_key.is_empty());
        },
    );
}

#[test]
fn a_hand_edited_config_key_does_not_change_the_trading_wallet() {
    common::run_isolated(
        module_path!(),
        "a_hand_edited_config_key_does_not_change_the_trading_wallet",
        || async {
            let (_dir, old) = trading_wallet().await;
            let (_, ciphertext, nonce) = new_key();
            screenerbot::config::update_config_section(
                |cfg| {
                    cfg.wallet_encrypted = ciphertext;
                    cfg.wallet_nonce = nonce;
                },
                true,
            )
            .expect("edit the config key");

            screenerbot::wallets::initialize()
                .await
                .expect("initialize the wallet manager again");

            let main = screenerbot::wallets::get_main_wallet()
                .await
                .expect("read the main wallet")
                .expect("a main wallet");
            assert_eq!(main.address, old, "the vault keeps the trading wallet");
        },
    );
}

#[test]
fn a_lost_vault_still_trips_the_wallet_check() {
    common::run_isolated(
        module_path!(),
        "a_lost_vault_still_trips_the_wallet_check",
        || async {
            let _dir = common::isolated_env();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            // The stores belong to a wallet the empty vault never held.
            let (previous, _, _) = new_key();
            seed_wallet_stores(&previous, 0);
            let current = common::configure_trading_wallet();
            screenerbot::wallets::initialize()
                .await
                .expect("initialize the wallet manager");

            let check = WalletValidator::validate_wallet_consistency()
                .await
                .expect("check the wallet");
            match check {
                WalletValidationResult::Mismatch {
                    current: checked,
                    stored,
                    ..
                } => {
                    assert_eq!(checked, current);
                    assert_eq!(stored, previous);
                }
                other => panic!("an unstaged key change must trip the check: {other:?}"),
            }
        },
    );
}

/// The main wallet the vault names now.
async fn vault_main() -> String {
    screenerbot::wallets::list_wallets(true)
        .await
        .expect("list the wallets")
        .into_iter()
        .find(|wallet| wallet.role == WalletRole::Main && wallet.is_active)
        .expect("a main wallet")
        .address
}

fn store_file_name(path: &Path) -> String {
    path.file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn a_resume_after_only_the_wal_moved_completes_and_keeps_its_committed_frames() {
    common::run_isolated(
        module_path!(),
        "a_resume_after_only_the_wal_moved_completes_and_keeps_its_committed_frames",
        || async {
            let scenario = staged().await;
            let backup = backup_directory_of(&marker());
            let positions = screenerbot::paths::get_positions_db_path();
            let sidecar = |suffix: &str| PathBuf::from(format!("{}{suffix}", positions.display()));

            // The main file holds an open position whose close is committed only in the
            // WAL: the main file alone shows it open.
            let (main_file, wal, shm) = {
                let connection = Connection::open(&positions).expect("open the positions store");
                connection
                    .execute_batch("PRAGMA journal_mode = WAL; PRAGMA wal_autocheckpoint = 0;")
                    .expect("switch to WAL");
                connection
                    .execute(
                        "INSERT INTO positions (wallet_address, chain_id) VALUES (?1, 'solana')",
                        params![scenario.old],
                    )
                    .expect("open a position");
                connection
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                    .expect("checkpoint the open position");
                connection
                    .execute(
                        "UPDATE positions SET exit_time = '2026-10-27T10:00:00Z', transaction_exit_verified = 1 WHERE wallet_address = ?1",
                        params![scenario.old],
                    )
                    .expect("close the position in the WAL");
                (
                    std::fs::read(&positions).expect("read the main file"),
                    std::fs::read(sidecar("-wal")).expect("read the WAL"),
                    std::fs::read(sidecar("-shm")).expect("read the shared memory"),
                )
            };
            // The interrupted apply moved the WAL and the shared memory, then stopped.
            std::fs::write(&positions, main_file).expect("restore the main file");
            let _ = std::fs::remove_file(sidecar("-wal"));
            let _ = std::fs::remove_file(sidecar("-shm"));
            std::fs::create_dir_all(&backup).expect("create the backup directory");
            let name = store_file_name(&positions);
            std::fs::write(backup.join(format!("{name}-wal")), wal).expect("move the WAL");
            std::fs::write(backup.join(format!("{name}-shm")), shm).expect("move the shm");

            apply_pending()
                .expect("resume the replacement")
                .expect("a begun move resumes and is never abandoned");

            assert!(!marker_path().exists(), "the marker is removed");
            assert_eq!(abandoned_replacement(), None, "nothing was abandoned");
            assert!(!positions.exists(), "the positions store moved");
            assert_eq!(vault_main().await, scenario.new);
            let moved = Connection::open(backup.join(&name)).expect("open the moved store");
            let open: i64 = moved
                .query_row(
                    "SELECT COUNT(*) FROM positions WHERE wallet_address = ?1 AND exit_time IS NULL",
                    params![scenario.old],
                    |row| row.get(0),
                )
                .expect("count open positions");
            assert_eq!(
                open, 0,
                "the close committed in the WAL moved with the store"
            );
        },
    );
}

#[test]
fn a_resume_after_one_store_moved_completes_with_an_open_position() {
    common::run_isolated(
        module_path!(),
        "a_resume_after_one_store_moved_completes_with_an_open_position",
        || async {
            let scenario = staged().await;
            // A position booked outside a trade between the stage and the restart.
            add_open_positions(&scenario.old, 1);
            let backup = backup_directory_of(&marker());
            let positions = screenerbot::paths::get_positions_db_path();
            let other = wallet_store_paths()
                .into_iter()
                .find(|path| *path != positions)
                .expect("a wallet-scoped store besides positions");
            move_into(&other, &backup);

            apply_pending()
                .expect("resume the replacement")
                .expect("a begun move resumes and is never abandoned");

            assert_applied(&scenario).await;
            let manifest: serde_json::Value = serde_json::from_slice(
                &std::fs::read(backup.join("manifest.json")).expect("read the manifest"),
            )
            .expect("decode the manifest");
            assert_eq!(manifest["movedOpenPositions"][&scenario.old], 1);
        },
    );
}

#[test]
fn a_resume_never_replaces_a_backed_up_store() {
    common::run_isolated(
        module_path!(),
        "a_resume_never_replaces_a_backed_up_store",
        || async {
            let scenario = staged().await;
            let backup = backup_directory_of(&marker());
            let positions = screenerbot::paths::get_positions_db_path();
            move_into(&positions, &backup);
            let backed_up = backup.join(store_file_name(&positions));
            let original = std::fs::read(&backed_up).expect("read the backed-up store");
            // A core without the replacement re-created the store at its original path.
            std::fs::write(&positions, b"written by another core").expect("re-create the store");

            assert!(apply_pending().is_err(), "the resume refuses the conflict");

            assert_eq!(
                std::fs::read(&backed_up).expect("read the backed-up store"),
                original,
                "the backed-up store is unchanged"
            );
            assert_eq!(
                std::fs::read(&positions).expect("read the re-created store"),
                b"written by another core"
            );
            assert!(marker_path().exists(), "the replacement stays pending");
            assert_eq!(vault_main().await, scenario.old, "the vault is not flipped");
        },
    );
}

#[test]
fn another_wallets_open_positions_do_not_block_a_stage() {
    common::run_isolated(
        module_path!(),
        "another_wallets_open_positions_do_not_block_a_stage",
        || async {
            let (_dir, old) = trading_wallet().await;
            seed_wallet_stores(&old, 0);
            add_open_positions("EarlierWa11et111111111111111111111111111111", 2);
            let (new, ciphertext, nonce) = new_key();

            let staged = stage(staged_key(&new, ciphertext, nonce))
                .await
                .expect("rows of another wallet do not block the stage");

            assert_eq!(staged.previous_wallet, old);
            assert!(marker_path().exists());
        },
    );
}

#[test]
fn a_refused_stage_changes_nothing() {
    common::run_isolated_cases(
        module_path!(),
        "a_refused_stage_changes_nothing",
        &[
            "trade_in_flight",
            "open_positions",
            "same_wallet",
            "pending",
        ],
        |case| async move {
            let (_dir, old) = trading_wallet().await;
            seed_wallet_stores(&old, 0);
            switch_trading_on();
            let (new, ciphertext, nonce) = new_key();
            let mut key = staged_key(&new, ciphertext, nonce);
            let mut _trade = None;
            match case.as_str() {
                "trade_in_flight" => {
                    _trade = Some(screenerbot::global::begin_trade().expect("begin a trade"));
                }
                "open_positions" => add_open_positions(&old, 1),
                "same_wallet" => {
                    let (ciphertext, nonce) = screenerbot::config::with_config(|cfg| {
                        (cfg.wallet_encrypted.clone(), cfg.wallet_nonce.clone())
                    });
                    key = staged_key(&old, ciphertext, nonce);
                }
                "pending" => {
                    std::fs::write(marker_path(), b"{}").expect("write a pending marker");
                }
                other => panic!("unknown case {other}"),
            }
            let marker_before = std::fs::read(marker_path()).ok();

            assert!(stage(key).await.is_err(), "the stage refuses");

            assert_eq!(trading_switches(), (true, true), "trading stays on");
            assert_eq!(std::fs::read(marker_path()).ok(), marker_before);
            assert!(
                screenerbot::global::try_hold_activity_for_restart().is_some(),
                "the restart hold is released"
            );
            let wallets = screenerbot::wallets::list_wallets(true)
                .await
                .expect("list the wallets");
            assert!(
                case == "same_wallet" || wallets.iter().all(|wallet| wallet.address != new),
                "the new key is not added"
            );
        },
    );
}

#[test]
fn a_stage_keeps_the_configured_endpoints_unless_it_was_given_some() {
    common::run_isolated_cases(
        module_path!(),
        "a_stage_keeps_the_configured_endpoints_unless_it_was_given_some",
        &["configured", "given"],
        |case| async move {
            let (_dir, old) = trading_wallet().await;
            seed_wallet_stores(&old, 0);
            let configured = vec![
                "https://rpc-one.example.com/".to_owned(),
                "https://rpc-two.example.com/".to_owned(),
            ];
            let urls = configured.clone();
            screenerbot::config::update_config_section(
                move |cfg| cfg.chains.rpc_mut(ChainId::Solana).urls = urls,
                true,
            )
            .expect("configure the endpoints");
            let (new, ciphertext, nonce) = new_key();
            let mut key = staged_key(&new, ciphertext, nonce);
            let given = vec!["https://rpc-new.example.com/".to_owned()];
            if case == "given" {
                key.rpc_urls = given.clone();
            }

            stage(key).await.expect("stage the replacement");

            let saved = screenerbot::config::with_config(|cfg| {
                cfg.chains.rpc(ChainId::Solana).urls.clone()
            });
            let on_disk = std::fs::read_to_string(screenerbot::paths::get_config_path())
                .expect("read config.toml");
            let expected = if case == "given" { given } else { configured };
            assert_eq!(saved, expected);
            for url in &expected {
                assert!(on_disk.contains(url.as_str()), "{url} is saved");
            }
        },
    );
}

#[test]
fn a_reused_vault_row_with_an_unreadable_key_takes_the_validated_key() {
    common::run_isolated(
        module_path!(),
        "a_reused_vault_row_with_an_unreadable_key_takes_the_validated_key",
        || async {
            let (_dir, old) = trading_wallet().await;
            seed_wallet_stores(&old, 0);
            let (new, ciphertext, nonce) = new_key();
            // A row of the new address from an earlier replacement, whose key no longer
            // decrypts.
            let (_, other_ciphertext, other_nonce) = new_key();
            Connection::open(screenerbot::paths::get_wallets_db_path())
                .expect("open the vault")
                .execute(
                    "INSERT INTO wallets (chain_id, name, address, encrypted_key, nonce, role, wallet_type, created_at)
                     VALUES ('solana', '', ?1, ?2, ?3, 'secondary', 'imported', '2026-10-01T00:00:00Z')",
                    params![new, other_ciphertext, other_nonce],
                )
                .expect("insert the stale row");

            stage(staged_key(&new, ciphertext, nonce))
                .await
                .expect("stage the replacement");

            let (stored_ciphertext, stored_nonce): (String, String) =
                Connection::open(screenerbot::paths::get_wallets_db_path())
                    .expect("open the vault")
                    .query_row(
                        "SELECT encrypted_key, nonce FROM wallets WHERE address = ?1",
                        params![new],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .expect("read the row");
            assert_eq!(
                screenerbot::chains::solana::accounts::address_from_encrypted_key(
                    &stored_ciphertext,
                    &stored_nonce
                )
                .expect("the row's key decrypts"),
                new
            );
        },
    );
}

#[test]
fn a_second_trading_wallet_abandons_before_anything_moves() {
    common::run_isolated(
        module_path!(),
        "a_second_trading_wallet_abandons_before_anything_moves",
        || async {
            let scenario = staged().await;
            let (other, ciphertext, nonce) = new_key();
            Connection::open(screenerbot::paths::get_wallets_db_path())
                .expect("open the vault")
                .execute(
                    "INSERT INTO wallets (chain_id, name, address, encrypted_key, nonce, role, wallet_type, created_at)
                     VALUES ('solana', 'Main Wallet', ?1, ?2, ?3, 'main', 'imported', '2026-10-01T00:00:00Z')",
                    params![other, ciphertext, nonce],
                )
                .expect("insert a second main wallet");

            assert_eq!(apply_pending().expect("apply"), None);

            assert!(!marker_path().exists(), "the abandoned marker is removed");
            for path in wallet_store_paths() {
                assert!(path.exists(), "{} stays in place", path.display());
            }
            assert!(wallet_reset_directories().is_empty(), "nothing was moved");
            let abandoned = abandoned_replacement().expect("the abandonment is recorded");
            assert_eq!(abandoned.reason, AbandonReason::VaultChanged);
            assert_eq!(abandoned.previous_wallet, scenario.old);
        },
    );
}

#[test]
fn an_apply_while_a_config_migration_is_pending_writes_only_its_keys() {
    common::run_isolated(
        module_path!(),
        "an_apply_while_a_config_migration_is_pending_writes_only_its_keys",
        || async {
            let dir = tempfile::tempdir().expect("temp dir");
            std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            // The start loads a config in a layout a load-time migration changes, which
            // stays on disk until startup completes.
            let (old, old_ciphertext, old_nonce) = new_key();
            let legacy = format!(
                "wallet_encrypted = \"{old_ciphertext}\"\nwallet_nonce = \"{old_nonce}\"\n\n{}",
                include_str!("fixtures/v0.2.13-config.toml")
            );
            let path = screenerbot::paths::get_config_path();
            std::fs::write(&path, &legacy).expect("write the legacy config");
            screenerbot::config::utils::load_config_from_path(path.to_str().expect("utf-8 path"))
                .expect("load the legacy config");
            screenerbot::wallets::initialize()
                .await
                .expect("initialize the wallet manager");
            seed_wallet_stores(&old, 0);
            // Staged by an earlier process; this one only applies it.
            std::fs::remove_file(&path).expect("set the config aside");
            let (new, ciphertext, nonce) = new_key();
            stage(staged_key(&new, ciphertext, nonce))
                .await
                .expect("stage the replacement");
            std::fs::write(&path, &legacy).expect("restore the legacy config");

            apply_pending().expect("apply").expect("applied");

            let written: toml::Table =
                toml::from_str(&std::fs::read_to_string(&path).expect("read config"))
                    .expect("config is TOML");
            for section in ["rpc", "swaps", "sol_price"] {
                assert!(
                    written.contains_key(section),
                    "[{section}] was migrated early"
                );
            }
            assert!(
                !written.contains_key("chains"),
                "the migrated layout reached disk"
            );
            assert_eq!(written["trader"]["enabled"].as_bool(), Some(false));
            assert_eq!(
                written["trader"]["max_open_positions"].as_integer(),
                Some(4)
            );
            assert_eq!(written["copy_trading"]["enabled"].as_bool(), Some(false));
            assert_eq!(config_key_address(), new);
            let on_disk_key = (
                written["wallet_encrypted"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                written["wallet_nonce"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            );
            assert_eq!(
                screenerbot::chains::solana::accounts::address_from_encrypted_key(
                    &on_disk_key.0,
                    &on_disk_key.1
                )
                .expect("the written key decrypts"),
                new
            );
        },
    );
}
