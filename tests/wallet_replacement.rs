// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Replacing the trading wallet: a staged replacement applies at the next start, resumes from every interruption, is abandoned while positions are open, and never lets an unstaged key change pass the wallet check.
//!
//! Every test runs in a process of its own with a data directory of its own: the wallet
//! manager, the store pools and the restart hold are process-wide.

use crate::common;

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use screenerbot::chains::solana::solana_sdk::signature::Keypair;
use screenerbot::database::catalog;
use screenerbot::wallets::replacement::{apply_pending, stage, StagedKey};
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
    add_open_positions(open_positions);
}

fn add_open_positions(count: usize) {
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
                "INSERT INTO positions (wallet_address, chain_id) VALUES ('wallet', 'solana')",
                [],
            )
            .expect("insert an open position");
    }
}

/// The trading wallet with stores and a replacement staged by a fresh key.
async fn staged() -> Scenario {
    let (dir, old) = trading_wallet().await;
    seed_wallet_stores(&old, 0);
    let (new, ciphertext, nonce) = new_key();
    let staged = stage(StagedKey {
        address: new.clone(),
        ciphertext,
        nonce,
        rpc_urls: Vec::new(),
    })
    .await
    .expect("stage the replacement");
    assert_eq!(staged.previous_wallet, old);
    assert_eq!(staged.new_wallet, new);
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

            screenerbot::database::open::open_all();
            let check = WalletValidator::validate_wallet_consistency()
                .await
                .expect("check the wallet");
            assert!(
                matches!(check, WalletValidationResult::FirstRun),
                "the new wallet starts clean: {check:?}"
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
            add_open_positions(1);

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
