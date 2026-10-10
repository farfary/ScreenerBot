// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! File-backed upgrade coverage for the released integer subject-delta schema.

mod common;

use rusqlite::{params, Connection};
use screenerbot::chains::ChainId;
use screenerbot::transactions::TransactionDatabase;

const LEGACY_SCHEMA: &str = include_str!("fixtures/v0.2.13-transactions.sql");

fn replace_subject_table(conn: &mut Connection, table_ddl: &str) {
    let tx = conn.transaction().unwrap();
    tx.execute_batch("ALTER TABLE subject_asset_deltas RENAME TO subject_asset_deltas_previous;")
        .unwrap();
    tx.execute_batch(table_ddl).unwrap();
    tx.execute(
        "INSERT INTO subject_asset_deltas (chain_id, wallet_address, signature, mint, slot, block_time, tx_index, delta_raw, before_raw, after_raw, decimals, kind, venue, fee_lamports, success)
         SELECT chain_id, wallet_address, signature, mint, slot, block_time, tx_index, delta_raw, before_raw, after_raw, decimals, kind, venue, fee_lamports, success FROM subject_asset_deltas_previous",
        [],
    ).unwrap();
    tx.execute_batch("DROP TABLE subject_asset_deltas_previous;")
        .unwrap();
    tx.execute_batch(
        "CREATE INDEX idx_subject_deltas_chain_wallet_mint ON subject_asset_deltas(chain_id, wallet_address, mint, slot, tx_index);
         CREATE INDEX idx_subject_deltas_chain_wallet_order ON subject_asset_deltas(chain_id, wallet_address, slot, tx_index, signature);",
    ).unwrap();
    tx.commit().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn released_subject_deltas_migrate_losslessly_and_reopen_idempotently() {
    let _guard = common::isolated_env();
    common::configure_own_wallet();
    let path = screenerbot::paths::get_transactions_db_path();
    let conn = common::seed_store(&path, screenerbot::database::TRANSACTIONS_DB, LEGACY_SCHEMA);
    conn.execute(
        "INSERT INTO db_metadata (key, value) VALUES ('schema_version', '7')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO db_metadata (key, value) VALUES ('sentinel', 'retained')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO subject_asset_deltas (chain_id, wallet_address, signature, mint, slot, block_time, tx_index, delta_raw, before_raw, after_raw, decimals, kind, venue, fee_lamports, success)
         VALUES ('solana', 'wallet-a', 'shared', 'mint-a', 123, 456, 2, ?1, ?2, ?3, 9, 'trade', 'raydium', 5000, 1)",
        params![i64::MAX, i64::MAX, 0i64],
    ).unwrap();
    conn.execute(
        "INSERT INTO subject_asset_deltas (chain_id, wallet_address, signature, mint, slot, block_time, tx_index, delta_raw, before_raw, after_raw, decimals, kind, venue, fee_lamports, success)
         VALUES ('solana', 'wallet-b', 'shared', 'mint-a', NULL, NULL, 0, -42, NULL, NULL, 6, 'transfer', NULL, NULL, 0)",
        [],
    ).unwrap();
    drop(conn);

    // Invalid legacy values must not advance the version or replace the table.
    let conn = Connection::open(&path).unwrap();
    conn.execute(
        "UPDATE subject_asset_deltas SET before_raw = -1 WHERE wallet_address = 'wallet-a'",
        [],
    )
    .unwrap();
    drop(conn);
    assert!(TransactionDatabase::new(ChainId::Solana).await.is_err());
    let conn = Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT before_raw FROM subject_asset_deltas WHERE wallet_address = 'wallet-a'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        -1
    );
    assert_eq!(
        conn.query_row(
            "SELECT value FROM db_metadata WHERE key = 'schema_version'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "7"
    );
    assert_eq!(
        conn.query_row(
            "SELECT type FROM pragma_table_info('subject_asset_deltas') WHERE name = 'delta_raw'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "INTEGER"
    );
    conn.execute(
        "UPDATE subject_asset_deltas SET before_raw = ?1 WHERE wallet_address = 'wallet-a'",
        [i64::MAX],
    )
    .unwrap();

    // Unknown columns cannot be discarded by rebuilding into the canonical shape.
    conn.execute(
        "ALTER TABLE subject_asset_deltas ADD COLUMN external_tag TEXT",
        [],
    )
    .unwrap();
    conn.execute("UPDATE subject_asset_deltas SET external_tag = 'preserve-me' WHERE wallet_address = 'wallet-a'", []).unwrap();
    drop(conn);
    assert!(TransactionDatabase::new(ChainId::Solana).await.is_err());
    let conn = Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT external_tag FROM subject_asset_deltas WHERE wallet_address = 'wallet-a'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "preserve-me"
    );
    assert_eq!(
        conn.query_row(
            "SELECT value FROM db_metadata WHERE key = 'schema_version'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "7"
    );
    assert_eq!(
        conn.query_row(
            "SELECT type FROM pragma_table_info('subject_asset_deltas') WHERE name = 'delta_raw'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "INTEGER"
    );
    conn.execute(
        "ALTER TABLE subject_asset_deltas DROP COLUMN external_tag",
        [],
    )
    .unwrap();
    drop(conn);

    let table_start = LEGACY_SCHEMA
        .find("CREATE TABLE IF NOT EXISTS subject_asset_deltas")
        .unwrap();
    let table_end = LEGACY_SCHEMA[table_start..]
        .find("\n\nCREATE INDEX")
        .unwrap()
        + table_start;
    let released_table_ddl = &LEGACY_SCHEMA[table_start..table_end];

    // Generated columns are absent from table_info; the source DDL gate sees them.
    let conn = Connection::open(&path).unwrap();
    conn.execute("ALTER TABLE subject_asset_deltas ADD COLUMN derived_mint TEXT GENERATED ALWAYS AS (mint) VIRTUAL", []).unwrap();
    let generated_ddl: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'subject_asset_deltas'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    drop(conn);
    assert!(TransactionDatabase::new(ChainId::Solana).await.is_err());
    let mut conn = Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'subject_asset_deltas'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        generated_ddl
    );
    assert_eq!(
        conn.query_row(
            "SELECT derived_mint FROM subject_asset_deltas WHERE wallet_address = 'wallet-a'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "mint-a"
    );
    assert_eq!(
        conn.query_row(
            "SELECT value FROM db_metadata WHERE key = 'schema_version'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "7"
    );
    replace_subject_table(&mut conn, released_table_ddl);

    // A table-level constraint is also part of the schema being replaced.
    let checked_table_ddl = released_table_ddl.replace(
        "    PRIMARY KEY (chain_id, wallet_address, signature, mint)",
        "    CHECK (length(mint) > 0),\n    PRIMARY KEY (chain_id, wallet_address, signature, mint)",
    );
    replace_subject_table(&mut conn, &checked_table_ddl);
    let checked_ddl: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'subject_asset_deltas'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    drop(conn);
    assert!(TransactionDatabase::new(ChainId::Solana).await.is_err());
    let mut conn = Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'subject_asset_deltas'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        checked_ddl
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM subject_asset_deltas", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        conn.query_row(
            "SELECT value FROM db_metadata WHERE key = 'schema_version'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "7"
    );
    replace_subject_table(&mut conn, released_table_ddl);
    drop(conn);

    let db = TransactionDatabase::new(ChainId::Solana).await.unwrap();
    let conn = Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT value FROM db_metadata WHERE key = 'schema_version'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "8"
    );
    assert_eq!(
        conn.query_row(
            "SELECT value FROM db_metadata WHERE key = 'sentinel'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "retained"
    );
    let rows: Vec<serde_json::Value> = conn.prepare(
        "SELECT json_array(chain_id, wallet_address, signature, mint, slot, block_time, tx_index, delta_raw, before_raw, after_raw, decimals, kind, venue, fee_raw, success, typeof(delta_raw), typeof(before_raw), typeof(after_raw)) FROM subject_asset_deltas ORDER BY wallet_address"
    ).unwrap().query_map([], |r| r.get::<_, String>(0)).unwrap()
        .map(|row| serde_json::from_str(&row.unwrap()).unwrap()).collect();
    assert_eq!(
        rows,
        vec![
            serde_json::json!([
                "solana",
                "wallet-a",
                "shared",
                "mint-a",
                123,
                456,
                2,
                i64::MAX.to_string(),
                i64::MAX.to_string(),
                "0",
                9,
                "trade",
                "raydium",
                5000,
                1,
                "text",
                "text",
                "text"
            ]),
            serde_json::json!([
                "solana", "wallet-b", "shared", "mint-a", null, null, 0, "-42", null, null, 6,
                "transfer", null, null, 0, "text", "null", "null"
            ]),
        ]
    );
    let indexes: Vec<String> = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'subject_asset_deltas' AND name NOT LIKE 'sqlite_autoindex_%' ORDER BY name"
    ).unwrap().query_map([], |r| r.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
    assert_eq!(
        indexes,
        [
            "idx_subject_deltas_chain_wallet_mint",
            "idx_subject_deltas_chain_wallet_order"
        ]
    );
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
    drop(conn);
    drop(db);

    let _db_again = TransactionDatabase::new(ChainId::Solana).await.unwrap();
    let conn = Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM subject_asset_deltas", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        conn.query_row(
            "SELECT typeof(delta_raw) FROM subject_asset_deltas WHERE wallet_address = 'wallet-a'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "text"
    );
}
