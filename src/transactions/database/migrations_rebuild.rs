//! Composite (signature, wallet_address) primary-key rebuilds for the v5
//! schema migration — the five per-table rebuild steps invoked by
//! `migrations::TransactionDatabase::migrate_signature_wallet_tables`.

use rusqlite::params;

use super::operations::TransactionDatabase;

impl TransactionDatabase {
    /// Rebuild `raw_transactions` in place: create the v5 shape, copy every row
    /// (backfilling a NULL/empty `wallet_address` with `own_wallet_address` rather
    /// than dropping the row), drop the old table, rename the new one into place.
    pub(super) fn rebuild_raw_transactions(
        tx: &rusqlite::Transaction,
        own_wallet_address: &str,
    ) -> Result<(), String> {
        if Self::has_composite_signature_wallet_key(tx, "raw_transactions")? {
            return Ok(());
        }

        tx.execute(
            "CREATE TABLE raw_transactions__v5 (
                signature TEXT NOT NULL,
                wallet_address TEXT NOT NULL,
                slot INTEGER,
                block_time INTEGER,
                timestamp TEXT NOT NULL,
                status TEXT NOT NULL,
                success BOOLEAN NOT NULL DEFAULT false,
                error_message TEXT,
                fee_lamports INTEGER,
                compute_units_consumed INTEGER,
                instructions_count INTEGER NOT NULL DEFAULT 0,
                accounts_count INTEGER NOT NULL DEFAULT 0,
                raw_transaction_data TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (signature, wallet_address)
            )",
            [],
        )
        .map_err(|e| format!("Failed to create raw_transactions__v5: {e}"))?;

        tx.execute(
            "INSERT INTO raw_transactions__v5
                (signature, wallet_address, slot, block_time, timestamp, status, success,
                 error_message, fee_lamports, compute_units_consumed, instructions_count,
                 accounts_count, raw_transaction_data, created_at, updated_at)
             SELECT signature, COALESCE(NULLIF(wallet_address, ''), ?1), slot, block_time,
                    timestamp, status, success, error_message, fee_lamports,
                    compute_units_consumed, instructions_count, accounts_count,
                    raw_transaction_data, created_at, updated_at
             FROM raw_transactions",
            params![own_wallet_address],
        )
        .map_err(|e| format!("Failed to copy raw_transactions rows into v5 shape: {e}"))?;

        tx.execute("DROP TABLE raw_transactions", [])
            .map_err(|e| format!("Failed to drop old raw_transactions: {e}"))?;
        tx.execute(
            "ALTER TABLE raw_transactions__v5 RENAME TO raw_transactions",
            [],
        )
        .map_err(|e| format!("Failed to rename raw_transactions__v5: {e}"))?;

        Ok(())
    }

    /// Rebuild `processed_transactions` in place. Same shape as
    /// `rebuild_raw_transactions`; its foreign key is updated to the composite parent
    /// key in the same change.
    pub(super) fn rebuild_processed_transactions(
        tx: &rusqlite::Transaction,
        own_wallet_address: &str,
    ) -> Result<(), String> {
        if Self::has_composite_signature_wallet_key(tx, "processed_transactions")? {
            return Ok(());
        }

        tx.execute(
            "CREATE TABLE processed_transactions__v5 (
                signature TEXT NOT NULL,
                wallet_address TEXT NOT NULL,
                transaction_type TEXT NOT NULL,
                direction TEXT NOT NULL,
                sol_balance_change TEXT,
                token_balance_changes TEXT,
                token_swap_info TEXT,
                swap_pnl_info TEXT,
                ata_operations TEXT,
                token_transfers TEXT,
                instruction_info TEXT,
                analysis_duration_ms INTEGER,
                cached_analysis TEXT,
                analysis_version INTEGER NOT NULL DEFAULT 2,
                fee_sol REAL NOT NULL DEFAULT 0,
                sol_delta REAL,
                processed_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (signature, wallet_address),
                FOREIGN KEY (signature, wallet_address)
                    REFERENCES raw_transactions(signature, wallet_address) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| format!("Failed to create processed_transactions__v5: {e}"))?;

        tx.execute(
            "INSERT INTO processed_transactions__v5
                (signature, wallet_address, transaction_type, direction, sol_balance_change,
                 token_balance_changes, token_swap_info, swap_pnl_info, ata_operations,
                 token_transfers, instruction_info, analysis_duration_ms, cached_analysis,
                 analysis_version, fee_sol, sol_delta, processed_at, updated_at)
             SELECT signature, COALESCE(NULLIF(wallet_address, ''), ?1), transaction_type,
                    direction, sol_balance_change, token_balance_changes, token_swap_info,
                    swap_pnl_info, ata_operations, token_transfers, instruction_info,
                    analysis_duration_ms, cached_analysis, analysis_version, fee_sol,
                    sol_delta, processed_at, updated_at
             FROM processed_transactions",
            params![own_wallet_address],
        )
        .map_err(|e| format!("Failed to copy processed_transactions rows into v5 shape: {e}"))?;

        tx.execute("DROP TABLE processed_transactions", [])
            .map_err(|e| format!("Failed to drop old processed_transactions: {e}"))?;
        tx.execute(
            "ALTER TABLE processed_transactions__v5 RENAME TO processed_transactions",
            [],
        )
        .map_err(|e| format!("Failed to rename processed_transactions__v5: {e}"))?;

        Ok(())
    }

    /// Rebuild `known_signatures` in place (no foreign key, no extra columns).
    pub(super) fn rebuild_known_signatures(
        tx: &rusqlite::Transaction,
        own_wallet_address: &str,
    ) -> Result<(), String> {
        if Self::has_composite_signature_wallet_key(tx, "known_signatures")? {
            return Ok(());
        }

        tx.execute(
            "CREATE TABLE known_signatures__v5 (
                signature TEXT NOT NULL,
                wallet_address TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'known',
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (signature, wallet_address)
            )",
            [],
        )
        .map_err(|e| format!("Failed to create known_signatures__v5: {e}"))?;

        tx.execute(
            "INSERT INTO known_signatures__v5 (signature, wallet_address, status, added_at)
             SELECT signature, COALESCE(NULLIF(wallet_address, ''), ?1), status, added_at
             FROM known_signatures",
            params![own_wallet_address],
        )
        .map_err(|e| format!("Failed to copy known_signatures rows into v5 shape: {e}"))?;

        tx.execute("DROP TABLE known_signatures", [])
            .map_err(|e| format!("Failed to drop old known_signatures: {e}"))?;
        tx.execute(
            "ALTER TABLE known_signatures__v5 RENAME TO known_signatures",
            [],
        )
        .map_err(|e| format!("Failed to rename known_signatures__v5: {e}"))?;

        Ok(())
    }

    /// Rebuild `pending_transactions` in place (no foreign key, no extra columns).
    pub(super) fn rebuild_pending_transactions(
        tx: &rusqlite::Transaction,
        own_wallet_address: &str,
    ) -> Result<(), String> {
        if Self::has_composite_signature_wallet_key(tx, "pending_transactions")? {
            return Ok(());
        }

        tx.execute(
            "CREATE TABLE pending_transactions__v5 (
                signature TEXT NOT NULL,
                wallet_address TEXT NOT NULL,
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_checked_at TEXT,
                check_count INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (signature, wallet_address)
            )",
            [],
        )
        .map_err(|e| format!("Failed to create pending_transactions__v5: {e}"))?;

        tx.execute(
            "INSERT INTO pending_transactions__v5
                (signature, wallet_address, added_at, last_checked_at, check_count)
             SELECT signature, COALESCE(NULLIF(wallet_address, ''), ?1), added_at,
                    last_checked_at, check_count
             FROM pending_transactions",
            params![own_wallet_address],
        )
        .map_err(|e| format!("Failed to copy pending_transactions rows into v5 shape: {e}"))?;

        tx.execute("DROP TABLE pending_transactions", [])
            .map_err(|e| format!("Failed to drop old pending_transactions: {e}"))?;
        tx.execute(
            "ALTER TABLE pending_transactions__v5 RENAME TO pending_transactions",
            [],
        )
        .map_err(|e| format!("Failed to rename pending_transactions__v5: {e}"))?;

        Ok(())
    }

    /// Rebuild `deferred_retries` in place. Unlike the other four tables this one has
    /// no `wallet_address` column at all pre-v5, so every existing row is attributed to
    /// the own wallet outright rather than backfilled from a NULL/empty value.
    pub(super) fn rebuild_deferred_retries(
        tx: &rusqlite::Transaction,
        own_wallet_address: &str,
    ) -> Result<(), String> {
        if Self::has_composite_signature_wallet_key(tx, "deferred_retries")? {
            return Ok(());
        }

        tx.execute(
            "CREATE TABLE deferred_retries__v5 (
                signature TEXT NOT NULL,
                wallet_address TEXT NOT NULL,
                next_retry_at TEXT NOT NULL,
                remaining_attempts INTEGER NOT NULL DEFAULT 3,
                current_delay_secs INTEGER NOT NULL DEFAULT 60,
                last_error TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (signature, wallet_address)
            )",
            [],
        )
        .map_err(|e| format!("Failed to create deferred_retries__v5: {e}"))?;

        tx.execute(
            "INSERT INTO deferred_retries__v5
                (signature, wallet_address, next_retry_at, remaining_attempts,
                 current_delay_secs, last_error, created_at, updated_at)
             SELECT signature, ?1, next_retry_at, remaining_attempts, current_delay_secs,
                    last_error, created_at, updated_at
             FROM deferred_retries",
            params![own_wallet_address],
        )
        .map_err(|e| format!("Failed to copy deferred_retries rows into v5 shape: {e}"))?;

        tx.execute("DROP TABLE deferred_retries", [])
            .map_err(|e| format!("Failed to drop old deferred_retries: {e}"))?;
        tx.execute(
            "ALTER TABLE deferred_retries__v5 RENAME TO deferred_retries",
            [],
        )
        .map_err(|e| format!("Failed to rename deferred_retries__v5: {e}"))?;

        Ok(())
    }
}
