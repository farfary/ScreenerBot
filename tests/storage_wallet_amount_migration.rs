// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Complete v0.2.13 wallet schemas through the production initializers.

use rusqlite::{params, Connection};
use std::path::Path;
use std::process::Command;

const CHILD: &str = "SCREENERBOT_WALLET_AMOUNT_CHILD";
const CASES: [u64; 4] = [0, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX];

fn open(dir: &Path, file: &str) -> Connection {
    Connection::open(dir.join("data").join(file)).unwrap()
}

fn boot(dir: &Path) {
    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("v0_2_13_wallet_amount_storage_survives_two_initializers")
        .env(CHILD, "1")
        .env("SCREENERBOT_DATA_DIR", dir)
        .status()
        .unwrap();
    assert!(status.success(), "wallet initializer failed");
}

#[test]
fn v0_2_13_wallet_amount_storage_survives_two_initializers() {
    if std::env::var_os(CHILD).is_some() {
        screenerbot::paths::ensure_all_directories().unwrap();
        screenerbot::config::load_config().unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            screenerbot::wallets::initialize().await.unwrap();
            screenerbot::wallet::initialize_wallet_database()
                .await
                .unwrap();
        });
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    let wallets = open(dir.path(), "wallets.db");
    wallets
        .execute_batch(include_str!("fixtures/v0.2.13-wallets.sql"))
        .unwrap();
    wallets.execute("INSERT INTO wallets (id, name, address, encrypted_key, nonce, role, notes, created_at) VALUES (17, 'main', 'address', 'ciphertext', 'nonce', 'main', 'kept', '2025-01-01')", []).unwrap();
    for (i, amount) in CASES.iter().enumerate() {
        wallets.execute("INSERT INTO wallet_token_balances (wallet_id, mint, balance, ui_amount, decimals, symbol, name, is_token_2022, updated_at) VALUES (17, ?1, ?2, 12.5, 9, 'SYM', 'Named', 1, '2025-01-02')", params![format!("mint-{i}"), *amount as i64]).unwrap();
    }
    drop(wallets);

    let monitor = open(dir.path(), "wallet.db");
    monitor
        .execute_batch(include_str!("fixtures/v0.2.13-wallet-monitor.sql"))
        .unwrap();
    monitor.execute("INSERT INTO wallet_snapshots (id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_equity_sol, total_tokens_count, total_nfts_count, created_at) VALUES (23, 'address', '2025-01-02T00:00:00+00:00', 1.5, 1500000000, 4.5, 4, 1, '2025-01-02')", []).unwrap();
    monitor.execute("INSERT INTO nft_balances (id, snapshot_id, mint, account_address, name, symbol, image_url, is_token_2022, created_at) VALUES (31, 23, 'nft', 'account', 'NFT', 'N', 'image', 1, '2025-01-02')", []).unwrap();
    monitor.execute("INSERT INTO wallet_metadata (key, value, updated_at) VALUES ('sentinel', 'kept', '2025-01-02')", []).unwrap();
    monitor.execute("INSERT INTO sol_flow_cache (wallet_address, signature, timestamp, sol_delta, created_at) VALUES ('address', 'signature', '2025-01-02', 1.25, '2025-01-02')", []).unwrap();
    monitor.execute("INSERT INTO wallet_dashboard_metrics (wallet_address, window_key, window_hours, snapshot_limit, token_limit, payload_blob, computed_at, valid_until, snapshot_count, flow_cache_rows) VALUES ('address', 'day', 24, 3, 4, X'010203', '2025-01-02', '2025-01-03', 1, 1)", []).unwrap();
    for (i, amount) in CASES.iter().enumerate() {
        monitor.execute("INSERT INTO token_balances (id, snapshot_id, mint, balance, balance_ui, decimals, is_token_2022, created_at) VALUES (?1, 23, ?2, ?3, 12.5, 9, 1, '2025-01-02')", params![40 + i as i64, format!("mint-{i}"), *amount as i64]).unwrap();
    }
    drop(monitor);

    boot(dir.path());
    boot(dir.path());

    let wallets = open(dir.path(), "wallets.db");
    let monitor = open(dir.path(), "wallet.db");
    for conn in [&wallets, &monitor] {
        assert_eq!(
            conn.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
    }
    for (i, amount) in CASES.iter().enumerate() {
        for (conn, table, id) in [
            (&wallets, "wallet_token_balances", "wallet_id"),
            (&monitor, "token_balances", "snapshot_id"),
        ] {
            let (stored, kind, ui, decimals, symbol): (String, String, f64, i64, String) = conn.query_row(&format!("SELECT CAST(balance AS TEXT), typeof(balance), {}, decimals, {} FROM {table} WHERE mint = ?1", if id == "wallet_id" {"ui_amount"} else {"balance_ui"}, if id == "wallet_id" {"symbol"} else {"mint"}), [format!("mint-{i}")], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))).unwrap();
            assert_eq!(stored, amount.to_string());
            assert_eq!(kind, "text");
            assert_eq!((ui, decimals), (12.5, 9));
            assert_eq!(
                symbol,
                if id == "wallet_id" {
                    "SYM".to_owned()
                } else {
                    format!("mint-{i}")
                }
            );
        }
    }
    assert_eq!(wallets.query_row("SELECT name, address, encrypted_key, nonce, role, notes, created_at FROM wallets WHERE id = 17", [], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?, r.get::<_, String>(6)?))).unwrap(), ("main".into(), "address".into(), "ciphertext".into(), "nonce".into(), "main".into(), "kept".into(), "2025-01-01".into()));
    assert_eq!(monitor.query_row("SELECT wallet_address, sol_balance, total_equity_sol, total_tokens_count, total_nfts_count FROM wallet_snapshots WHERE id = 23", [], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?, r.get::<_, i64>(3)?, r.get::<_, i64>(4)?))).unwrap(), ("address".into(), 1.5, 4.5, 4, 1));
    assert_eq!(
        monitor
            .query_row(
                "SELECT account_address, name, symbol, image_url FROM nft_balances WHERE id = 31",
                [],
                |r| Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?
                ))
            )
            .unwrap(),
        ("account".into(), "NFT".into(), "N".into(), "image".into())
    );
    assert_eq!(
        monitor
            .query_row(
                "SELECT value, updated_at FROM wallet_metadata WHERE key = 'sentinel'",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            )
            .unwrap(),
        ("kept".into(), "2025-01-02".into())
    );
    assert_eq!(
        monitor
            .query_row("SELECT signature, sol_delta FROM sol_flow_cache", [], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
            })
            .unwrap(),
        ("signature".into(), 1.25)
    );
    assert_eq!(
        monitor
            .query_row(
                "SELECT payload_blob, window_hours FROM wallet_dashboard_metrics",
                [],
                |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, i64>(1)?))
            )
            .unwrap(),
        (vec![1, 2, 3], 24)
    );

    for (conn, indexes) in [
        (
            &wallets,
            &[
                "idx_wallets_chain_address",
                "idx_wallets_role",
                "idx_wallets_active",
                "idx_token_balances_wallet",
                "idx_token_balances_mint",
            ][..],
        ),
        (
            &monitor,
            &[
                "idx_flow_cache_chain_wallet_timestamp",
                "idx_dashboard_metrics_chain_wallet_valid_until",
                "idx_wallet_snapshots_chain_address",
                "idx_wallet_snapshots_chain_address_time",
                "idx_token_balances_snapshot_id",
                "idx_token_balances_mint",
                "idx_token_balances_snapshot_mint",
                "idx_nft_balances_snapshot_id",
                "idx_nft_balances_mint",
            ][..],
        ),
    ] {
        for name in indexes {
            assert_eq!(
                conn.query_row(
                    "SELECT type FROM sqlite_master WHERE name = ?1",
                    [name],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "index",
                "missing released index {name}"
            );
        }
    }
    let wide = screenerbot::chains::RawAmount::from(u128::MAX);
    wallets.execute("INSERT INTO wallet_token_balances (wallet_id, mint, balance, ui_amount, decimals, updated_at) VALUES (17, 'wide', ?1, 0, 0, '2025-01-03')", [wide]).unwrap();
    monitor.execute("INSERT INTO token_balances (snapshot_id, mint, balance, balance_ui) VALUES (23, 'wide', ?1, 0)", [wide]).unwrap();
    for (conn, table) in [
        (&wallets, "wallet_token_balances"),
        (&monitor, "token_balances"),
    ] {
        let amount: screenerbot::chains::RawAmount = conn
            .query_row(
                &format!("SELECT balance FROM {table} WHERE mint = 'wide'"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(amount, wide);
    }
}

#[test]
fn unknown_wallet_amount_schema_leaves_original_data_and_ddl() {
    for (file, fixture, table, row) in [
        ("wallets.db", include_str!("fixtures/v0.2.13-wallets.sql"), "wallet_token_balances", "INSERT INTO wallets (id, name, address, encrypted_key, nonce, role) VALUES (17, 'main', 'address', 'ciphertext', 'nonce', 'main'); INSERT INTO wallet_token_balances (wallet_id, mint, balance, ui_amount, decimals, updated_at) VALUES (17, 'mint', -1, 1, 9, '2025-01-02');"),
        ("wallet.db", include_str!("fixtures/v0.2.13-wallet-monitor.sql"), "token_balances", "INSERT INTO wallet_snapshots (id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports) VALUES (23, 'address', '2025-01-02T00:00:00+00:00', 1, 1000000000); INSERT INTO token_balances (snapshot_id, mint, balance, balance_ui) VALUES (23, 'mint', -1, 1);"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("data")).unwrap();
        let conn = open(dir.path(), file);
        conn.execute_batch(fixture).unwrap();
        conn.execute_batch(row).unwrap();
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN unfamiliar TEXT"), []).unwrap();
        let before: (String, i64) = conn.query_row(&format!("SELECT sql, (SELECT balance FROM {table} WHERE mint = 'mint') FROM sqlite_master WHERE type = 'table' AND name = ?1"), [table], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        drop(conn);
        if file == "wallet.db" {
            let wallets = open(dir.path(), "wallets.db");
            wallets.execute_batch(include_str!("fixtures/v0.2.13-wallets.sql")).unwrap();
            wallets.execute("INSERT INTO wallets (id, name, address, encrypted_key, nonce, role) VALUES (17, 'main', 'address', 'ciphertext', 'nonce', 'main')", []).unwrap();
        }
        let status = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("v0_2_13_wallet_amount_storage_survives_two_initializers")
            .env(CHILD, "1")
            .env("SCREENERBOT_DATA_DIR", dir.path())
            .output().unwrap();
        assert!(!status.status.success(), "unfamiliar {table} shape was accepted");
        let output = format!("{}{}", String::from_utf8_lossy(&status.stdout), String::from_utf8_lossy(&status.stderr));
        assert!(output.contains("unrecognized table definition"), "unexpected failure for {table}: {output}");
        let conn = open(dir.path(), file);
        let after: (String, i64) = conn.query_row(&format!("SELECT sql, (SELECT balance FROM {table} WHERE mint = 'mint') FROM sqlite_master WHERE type = 'table' AND name = ?1"), [table], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(after, before, "rejected {table} migration changed data or DDL");
    }
}
