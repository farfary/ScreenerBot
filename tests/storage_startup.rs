//! Regression coverage for the SQLite stores opened during desktop startup.

use std::collections::BTreeSet;
use std::process::Command;

use rusqlite::Connection;
use screenerbot::chains::ChainId;

const CHILD_ENV: &str = "SCREENERBOT_STORAGE_STARTUP_CHILD";
const DATABASE_FILES: &[&str] = &[
    "actions.db",
    "agent_control.db",
    "ai.db",
    "ai_chat.db",
    "copy_trading.db",
    "events.db",
    "ohlcvs.db",
    "pools.db",
    "positions.db",
    "rpc_stats.db",
    "strategies.db",
    "tokens.db",
    "tools.db",
    "transactions.db",
    "wallet.db",
    "wallets.db",
];

#[test]
fn database_filename_inventory_matches_production_paths() {
    let mut production = database_filenames(include_str!("../src/paths/database.rs"));
    production.extend(database_filenames(include_str!(
        "../src/rpc/stats/database.rs"
    )));

    let expected = DATABASE_FILES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    assert_eq!(
        production, expected,
        "classify every production SQLite filename in the startup-storage matrix"
    );
}

#[test]
fn startup_storage_matrix_survives_two_fresh_process_launches() {
    if std::env::var_os(CHILD_ENV).is_some() {
        initialize_startup_stores();
        return;
    }

    let directory = tempfile::tempdir().expect("create isolated startup-storage directory");
    for launch in 1..=2 {
        let status = Command::new(std::env::current_exe().expect("test executable"))
            .arg("--exact")
            .arg("startup_storage_matrix_survives_two_fresh_process_launches")
            .arg("--nocapture")
            .env(CHILD_ENV, "1")
            .env("SCREENERBOT_DATA_DIR", directory.path())
            .status()
            .expect("run isolated startup-storage child");
        assert!(status.success(), "startup-storage launch {launch} failed");
    }

    for database in DATABASE_FILES {
        let path = directory.path().join("data").join(database);
        assert!(
            path.exists(),
            "production initializer did not create {database}"
        );
        let connection = Connection::open(&path).expect("reopen initialized database");
        let sentinel: String = connection
            .query_row(
                "SELECT value FROM __startup_storage_test_sentinel WHERE key = 'launches'",
                [],
                |row| row.get(0),
            )
            .expect("preserve startup-storage sentinel across relaunch");
        assert_eq!(
            sentinel, "2",
            "startup-storage sentinel was not preserved for {database}"
        );
        assert_eq!(
            connection
                .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                .expect("run integrity check"),
            "ok",
            "integrity check failed for {database}"
        );
        let foreign_key_errors: i64 = connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .expect("run foreign-key check");
        assert_eq!(
            foreign_key_errors, 0,
            "foreign-key check failed for {database}"
        );
    }
}

fn database_filenames(source: &str) -> BTreeSet<String> {
    source
        .split("join(\"")
        .skip(1)
        .filter_map(|tail| tail.split('"').next())
        .filter(|name| name.ends_with(".db"))
        .map(str::to_owned)
        .collect()
}

fn initialize_startup_stores() {
    screenerbot::paths::ensure_all_directories().expect("create startup-storage directories");
    screenerbot::config::load_config().expect("load isolated configuration");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("create multi-thread runtime");
    runtime.block_on(async {
        screenerbot::wallets::initialize()
            .await
            .expect("initialize wallets database");
        if !screenerbot::wallets::has_main_wallet().await {
            screenerbot::wallets::create_wallet(screenerbot::wallets::CreateWalletRequest {
                name: "startup-storage".to_owned(),
                notes: None,
                set_as_main: true,
            })
            .await
            .expect("create isolated main wallet");
        }

        screenerbot::actions::init_database()
            .await
            .expect("initialize actions database");
        screenerbot::events::database::EventsDatabase::new()
            .await
            .expect("initialize events database");
        screenerbot::positions::initialize_positions_database()
            .await
            .expect("initialize positions database");
        screenerbot::transactions::init_transaction_database()
            .await
            .expect("initialize transactions database");
        screenerbot::wallet::initialize_wallet_database()
            .await
            .expect("initialize wallet-monitor database");

        let mut pools = screenerbot::pools::database::PoolsDatabase::new(ChainId::Solana);
        pools.initialize().await.expect("initialize pools database");
        screenerbot::ohlcvs::OhlcvService::initialize()
            .await
            .expect("initialize OHLCV database");
        screenerbot::strategies::init_strategy_system(
            screenerbot::strategies::engine::EngineConfig::default(),
        )
        .await
        .expect("initialize strategies database");
    });

    let token_path = screenerbot::paths::get_tokens_db_path();
    screenerbot::tokens::database::TokenDatabase::new(
        &token_path.to_string_lossy(),
        ChainId::Solana,
    )
    .expect("initialize tokens database");
    screenerbot::trader::copy::CopyDatabase::new(ChainId::Solana)
        .expect("initialize copy-trading database");
    screenerbot::tools::init_tools_db().expect("initialize tools database");
    screenerbot::llm_analysis::init_analysis_database().expect("initialize analysis database");
    screenerbot::assistant::init_chat_db().expect("initialize chat database");
    screenerbot::agent_control::init_store().expect("initialize agent-control database");
    screenerbot::rpc::RpcStatsDatabase::new().expect("initialize RPC statistics database");

    for database in DATABASE_FILES {
        let path = screenerbot::paths::get_data_directory().join(database);
        let connection = Connection::open(&path).expect("open initialized database for sentinel");
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS __startup_storage_test_sentinel (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            )
            .expect("create startup-storage sentinel");
        connection
            .execute(
                "INSERT INTO __startup_storage_test_sentinel (key, value) VALUES ('launches', '1') \
                 ON CONFLICT(key) DO UPDATE SET value = CAST(value AS INTEGER) + 1",
                [],
            )
            .expect("advance startup-storage sentinel");
    }
}
