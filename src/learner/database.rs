//! learner/database.rs
//!
//! SQLite database management for the learning system.
//!
//! This module handles all database operations for storing and retrieving:
//! * Trade records from completed positions
//! * Feature vectors extracted from trades
//! * Model weights and training metadata
//! * Trading patterns and similarity data
//!
//! Design principles:
//! * Append-only for thread safety
//! * Indexed for fast queries
//! * Backward compatible schema evolution
//! * Automatic cleanup of old data

use crate::global::is_debug_learning_enabled;
use crate::learner::types::*;
use crate::logger::{log, LogTag};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Learning database manager
pub struct LearningDatabase {
    connection: Arc<Mutex<Connection>>,
}

impl LearningDatabase {
    /// Create new learning database instance
    pub async fn new() -> Result<Self, String> {
        let db_path = "data/learning.db";

        // Ensure data directory exists
        if let Some(parent) = Path::new(db_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let connection = Connection::open(db_path)
            .map_err(|e| format!("Failed to open learning database: {}", e))?;

        let db = Self {
            connection: Arc::new(Mutex::new(connection)),
        };

        // Initialize schema
        db.initialize_schema().await?;

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                &format!("Learning database initialized: {}", db_path),
            );
        }

        Ok(db)
    }

    /// Initialize database schema
    async fn initialize_schema(&self) -> Result<(), String> {
        let conn = self.connection.lock().await;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;

        // Create trades table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mint TEXT NOT NULL,
                symbol TEXT NOT NULL,
                name TEXT NOT NULL,
                
                entry_time TEXT NOT NULL,
                exit_time TEXT NOT NULL,
                entry_price REAL NOT NULL,
                exit_price REAL NOT NULL,
                hold_duration_sec INTEGER NOT NULL,
                
                pnl_pct REAL NOT NULL,
                max_up_pct REAL NOT NULL,
                max_down_pct REAL NOT NULL,
                peak_reached_sec INTEGER,
                dd_reached_sec INTEGER,
                
                entry_size_sol REAL NOT NULL,
                token_amount INTEGER,
                liquidity_at_entry REAL,
                sol_reserves_at_entry REAL,
                
                tx_activity_5m INTEGER,
                tx_activity_1h INTEGER,
                security_score INTEGER,
                holder_count INTEGER,
                
                drop_10s_pct REAL,
                drop_30s_pct REAL,
                drop_60s_pct REAL,
                drop_120s_pct REAL,
                drop_320s_pct REAL,
                
                ath_dist_15m_pct REAL,
                ath_dist_1h_pct REAL,
                ath_dist_6h_pct REAL,
                
                hour_of_day INTEGER NOT NULL,
                day_of_week INTEGER NOT NULL,
                
                was_re_entry BOOLEAN NOT NULL DEFAULT 0,
                phantom_exit BOOLEAN NOT NULL DEFAULT 0,
                forced_exit BOOLEAN NOT NULL DEFAULT 0,
                
                features_extracted BOOLEAN NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| format!("Failed to create trades table: {}", e))?;

        // Create features table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS features (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                trade_id INTEGER NOT NULL,
                
                drop_10s_norm REAL NOT NULL,
                drop_30s_norm REAL NOT NULL,
                drop_60s_norm REAL NOT NULL,
                drop_120s_norm REAL NOT NULL,
                drop_320s_norm REAL NOT NULL,
                drop_velocity_30s REAL NOT NULL,
                drop_acceleration REAL NOT NULL,
                
                liquidity_tier REAL NOT NULL,
                tx_activity_score REAL NOT NULL,
                security_score_norm REAL NOT NULL,
                holder_count_log REAL NOT NULL,
                market_cap_tier REAL NOT NULL,
                
                ath_prox_15m REAL NOT NULL,
                ath_prox_1h REAL NOT NULL,
                ath_prox_6h REAL NOT NULL,
                ath_risk_score REAL NOT NULL,
                
                hour_sin REAL NOT NULL,
                hour_cos REAL NOT NULL,
                day_sin REAL NOT NULL,
                day_cos REAL NOT NULL,
                
                re_entry_flag REAL NOT NULL,
                token_trade_count REAL NOT NULL,
                recent_exit_count REAL NOT NULL,
                avg_hold_duration REAL NOT NULL,
                
                success_label REAL,
                quick_success_label REAL,
                risk_label REAL,
                peak_time_label REAL,
                
                created_at TEXT NOT NULL,
                
                FOREIGN KEY (trade_id) REFERENCES trades (id) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| format!("Failed to create features table: {}", e))?;

        // Create models table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS models (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                version INTEGER NOT NULL,
                model_type TEXT NOT NULL,
                
                success_weights TEXT NOT NULL,  -- JSON array
                success_intercept REAL NOT NULL,
                success_threshold REAL NOT NULL,
                
                risk_weights TEXT NOT NULL,     -- JSON array
                risk_intercept REAL NOT NULL,
                risk_threshold REAL NOT NULL,
                
                training_samples INTEGER NOT NULL,
                validation_accuracy REAL NOT NULL,
                feature_importance TEXT NOT NULL, -- JSON array
                
                created_at TEXT NOT NULL,
                trained_on_trades INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| format!("Failed to create models table: {}", e))?;

        // Create patterns table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS patterns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pattern_id TEXT UNIQUE NOT NULL,
                pattern_type TEXT NOT NULL,
                confidence REAL NOT NULL,
                
                drop_sequence TEXT NOT NULL,    -- JSON array
                duration_min INTEGER NOT NULL,
                duration_max INTEGER NOT NULL,
                success_rate REAL NOT NULL,
                avg_profit REAL NOT NULL,
                avg_duration INTEGER NOT NULL,
                sample_count INTEGER NOT NULL,
                
                liquidity_min REAL,
                tx_activity_min INTEGER,
                ath_distance_max REAL,
                
                last_updated TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| format!("Failed to create patterns table: {}", e))?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trades_mint ON trades(mint)",
            [],
        )
        .map_err(|e| format!("Failed to create trades mint index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trades_created_at ON trades(created_at)",
            [],
        )
        .map_err(|e| format!("Failed to create trades created_at index: {}", e))?;

        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_trades_features_extracted ON trades(features_extracted)",
                []
            )
            .map_err(|e| format!("Failed to create trades features_extracted index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_features_trade_id ON features(trade_id)",
            [],
        )
        .map_err(|e| format!("Failed to create features trade_id index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_models_version ON models(version)",
            [],
        )
        .map_err(|e| format!("Failed to create models version index: {}", e))?;

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                "Learning database schema initialized",
            );
        }

        Ok(())
    }

    /// Store a completed trade record
    pub async fn store_trade(&self, trade: &TradeRecord) -> Result<i64, String> {
        let conn = self.connection.lock().await;

        let trade_id = conn
            .execute(
                "INSERT INTO trades (
                mint, symbol, name,
                entry_time, exit_time, entry_price, exit_price, hold_duration_sec,
                pnl_pct, max_up_pct, max_down_pct, peak_reached_sec, dd_reached_sec,
                entry_size_sol, token_amount, liquidity_at_entry, sol_reserves_at_entry,
                tx_activity_5m, tx_activity_1h, security_score, holder_count,
                drop_10s_pct, drop_30s_pct, drop_60s_pct, drop_120s_pct, drop_320s_pct,
                ath_dist_15m_pct, ath_dist_1h_pct, ath_dist_6h_pct,
                hour_of_day, day_of_week,
                was_re_entry, phantom_exit, forced_exit,
                features_extracted, created_at
            ) VALUES (
                ?1, ?2, ?3,
                ?4, ?5, ?6, ?7, ?8,
                ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21,
                ?22, ?23, ?24, ?25, ?26,
                ?27, ?28, ?29,
                ?30, ?31,
                ?32, ?33, ?34,
                ?35, ?36
            )",
                params![
                    trade.mint,
                    trade.symbol,
                    trade.name,
                    trade.entry_time.to_rfc3339(),
                    trade.exit_time.to_rfc3339(),
                    trade.entry_price,
                    trade.exit_price,
                    trade.hold_duration_sec,
                    trade.pnl_pct,
                    trade.max_up_pct,
                    trade.max_down_pct,
                    trade.peak_reached_sec,
                    trade.dd_reached_sec,
                    trade.entry_size_sol,
                    trade.token_amount,
                    trade.liquidity_at_entry,
                    trade.sol_reserves_at_entry,
                    trade.tx_activity_5m,
                    trade.tx_activity_1h,
                    trade.security_score,
                    trade.holder_count,
                    trade.drop_10s_pct,
                    trade.drop_30s_pct,
                    trade.drop_60s_pct,
                    trade.drop_120s_pct,
                    trade.drop_320s_pct,
                    trade.ath_dist_15m_pct,
                    trade.ath_dist_1h_pct,
                    trade.ath_dist_6h_pct,
                    trade.hour_of_day,
                    trade.day_of_week,
                    trade.was_re_entry,
                    trade.phantom_exit,
                    trade.forced_exit,
                    trade.features_extracted,
                    trade.created_at.to_rfc3339()
                ],
            )
            .map_err(|e| format!("Failed to store trade: {}", e))?;

        let trade_id = conn.last_insert_rowid();

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                &format!(
                    "Stored trade {}: {} {} -> {:.2}%",
                    trade_id, trade.symbol, trade.mint, trade.pnl_pct
                ),
            );
        }

        Ok(trade_id)
    }

    /// Store feature vector
    pub async fn store_features(&self, features: &FeatureVector) -> Result<(), String> {
        let conn = self.connection.lock().await;

        conn
            .execute(
                "INSERT INTO features (
                trade_id,
                drop_10s_norm, drop_30s_norm, drop_60s_norm, drop_120s_norm, drop_320s_norm,
                drop_velocity_30s, drop_acceleration,
                liquidity_tier, tx_activity_score, security_score_norm, holder_count_log, market_cap_tier,
                ath_prox_15m, ath_prox_1h, ath_prox_6h, ath_risk_score,
                hour_sin, hour_cos, day_sin, day_cos,
                re_entry_flag, token_trade_count, recent_exit_count, avg_hold_duration,
                success_label, quick_success_label, risk_label, peak_time_label,
                created_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25,
                ?26, ?27, ?28, ?29, ?30
            )",
                params![
                    features.trade_id,
                    features.drop_10s_norm,
                    features.drop_30s_norm,
                    features.drop_60s_norm,
                    features.drop_120s_norm,
                    features.drop_320s_norm,
                    features.drop_velocity_30s,
                    features.drop_acceleration,
                    features.liquidity_tier,
                    features.tx_activity_score,
                    features.security_score_norm,
                    features.holder_count_log,
                    features.market_cap_tier,
                    features.ath_prox_15m,
                    features.ath_prox_1h,
                    features.ath_prox_6h,
                    features.ath_risk_score,
                    features.hour_sin,
                    features.hour_cos,
                    features.day_sin,
                    features.day_cos,
                    features.re_entry_flag,
                    features.token_trade_count,
                    features.recent_exit_count,
                    features.avg_hold_duration,
                    features.success_label,
                    features.quick_success_label,
                    features.risk_label,
                    features.peak_time_label,
                    features.created_at.to_rfc3339()
                ]
            )
            .map_err(|e| format!("Failed to store features: {}", e))?;

        Ok(())
    }

    /// Store model weights
    pub async fn store_model_weights(&self, weights: &ModelWeights) -> Result<(), String> {
        let conn = self.connection.lock().await;

        let success_weights_json = serde_json::to_string(&weights.success_weights)
            .map_err(|e| format!("Failed to serialize success weights: {}", e))?;
        let risk_weights_json = serde_json::to_string(&weights.risk_weights)
            .map_err(|e| format!("Failed to serialize risk weights: {}", e))?;
        let feature_importance_json = serde_json::to_string(&weights.feature_importance)
            .map_err(|e| format!("Failed to serialize feature importance: {}", e))?;

        conn.execute(
            "INSERT INTO models (
                version, model_type,
                success_weights, success_intercept, success_threshold,
                risk_weights, risk_intercept, risk_threshold,
                training_samples, validation_accuracy, feature_importance,
                created_at, trained_on_trades
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13
            )",
            params![
                weights.version,
                weights.model_type,
                success_weights_json,
                weights.success_intercept,
                weights.success_threshold,
                risk_weights_json,
                weights.risk_intercept,
                weights.risk_threshold,
                weights.training_samples,
                weights.validation_accuracy,
                feature_importance_json,
                weights.created_at.to_rfc3339(),
                weights.trained_on_trades
            ],
        )
        .map_err(|e| format!("Failed to store model weights: {}", e))?;

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                &format!(
                    "Stored model weights v{}: {} samples, {:.2}% accuracy",
                    weights.version,
                    weights.training_samples,
                    weights.validation_accuracy * 100.0
                ),
            );
        }

        Ok(())
    }

    /// Get trades that haven't had features extracted
    pub async fn get_unprocessed_trades(&self) -> Result<Vec<TradeRecord>, String> {
        let conn = self.connection.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT * FROM trades WHERE features_extracted = 0 ORDER BY created_at ASC LIMIT 100"
            )
            .map_err(|e| format!("Failed to prepare unprocessed trades query: {}", e))?;

        let trade_iter = stmt
            .query_map([], |row| Ok(self.row_to_trade_record(row)?))
            .map_err(|e| format!("Failed to query unprocessed trades: {}", e))?;

        let mut trades = Vec::new();
        for trade_result in trade_iter {
            trades.push(trade_result.map_err(|e| format!("Failed to parse trade record: {}", e))?);
        }

        Ok(trades)
    }

    /// Mark trade as having features extracted
    pub async fn mark_trade_processed(&self, trade_id: i64) -> Result<(), String> {
        let conn = self.connection.lock().await;

        conn.execute(
            "UPDATE trades SET features_extracted = 1 WHERE id = ?1",
            params![trade_id],
        )
        .map_err(|e| format!("Failed to mark trade as processed: {}", e))?;

        Ok(())
    }

    /// Get all features for model training
    pub async fn get_all_features(&self) -> Result<Vec<FeatureVector>, String> {
        let conn = self.connection.lock().await;

        let mut stmt = conn
            .prepare("SELECT * FROM features ORDER BY created_at ASC")
            .map_err(|e| format!("Failed to prepare features query: {}", e))?;

        let feature_iter = stmt
            .query_map([], |row| Ok(self.row_to_feature_vector(row)?))
            .map_err(|e| format!("Failed to query features: {}", e))?;

        let mut features = Vec::new();
        for feature_result in feature_iter {
            features.push(
                feature_result.map_err(|e| format!("Failed to parse feature vector: {}", e))?,
            );
        }

        Ok(features)
    }

    /// Get latest model weights
    pub async fn get_latest_model_weights(&self) -> Result<Option<ModelWeights>, String> {
        let conn = self.connection.lock().await;

        let result = conn
            .query_row(
                "SELECT * FROM models ORDER BY version DESC LIMIT 1",
                [],
                |row| Ok(self.row_to_model_weights(row)?),
            )
            .optional()
            .map_err(|e| format!("Failed to query latest model weights: {}", e))?;

        Ok(result)
    }

    /// Get total trade count
    pub async fn get_total_trade_count(&self) -> Result<usize, String> {
        let conn = self.connection.lock().await;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM trades", [], |row| row.get(0))
            .map_err(|e| format!("Failed to query trade count: {}", e))?;

        Ok(count as usize)
    }

    /// Get new trades since timestamp
    pub async fn get_new_trades_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<TradeRecord>, String> {
        let conn = self.connection.lock().await;

        let mut stmt = conn
            .prepare("SELECT * FROM trades WHERE created_at > ?1 ORDER BY created_at ASC")
            .map_err(|e| format!("Failed to prepare new trades query: {}", e))?;

        let trade_iter = stmt
            .query_map(params![since.to_rfc3339()], |row| {
                Ok(self.row_to_trade_record(row)?)
            })
            .map_err(|e| format!("Failed to query new trades: {}", e))?;

        let mut trades = Vec::new();
        for trade_result in trade_iter {
            trades.push(trade_result.map_err(|e| format!("Failed to parse trade record: {}", e))?);
        }

        Ok(trades)
    }

    /// Get trades for specific mint
    pub async fn get_trades_for_mint(&self, mint: &str) -> Result<Vec<TradeRecord>, String> {
        let conn = self.connection.lock().await;

        let mut stmt = conn
            .prepare("SELECT * FROM trades WHERE mint = ?1 ORDER BY created_at DESC")
            .map_err(|e| format!("Failed to prepare mint trades query: {}", e))?;

        let trade_iter = stmt
            .query_map(params![mint], |row| Ok(self.row_to_trade_record(row)?))
            .map_err(|e| format!("Failed to query mint trades: {}", e))?;

        let mut trades = Vec::new();
        for trade_result in trade_iter {
            trades.push(trade_result.map_err(|e| format!("Failed to parse trade record: {}", e))?);
        }

        Ok(trades)
    }

    /// Get all unique mints from the trades table
    pub async fn get_all_unique_mints(&self) -> Result<Vec<String>, String> {
        let conn = self.connection.lock().await;

        let mut stmt = conn
            .prepare("SELECT DISTINCT mint FROM trades ORDER BY mint")
            .map_err(|e| format!("Failed to prepare unique mints query: {}", e))?;

        let mint_iter = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Failed to query unique mints: {}", e))?;

        let mut mints = Vec::new();
        for mint_result in mint_iter {
            mints.push(mint_result.map_err(|e| format!("Failed to parse mint: {}", e))?);
        }

        Ok(mints)
    }

    /// Helper: Convert database row to TradeRecord
    fn row_to_trade_record(&self, row: &Row) -> Result<TradeRecord, rusqlite::Error> {
        Ok(TradeRecord {
            id: row.get("id")?,
            mint: row.get("mint")?,
            symbol: row.get("symbol")?,
            name: row.get("name")?,
            entry_time: DateTime::parse_from_rfc3339(&row.get::<_, String>("entry_time")?)
                .map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "entry_time".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?
                .with_timezone(&Utc),
            exit_time: DateTime::parse_from_rfc3339(&row.get::<_, String>("exit_time")?)
                .map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "exit_time".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?
                .with_timezone(&Utc),
            entry_price: row.get("entry_price")?,
            exit_price: row.get("exit_price")?,
            hold_duration_sec: row.get("hold_duration_sec")?,
            pnl_pct: row.get("pnl_pct")?,
            max_up_pct: row.get("max_up_pct")?,
            max_down_pct: row.get("max_down_pct")?,
            peak_reached_sec: row.get("peak_reached_sec")?,
            dd_reached_sec: row.get("dd_reached_sec")?,
            entry_size_sol: row.get("entry_size_sol")?,
            token_amount: row.get("token_amount")?,
            liquidity_at_entry: row.get("liquidity_at_entry")?,
            sol_reserves_at_entry: row.get("sol_reserves_at_entry")?,
            tx_activity_5m: row.get("tx_activity_5m")?,
            tx_activity_1h: row.get("tx_activity_1h")?,
            security_score: row.get("security_score")?,
            holder_count: row.get("holder_count")?,
            drop_10s_pct: row.get("drop_10s_pct")?,
            drop_30s_pct: row.get("drop_30s_pct")?,
            drop_60s_pct: row.get("drop_60s_pct")?,
            drop_120s_pct: row.get("drop_120s_pct")?,
            drop_320s_pct: row.get("drop_320s_pct")?,
            ath_dist_15m_pct: row.get("ath_dist_15m_pct")?,
            ath_dist_1h_pct: row.get("ath_dist_1h_pct")?,
            ath_dist_6h_pct: row.get("ath_dist_6h_pct")?,
            hour_of_day: row.get("hour_of_day")?,
            day_of_week: row.get("day_of_week")?,
            was_re_entry: row.get("was_re_entry")?,
            phantom_exit: row.get("phantom_exit")?,
            forced_exit: row.get("forced_exit")?,
            features_extracted: row.get("features_extracted")?,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                .map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "created_at".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?
                .with_timezone(&Utc),
        })
    }

    /// Helper: Convert database row to FeatureVector
    fn row_to_feature_vector(&self, row: &Row) -> Result<FeatureVector, rusqlite::Error> {
        Ok(FeatureVector {
            trade_id: row.get("trade_id")?,
            drop_10s_norm: row.get("drop_10s_norm")?,
            drop_30s_norm: row.get("drop_30s_norm")?,
            drop_60s_norm: row.get("drop_60s_norm")?,
            drop_120s_norm: row.get("drop_120s_norm")?,
            drop_320s_norm: row.get("drop_320s_norm")?,
            drop_velocity_30s: row.get("drop_velocity_30s")?,
            drop_acceleration: row.get("drop_acceleration")?,
            liquidity_tier: row.get("liquidity_tier")?,
            tx_activity_score: row.get("tx_activity_score")?,
            security_score_norm: row.get("security_score_norm")?,
            holder_count_log: row.get("holder_count_log")?,
            market_cap_tier: row.get("market_cap_tier")?,
            ath_prox_15m: row.get("ath_prox_15m")?,
            ath_prox_1h: row.get("ath_prox_1h")?,
            ath_prox_6h: row.get("ath_prox_6h")?,
            ath_risk_score: row.get("ath_risk_score")?,
            hour_sin: row.get("hour_sin")?,
            hour_cos: row.get("hour_cos")?,
            day_sin: row.get("day_sin")?,
            day_cos: row.get("day_cos")?,
            re_entry_flag: row.get("re_entry_flag")?,
            token_trade_count: row.get("token_trade_count")?,
            recent_exit_count: row.get("recent_exit_count")?,
            avg_hold_duration: row.get("avg_hold_duration")?,
            success_label: row.get("success_label")?,
            quick_success_label: row.get("quick_success_label")?,
            risk_label: row.get("risk_label")?,
            peak_time_label: row.get("peak_time_label")?,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                .map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "created_at".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?
                .with_timezone(&Utc),
        })
    }

    /// Helper: Convert database row to ModelWeights
    fn row_to_model_weights(&self, row: &Row) -> Result<ModelWeights, rusqlite::Error> {
        let success_weights: Vec<f64> =
            serde_json::from_str(&row.get::<_, String>("success_weights")?).map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    0,
                    "success_weights".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?;
        let risk_weights: Vec<f64> = serde_json::from_str(&row.get::<_, String>("risk_weights")?)
            .map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                0,
                "risk_weights".to_string(),
                rusqlite::types::Type::Text,
            )
        })?;
        let feature_importance: Vec<f64> =
            serde_json::from_str(&row.get::<_, String>("feature_importance")?).map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    0,
                    "feature_importance".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?;

        Ok(ModelWeights {
            version: row.get("version")?,
            model_type: row.get("model_type")?,
            success_weights,
            success_intercept: row.get("success_intercept")?,
            success_threshold: row.get("success_threshold")?,
            risk_weights,
            risk_intercept: row.get("risk_intercept")?,
            risk_threshold: row.get("risk_threshold")?,
            training_samples: row.get("training_samples")?,
            validation_accuracy: row.get("validation_accuracy")?,
            feature_importance,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                .map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "created_at".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?
                .with_timezone(&Utc),
            trained_on_trades: row.get("trained_on_trades")?,
        })
    }
}
