//! learner/mod.rs
//!
//! Self-improving trading system through pattern analysis and machine learning.
//!
//! This module implements an online learning system that:
//! * Automatically collects trade data from completed positions
//! * Analyzes patterns in drops, peaks, durations, and outcomes
//! * Builds predictive models for entry success and risk assessment
//! * Provides real-time predictions to entry and profit systems
//! * Continuously adapts based on new trade results
//!
//! Key components:
//! * Database: SQLite storage for trades, features, and model weights
//! * Analyzer: Pattern matching and similarity detection
//! * Model: Incremental learning with hot-swappable weights
//! * Integration: Clean APIs for entry.rs and profit.rs
//!
//! Design principles:
//! * Non-blocking: All predictions complete in <5ms or fallback
//! * Append-only: Safe for concurrent reads during writes
//! * Cold-start safe: Works from first trade, improves over time
//! * Fail-safe: Falls back to existing heuristics on errors

pub mod analyzer;
pub mod database;
pub mod integration;
pub mod model;
pub mod types;

use crate::global::is_debug_learning_enabled;
use crate::global::*;
use crate::logger::{log, LogTag};
use crate::positions::Position;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Notify, RwLock as AsyncRwLock};

use analyzer::PatternAnalyzer;
use database::LearningDatabase;
use integration::LearningIntegration;
use model::ModelManager;
use types::*;

// =============================================================================
// LEARNING SYSTEM CONFIGURATION
// =============================================================================

/// Minimum trades required before enabling model-based decisions
const MIN_TRADES_FOR_MODEL: usize = 20;

/// Feature building interval (seconds)
const FEATURE_BUILD_INTERVAL_SEC: u64 = 600; // 10 minutes

/// Model training interval (seconds)
const MODEL_TRAINING_INTERVAL_SEC: u64 = 1800; // 30 minutes

/// Minimum new trades to trigger model retraining
const MIN_NEW_TRADES_FOR_TRAINING: usize = 25;

/// Pattern analysis cache TTL (seconds)
const PATTERN_CACHE_TTL_SEC: u64 = 300; // 5 minutes

/// Maximum feature extraction time (milliseconds)
const MAX_FEATURE_EXTRACT_MS: u64 = 5;

/// Maximum prediction time (milliseconds)
const MAX_PREDICTION_MS: u64 = 5;

// =============================================================================
// GLOBAL LEARNING SYSTEM STATE
// =============================================================================

/// Global learning system instance
static LEARNING_SYSTEM: Lazy<Arc<LearningSystem>> = Lazy::new(|| Arc::new(LearningSystem::new()));

/// Main learning system coordinator
pub struct LearningSystem {
    database: Arc<AsyncRwLock<Option<LearningDatabase>>>,
    analyzer: Arc<PatternAnalyzer>,
    model_manager: Arc<ModelManager>,
    integration: Arc<LearningIntegration>,
    shutdown_notify: Arc<Notify>,
    is_running: Arc<AsyncRwLock<bool>>,
    last_feature_build: Arc<AsyncRwLock<Option<DateTime<Utc>>>>,
    last_model_training: Arc<AsyncRwLock<Option<DateTime<Utc>>>>,
}

impl LearningSystem {
    /// Create new learning system instance
    fn new() -> Self {
        let database = Arc::new(AsyncRwLock::new(None));
        let analyzer = Arc::new(PatternAnalyzer::new());
        let model_manager = Arc::new(ModelManager::new());
        let integration = Arc::new(LearningIntegration::new(
            analyzer.clone(),
            model_manager.clone(),
        ));

        Self {
            database,
            analyzer,
            model_manager,
            integration,
            shutdown_notify: Arc::new(Notify::new()),
            is_running: Arc::new(AsyncRwLock::new(false)),
            last_feature_build: Arc::new(AsyncRwLock::new(None)),
            last_model_training: Arc::new(AsyncRwLock::new(None)),
        }
    }

    /// Initialize the learning system
    pub async fn initialize() -> Result<(), String> {
        let system = &*LEARNING_SYSTEM;

        if is_debug_learning_enabled() {
            log(LogTag::Learning, "INFO", "Initializing learning system...");
        }

        // Initialize database
        let db = LearningDatabase::new().await?;
        *system.database.write().await = Some(db);

        // Load existing model weights
        if let Some(database) = system.database.read().await.as_ref() {
            system.model_manager.load_latest_weights(database).await?;
        }

        // Mark as running
        *system.is_running.write().await = true;

        // Start background tasks
        system.spawn_background_tasks().await;

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                "Learning system initialized successfully",
            );
        }

        Ok(())
    }

    /// Start background analysis and training tasks
    async fn spawn_background_tasks(&self) {
        let system_clone = LEARNING_SYSTEM.clone();
        let shutdown_notify = self.shutdown_notify.clone();

        // Feature building task
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(FEATURE_BUILD_INTERVAL_SEC));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = system_clone.run_feature_building().await {
                            log(LogTag::Learning, "ERROR", &format!("Feature building error: {}", e));
                        }
                    }
                    _ = shutdown_notify.notified() => {
                        if is_debug_learning_enabled() {
                            log(LogTag::Learning, "INFO", "Feature building task shutting down");
                        }
                        break;
                    }
                }
            }
        });

        let system_clone = LEARNING_SYSTEM.clone();
        let shutdown_notify = self.shutdown_notify.clone();

        // Model training task
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(MODEL_TRAINING_INTERVAL_SEC));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = system_clone.run_model_training().await {
                            log(LogTag::Learning, "ERROR", &format!("Model training error: {}", e));
                        }
                    }
                    _ = shutdown_notify.notified() => {
                        if is_debug_learning_enabled() {
                            log(LogTag::Learning, "INFO", "Model training task shutting down");
                        }
                        break;
                    }
                }
            }
        });
    }

    /// Run feature building cycle
    async fn run_feature_building(&self) -> Result<(), String> {
        let database_guard = self.database.read().await;
        let database = database_guard
            .as_ref()
            .ok_or("Learning database not initialized")?;

        // Check if we need to build features
        let last_build = *self.last_feature_build.read().await;
        let now = Utc::now();

        if let Some(last) = last_build {
            if (now - last).num_seconds() < (FEATURE_BUILD_INTERVAL_SEC as i64) {
                return Ok(());
            }
        }

        let start_time = Instant::now();

        // Get unprocessed trades
        let unprocessed_trades = database.get_unprocessed_trades().await?;

        if unprocessed_trades.is_empty() {
            return Ok(());
        }

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                &format!("Building features for {} trades", unprocessed_trades.len()),
            );
        }

        // Build features for each trade
        let mut features_built = 0;
        for trade in unprocessed_trades {
            match self.analyzer.extract_features(&trade, database).await {
                Ok(features) => {
                    database.store_features(&features).await?;
                    database.mark_trade_processed(trade.id).await?;
                    features_built += 1;
                }
                Err(e) => {
                    log(
                        LogTag::Learning,
                        "ERROR",
                        &format!("Feature extraction failed for trade {}: {}", trade.id, e),
                    );
                }
            }
        }

        *self.last_feature_build.write().await = Some(now);

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                &format!(
                    "Feature building completed: {} features built in {}ms",
                    features_built,
                    start_time.elapsed().as_millis()
                ),
            );
        }

        Ok(())
    }

    /// Run model training cycle
    async fn run_model_training(&self) -> Result<(), String> {
        let database_guard = self.database.read().await;
        let database = database_guard
            .as_ref()
            .ok_or("Learning database not initialized")?;

        // Check if we have enough trades
        let total_trades = database.get_total_trade_count().await?;
        if total_trades < MIN_TRADES_FOR_MODEL {
            return Ok(());
        }

        // Check if we need to retrain
        let last_training = *self.last_model_training.read().await;
        let now = Utc::now();

        let should_retrain = if let Some(last) = last_training {
            let elapsed = (now - last).num_seconds();
            if elapsed < (MODEL_TRAINING_INTERVAL_SEC as i64) {
                // Check if we have enough new trades
                let new_trades = database.get_new_trades_since(last).await?;
                new_trades.len() >= MIN_NEW_TRADES_FOR_TRAINING
            } else {
                true
            }
        } else {
            true
        };

        if !should_retrain {
            return Ok(());
        }

        let start_time = Instant::now();

        if is_debug_learning_enabled() {
            log(LogTag::Learning, "INFO", "Starting model training...");
        }

        // Get all features for training
        let training_data = database.get_all_features().await?;

        if training_data.is_empty() {
            return Ok(());
        }

        // Train the model
        let model_weights = self.model_manager.train_incremental(&training_data).await?;

        // Store new weights
        database.store_model_weights(&model_weights).await?;

        // Update model manager with new weights
        self.model_manager.update_weights(model_weights).await;

        *self.last_model_training.write().await = Some(now);

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                &format!(
                    "Model training completed: {} samples processed in {}ms",
                    training_data.len(),
                    start_time.elapsed().as_millis()
                ),
            );
        }

        Ok(())
    }

    /// Record a completed trade for learning
    pub async fn record_trade(
        position: &Position,
        max_up_pct: f64,
        max_down_pct: f64,
    ) -> Result<(), String> {
        let system = &*LEARNING_SYSTEM;

        let database_guard = system.database.read().await;
        let database = database_guard
            .as_ref()
            .ok_or("Learning database not initialized")?;

        let trade_record = TradeRecord::from_position(position, max_up_pct, max_down_pct).await?;

        database.store_trade(&trade_record).await?;

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                &format!(
                    "Recorded trade: {} {} -> pnl: {:.2}%, peak: {:.2}%, dd: {:.2}%",
                    trade_record.symbol,
                    trade_record.mint,
                    trade_record.pnl_pct,
                    max_up_pct,
                    max_down_pct
                ),
            );
        }

        Ok(())
    }

    /// Shutdown the learning system
    pub async fn shutdown() {
        let system = &*LEARNING_SYSTEM;

        if is_debug_learning_enabled() {
            log(LogTag::Learning, "INFO", "Shutting down learning system...");
        }

        *system.is_running.write().await = false;
        system.shutdown_notify.notify_waiters();

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                "Learning system shutdown complete",
            );
        }
    }

    /// Get learning integration interface
    pub fn get_integration() -> Arc<LearningIntegration> {
        LEARNING_SYSTEM.integration.clone()
    }

    /// Check if learning system is enabled and has sufficient data
    pub async fn is_ready_for_predictions() -> bool {
        let system = &*LEARNING_SYSTEM;

        if !*system.is_running.read().await {
            return false;
        }

        let database_guard = system.database.read().await;
        if let Some(database) = database_guard.as_ref() {
            if let Ok(count) = database.get_total_trade_count().await {
                return count >= MIN_TRADES_FOR_MODEL;
            }
        }

        false
    }
}

// =============================================================================
// PUBLIC API FUNCTIONS
// =============================================================================

/// Initialize the learning system (call from trader.rs)
pub async fn initialize_learning_system() -> Result<(), String> {
    LearningSystem::initialize().await
}

/// Record a completed trade (call from positions module)
pub async fn record_completed_trade(
    position: &Position,
    max_up_pct: f64,
    max_down_pct: f64,
) -> Result<(), String> {
    LearningSystem::record_trade(position, max_up_pct, max_down_pct).await
}

/// Shutdown learning system (call from trader.rs)
pub async fn shutdown_learning_system() {
    LearningSystem::shutdown().await
}

/// Get integration interface for entry/profit modules
pub fn get_learning_integration() -> Arc<LearningIntegration> {
    LearningSystem::get_integration()
}

/// Check if learning predictions are available
pub async fn learning_predictions_available() -> bool {
    LearningSystem::is_ready_for_predictions().await
}
