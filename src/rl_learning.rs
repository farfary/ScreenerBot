use std::collections::VecDeque;
use std::sync::{ Arc, Mutex };
use std::path::Path;
use chrono::{ DateTime, Utc };
use tokio::time::{ Duration, interval };
use tokio::sync::Notify;
use tokio::fs;
use serde::{ Deserialize, Serialize };
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::ensemble::random_forest_regressor::{
    RandomForestRegressor,
    RandomForestRegressorParameters,
};
use smartcore::api::{ SupervisedEstimator, Predictor };

use crate::logger::{ log, LogTag };
use crate::global::{ is_debug_trader_enabled, is_debug_rl_learn_enabled };
use crate::tokens::pool::get_pool_service;
use crate::positions::get_open_positions;

/// Model performance metrics for tracking training quality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    pub training_records: usize,
    pub training_time: DateTime<Utc>,
    pub feature_count: usize,
    pub trees_count: usize,
    pub last_prediction_count: usize,
}

/// Comprehensive price analysis for entry decisions
#[derive(Debug, Clone)]
pub struct PriceAnalysis {
    pub current_price: f64,
    pub price_change_5min: f64,
    pub price_change_10min: f64,
    pub price_change_30min: f64,
    pub price_change_1hour: f64,
    pub recent_high: f64,
    pub recent_low: f64,
    pub drop_percentage: f64, // Percentage drop from recent high
    pub range_position: f64, // Position within recent range (0.0 = low, 1.0 = high)
    pub volatility: f64, // Recent price volatility
    pub momentum_score: f64, // Momentum/acceleration score
    pub pool_price: f64, // Real-time pool price
}

impl PriceAnalysis {
    pub fn default_for_price(price: f64) -> Self {
        Self {
            current_price: price,
            price_change_5min: 0.0,
            price_change_10min: 0.0,
            price_change_30min: 0.0,
            price_change_1hour: 0.0,
            recent_high: price,
            recent_low: price,
            drop_percentage: 0.0,
            range_position: 0.5,
            volatility: 0.0,
            momentum_score: 0.5,
            pool_price: price,
        }
    }
}

/// Entry recommendation based on analysis
#[derive(Debug, Clone, PartialEq)]
pub enum EntryRecommendation {
    StrongBuy,
    Buy,
    WeakBuy,
    Hold,
    WeakSell,
    Sell,
}

impl EntryRecommendation {
    pub fn to_string(&self) -> &'static str {
        match self {
            EntryRecommendation::StrongBuy => "STRONG BUY",
            EntryRecommendation::Buy => "BUY",
            EntryRecommendation::WeakBuy => "WEAK BUY",
            EntryRecommendation::Hold => "HOLD",
            EntryRecommendation::WeakSell => "WEAK SELL",
            EntryRecommendation::Sell => "SELL",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            EntryRecommendation::StrongBuy => "🚀",
            EntryRecommendation::Buy => "📈",
            EntryRecommendation::WeakBuy => "📊",
            EntryRecommendation::Hold => "⏸️",
            EntryRecommendation::WeakSell => "📉",
            EntryRecommendation::Sell => "🛑",
        }
    }
}

/// Comprehensive entry analysis result
#[derive(Debug, Clone)]
pub struct EntryAnalysis {
    pub rl_score: f64, // RL model prediction score (0.0-1.0)
    pub timing_score: f64, // Price timing analysis score (0.0-1.0)
    pub risk_score: f64, // Risk assessment score (0.0-1.0)
    pub combined_score: f64, // Combined weighted score (0.0-1.0)
    pub price_analysis: PriceAnalysis,
    pub recommendation: EntryRecommendation,
    pub confidence: f64, // Confidence in the analysis (0.0-1.0)
}

/// Persistent state structure for saving/loading RL data
#[derive(Debug, Serialize, Deserialize)]
struct RlPersistentState {
    records: Vec<LearningRecord>, // Convert from VecDeque for JSON serialization
    is_trained: bool,
    last_training_time: Option<DateTime<Utc>>,
    model_metrics: Option<ModelMetrics>,
    version: u32, // For future compatibility
}

/// Learning data point that captures market state and trading outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningRecord {
    pub timestamp: DateTime<Utc>,
    pub token_mint: String,
    pub token_symbol: String,

    // Market features at time of decision
    pub current_price: f64,
    pub price_change_5min: f64,
    pub price_change_10min: f64,
    pub price_change_30min: f64,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub market_cap: Option<f64>,
    pub rugcheck_score: Option<f64>,

    // Pool-specific features
    pub pool_price: f64,
    pub price_drop_detected: f64, // Percentage drop that triggered entry
    pub confidence_score: f64, // Entry confidence from our current system

    // Trading outcome (what we're trying to predict)
    pub actual_profit_percent: f64, // Final profit/loss percentage
    pub hold_duration_minutes: f64, // How long position was held
    pub success: bool, // Whether trade was profitable
}

/// Simple RL learning system that uses Random Forest to predict trading outcomes
#[derive(Debug)]
pub struct TradingLearner {
    records: Arc<Mutex<VecDeque<LearningRecord>>>,
    model: Arc<Mutex<Option<RandomForestRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>>>>>,
    is_trained: Arc<Mutex<bool>>,
    model_metrics: Arc<Mutex<Option<ModelMetrics>>>,
    max_records: usize,
    min_records_for_training: usize,
    last_save_time: Arc<Mutex<DateTime<Utc>>>,
}

impl TradingLearner {
    pub fn new() -> Self {
        let mut instance = Self {
            records: Arc::new(Mutex::new(VecDeque::new())),
            model: Arc::new(Mutex::new(None)),
            is_trained: Arc::new(Mutex::new(false)),
            model_metrics: Arc::new(Mutex::new(None)),
            max_records: 1000, // Keep last 1000 trading records
            min_records_for_training: 50, // Need at least 50 records to start training
            last_save_time: Arc::new(Mutex::new(Utc::now())),
        };

        // Load existing data from disk
        if let Err(e) = instance.load_from_disk() {
            if is_debug_rl_learn_enabled() {
                log(
                    LogTag::RlLearn,
                    "LOAD_ERROR",
                    &format!("⚠️ Failed to load RL data from disk: {} - starting fresh", e)
                );
            }
        }

        instance
    }

    /// Save RL data to disk for persistence
    pub fn save_to_disk(&self) -> Result<(), String> {
        const DATA_FILE: &str = "rl_learning_records.json";
        const TEMP_FILE: &str = "rl_learning_records.json.tmp";

        // Gather data from protected fields
        let records_data = {
            let records_guard = self.records
                .lock()
                .map_err(|_| "Failed to lock records for saving")?;
            records_guard.iter().cloned().collect::<Vec<_>>()
        };

        let is_trained = {
            let trained_guard = self.is_trained
                .lock()
                .map_err(|_| "Failed to lock training status for saving")?;
            *trained_guard
        };

        let model_metrics = {
            let metrics_guard = self.model_metrics
                .lock()
                .map_err(|_| "Failed to lock model metrics for saving")?;
            metrics_guard.clone()
        };

        let last_training_time = model_metrics.as_ref().map(|m| m.training_time);

        // Create persistent state
        let state = RlPersistentState {
            records: records_data,
            is_trained,
            last_training_time,
            model_metrics,
            version: 1,
        };

        // Serialize to JSON
        let json_data = serde_json
            ::to_string_pretty(&state)
            .map_err(|e| format!("Failed to serialize RL data: {}", e))?;

        // Write to temporary file first (atomic operation)
        std::fs
            ::write(TEMP_FILE, json_data)
            .map_err(|e| format!("Failed to write RL data to temp file: {}", e))?;

        // Rename temp file to final file (atomic operation)
        std::fs
            ::rename(TEMP_FILE, DATA_FILE)
            .map_err(|e| format!("Failed to rename RL data file: {}", e))?;

        // Update last save time
        if let Ok(mut last_save_guard) = self.last_save_time.lock() {
            *last_save_guard = Utc::now();
        }

        if is_debug_rl_learn_enabled() {
            log(
                LogTag::RlLearn,
                "SAVE_SUCCESS",
                &format!(
                    "💾 Saved RL data to disk: {} records, trained: {}, metrics: {}",
                    state.records.len(),
                    state.is_trained,
                    state.model_metrics.is_some()
                )
            );
        }

        Ok(())
    }

    /// Load RL data from disk for persistence
    pub fn load_from_disk(&self) -> Result<(), String> {
        const DATA_FILE: &str = "rl_learning_records.json";

        // Check if file exists
        if !Path::new(DATA_FILE).exists() {
            if is_debug_rl_learn_enabled() {
                log(
                    LogTag::RlLearn,
                    "LOAD_SKIP",
                    "📁 No existing RL data file found - starting with empty state"
                );
            }
            return Ok(()); // Not an error - first run
        }

        // Read and parse file
        let file_content = std::fs
            ::read_to_string(DATA_FILE)
            .map_err(|e| format!("Failed to read RL data file: {}", e))?;

        let state: RlPersistentState = serde_json
            ::from_str(&file_content)
            .map_err(|e| format!("Failed to parse RL data file: {}", e))?;

        // Validate version compatibility
        if state.version > 1 {
            return Err(format!("Unsupported RL data version: {}", state.version));
        }

        // Load records
        {
            let mut records_guard = self.records
                .lock()
                .map_err(|_| "Failed to lock records for loading")?;
            records_guard.clear();
            for record in &state.records {
                records_guard.push_back(record.clone());
            }
            // Truncate if too many records
            while records_guard.len() > self.max_records {
                records_guard.pop_front();
            }
        }

        // Load training status
        {
            let mut trained_guard = self.is_trained
                .lock()
                .map_err(|_| "Failed to lock training status for loading")?;
            *trained_guard = state.is_trained;
        }

        // Load model metrics
        {
            let mut metrics_guard = self.model_metrics
                .lock()
                .map_err(|_| "Failed to lock model metrics for loading")?;
            *metrics_guard = state.model_metrics.clone();
        }

        // If we had a trained model, we need to retrain since we can't serialize the actual model
        if state.is_trained {
            // Reset training status - we'll retrain with loaded data
            if let Ok(mut trained_guard) = self.is_trained.lock() {
                *trained_guard = false;
            }

            if is_debug_rl_learn_enabled() {
                log(
                    LogTag::RlLearn,
                    "RETRAIN_NEEDED",
                    "🔄 Model was previously trained - will retrain with loaded data"
                );
            }
        }

        if is_debug_rl_learn_enabled() {
            log(
                LogTag::RlLearn,
                "LOAD_SUCCESS",
                &format!(
                    "📂 Successfully loaded RL data: {} records, last trained: {}",
                    state.records.len(),
                    state.last_training_time.map_or("never".to_string(), |t|
                        t.format("%Y-%m-%d %H:%M:%S").to_string()
                    )
                )
            );
        }

        Ok(())
    }

    /// Add a new learning record (called when a position is closed)
    pub fn add_learning_record(&self, record: LearningRecord) {
        if let Ok(mut records) = self.records.lock() {
            // Keep only most recent records
            if records.len() >= self.max_records {
                records.pop_front();
            }
            records.push_back(record.clone());

            if is_debug_rl_learn_enabled() {
                log(
                    LogTag::RlLearn,
                    "RECORD_ADDED",
                    &format!(
                        "📝 Added learning record for {}: {:.2}% profit in {:.1}min, total records: {}",
                        record.token_symbol,
                        record.actual_profit_percent,
                        record.hold_duration_minutes,
                        records.len()
                    )
                );
            }
        }

        // Save to disk after adding new record
        if let Err(e) = self.save_to_disk() {
            if is_debug_rl_learn_enabled() {
                log(
                    LogTag::RlLearn,
                    "SAVE_ERROR",
                    &format!("❌ Failed to save RL data after adding record: {}", e)
                );
            }
        }
    }

    /// Convert learning records to training features
    fn prepare_training_data(
        &self,
        records: &VecDeque<LearningRecord>
    ) -> Result<(DenseMatrix<f64>, Vec<f64>, usize), String> {
        if records.is_empty() {
            return Err("No records available for training".to_string());
        }

        let mut features = Vec::new();
        let mut targets = Vec::new();

        for record in records.iter() {
            // Create feature vector (11 features) with NaN/Infinity protection
            let feature_row = vec![
                record.current_price.max(0.0).min(1e10), // Cap extreme values
                record.price_change_5min.max(-1000.0).min(1000.0), // Cap price changes
                record.price_change_10min.max(-1000.0).min(1000.0),
                record.price_change_30min.max(-1000.0).min(1000.0),
                if record.liquidity_usd > 0.0 {
                    record.liquidity_usd.log10().max(0.0)
                } else {
                    0.0
                },
                if record.volume_24h > 0.0 {
                    record.volume_24h.log10().max(0.0)
                } else {
                    0.0
                },
                if let Some(mc) = record.market_cap {
                    if mc > 0.0 { mc.log10().max(0.0) } else { 0.0 }
                } else {
                    0.0
                },
                record.rugcheck_score.unwrap_or(50.0).max(0.0).min(100.0), // Cap risk score
                record.pool_price.max(0.0).min(1e10), // Cap pool price
                record.price_drop_detected.max(0.0).min(100.0), // Cap drop percentage
                record.confidence_score.max(0.0).min(1.0) // Cap confidence
            ];

            // Validate all features are finite
            let valid_features: Vec<f64> = feature_row
                .into_iter()
                .map(|f| if f.is_finite() { f } else { 0.0 })
                .collect();

            features.push(valid_features);

            // Validate target is finite
            let target = if record.actual_profit_percent.is_finite() {
                record.actual_profit_percent.max(-1000.0).min(1000.0) // Cap extreme profits
            } else {
                0.0
            };
            targets.push(target);
        }

        let num_features = if !features.is_empty() { features[0].len() } else { 0 };

        // Convert to DenseMatrix
        let feature_matrix = DenseMatrix::from_2d_vec(&features).map_err(|e|
            format!("Failed to create feature matrix: {}", e)
        )?;

        Ok((feature_matrix, targets, num_features))
    }

    /// Train the model with current records
    pub async fn train_model(&self) -> Result<(), String> {
        let records = {
            let records_guard = self.records
                .lock()
                .map_err(|_| "Failed to lock records for training")?;

            if records_guard.len() < self.min_records_for_training {
                if is_debug_rl_learn_enabled() {
                    log(
                        LogTag::RlLearn,
                        "TRAINING_SKIP",
                        &format!(
                            "⏳ Need {} records for training, have {} - skipping training",
                            self.min_records_for_training,
                            records_guard.len()
                        )
                    );
                }
                return Err(
                    format!(
                        "Need at least {} records for training, have {}",
                        self.min_records_for_training,
                        records_guard.len()
                    )
                );
            }

            records_guard.clone()
        };

        // Prepare training data
        if is_debug_rl_learn_enabled() {
            log(
                LogTag::RlLearn,
                "TRAINING_START",
                &format!("🎯 Starting model training with {} records", records.len())
            );
        }

        let (features, targets, num_features) = self.prepare_training_data(&records)?;

        // Train Random Forest model
        let parameters = RandomForestRegressorParameters {
            n_trees: 50, // 50 trees for good performance
            max_depth: Some(10), // Limit depth to prevent overfitting
            min_samples_leaf: 3, // Minimum samples per leaf
            min_samples_split: 5, // Minimum samples to split
            m: Some(4), // Use sqrt of features (~3.3, rounded to 4)
            keep_samples: false, // Don't need OOB for this use case
            seed: 42, // Fixed seed for reproducibility
        };

        let model = RandomForestRegressor::fit(&features, &targets, parameters).map_err(|e|
            format!("Failed to train model: {:?}", e)
        )?;

        // Update model and training status
        {
            let mut model_guard = self.model.lock().map_err(|_| "Failed to lock model for update")?;
            *model_guard = Some(model);
        }

        {
            let mut trained_guard = self.is_trained
                .lock()
                .map_err(|_| "Failed to lock training status")?;
            *trained_guard = true;
        }

        // Save model metrics
        {
            let metrics = ModelMetrics {
                training_records: records.len(),
                training_time: Utc::now(),
                feature_count: num_features,
                trees_count: 50,
                last_prediction_count: 0,
            };

            let mut metrics_guard = self.model_metrics
                .lock()
                .map_err(|_| "Failed to lock model metrics")?;
            *metrics_guard = Some(metrics);
        }

        log(
            LogTag::RlLearn,
            "TRAINING_SUCCESS",
            &format!(
                "🎓 Successfully trained Random Forest model: {} records, {} features, 50 trees",
                records.len(),
                num_features
            )
        );

        // Save to disk after successful training
        if let Err(e) = self.save_to_disk() {
            if is_debug_rl_learn_enabled() {
                log(
                    LogTag::RlLearn,
                    "SAVE_ERROR",
                    &format!("❌ Failed to save RL data after training: {}", e)
                );
            }
        }

        Ok(())
    }

    /// Predict profit percentage for a potential trade
    pub async fn predict_profit(
        &self,
        token_mint: &str,
        current_price: f64,
        price_changes: (f64, f64, f64), // 5min, 10min, 30min
        liquidity_usd: f64,
        volume_24h: f64,
        market_cap: Option<f64>,
        rugcheck_score: Option<f64>,
        pool_price: f64,
        price_drop_detected: f64,
        confidence_score: f64
    ) -> Result<f64, String> {
        // Check if model is trained
        let is_trained = self.is_trained
            .lock()
            .map_err(|_| "Failed to lock training status")?
            .clone();

        if !is_trained {
            return Err("Model not yet trained".to_string());
        }

        // Create feature vector first with same validation as training
        let features = vec![
            current_price.max(0.0).min(1e10), // Cap extreme values
            price_changes.0.max(-1000.0).min(1000.0), // 5-min change
            price_changes.1.max(-1000.0).min(1000.0), // 10-min change
            price_changes.2.max(-1000.0).min(1000.0), // 30-min change
            if liquidity_usd > 0.0 {
                liquidity_usd.log10().max(0.0)
            } else {
                0.0
            },
            if volume_24h > 0.0 {
                volume_24h.log10().max(0.0)
            } else {
                0.0
            },
            if let Some(mc) = market_cap {
                if mc > 0.0 { mc.log10().max(0.0) } else { 0.0 }
            } else {
                0.0
            },
            rugcheck_score.unwrap_or(50.0).max(0.0).min(100.0) as f64, // Cap risk score
            pool_price.max(0.0).min(1e10), // Cap pool price
            price_drop_detected.max(0.0).min(100.0), // Use actual drop percentage, not binary
            confidence_score.max(0.0).min(1.0) // Cap confidence
        ];

        // Validate all features are finite
        let valid_features: Vec<f64> = features
            .into_iter()
            .map(|f| if f.is_finite() { f } else { 0.0 })
            .collect();

        // Use model within lock scope without cloning
        let model_guard = self.model.lock().map_err(|_| "Failed to lock model for prediction")?;

        let model = model_guard.as_ref().ok_or("Model not available")?;

        // Convert to matrix (1 row, 11 columns)
        let feature_matrix = DenseMatrix::from_2d_array(&[&valid_features]).map_err(|e|
            format!("Failed to create prediction matrix: {}", e)
        )?;

        // Make prediction
        let prediction = model
            .predict(&feature_matrix)
            .map_err(|e| format!("Failed to make prediction: {:?}", e))?;

        let predicted_profit = prediction[0];

        // Update prediction count in metrics
        if let Ok(mut metrics_guard) = self.model_metrics.lock() {
            if let Some(ref mut metrics) = metrics_guard.as_mut() {
                metrics.last_prediction_count += 1;
            }
        }

        if is_debug_rl_learn_enabled() {
            log(
                LogTag::RlLearn,
                "PREDICTION",
                &format!(
                    "🔮 {} prediction: {:.2}% profit (features: price:{:.12}, liq:${:.0}, vol:${:.0}, risk:{:.0})",
                    token_mint,
                    predicted_profit,
                    current_price,
                    liquidity_usd,
                    volume_24h,
                    rugcheck_score.unwrap_or(100.0)
                )
            );
        }

        Ok(predicted_profit)
    }

    /// Get a learning score (0.0 to 1.0) that can be used alongside existing entry logic
    pub async fn get_learning_score(
        &self,
        token_mint: &str,
        current_price: f64,
        price_changes: (f64, f64, f64),
        liquidity_usd: f64,
        volume_24h: f64,
        market_cap: Option<f64>,
        rugcheck_score: Option<f64>,
        pool_price: f64,
        price_drop_detected: f64,
        confidence_score: f64
    ) -> f64 {
        match
            self.predict_profit(
                token_mint,
                current_price,
                price_changes,
                liquidity_usd,
                volume_24h,
                market_cap,
                rugcheck_score,
                pool_price,
                price_drop_detected,
                confidence_score
            ).await
        {
            Ok(predicted_profit) => {
                // Convert profit prediction to score (0.0 to 1.0)
                // Positive profits get higher scores, capped at 1.0
                let score = (predicted_profit / 100.0 + 0.5).clamp(0.0, 1.0);
                score
            }
            Err(_) => {
                // If model not ready, return neutral score
                0.5
            }
        }
    }

    /// Check if the learning system is ready to make predictions
    pub fn is_model_ready(&self) -> bool {
        self.is_trained
            .lock()
            .map(|guard| *guard)
            .unwrap_or(false)
    }

    /// Get current number of learning records
    pub fn get_record_count(&self) -> usize {
        self.records
            .lock()
            .map(|guard| guard.len())
            .unwrap_or(0)
    }

    /// Get model performance metrics
    pub fn get_model_metrics(&self) -> Option<ModelMetrics> {
        self.model_metrics
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Force save to disk (useful for graceful shutdown)
    pub async fn force_save(&self) -> Result<(), String> {
        // Create a simple task to save synchronously
        // Since save_to_disk() is sync, we don't need spawn_blocking
        self.save_to_disk()
    }

    /// Advanced entry analysis that considers price patterns and optimal timing
    pub async fn analyze_entry_opportunity(
        &self,
        token_mint: &str,
        current_price: f64,
        liquidity_usd: f64,
        volume_24h: f64,
        market_cap: Option<f64>,
        rugcheck_score: Option<f64>
    ) -> EntryAnalysis {
        let pool_service = get_pool_service();
        let price_history = pool_service.get_recent_price_history(token_mint).await;

        // Calculate comprehensive price analysis
        let mut price_analysis = self.analyze_price_patterns(&price_history, current_price);

        // Get real-time pool price
        if let Some(price_result) = pool_service.get_pool_price(token_mint, None).await {
            if let Some(pool_price) = price_result.price_sol {
                price_analysis.pool_price = pool_price;
            }
        }

        // Get RL prediction if model is ready
        let rl_score = if self.is_model_ready() {
            self.get_learning_score(
                token_mint,
                current_price,
                (
                    price_analysis.price_change_5min,
                    price_analysis.price_change_10min,
                    price_analysis.price_change_30min,
                ),
                liquidity_usd,
                volume_24h,
                market_cap,
                rugcheck_score,
                price_analysis.pool_price,
                price_analysis.drop_percentage,
                price_analysis.momentum_score
            ).await
        } else {
            0.5 // Neutral score if model not ready
        };

        // Calculate timing score based on price patterns
        let timing_score = self.calculate_timing_score(&price_analysis);

        // Calculate risk-adjusted score
        let risk_score = self.calculate_risk_score(liquidity_usd, volume_24h, rugcheck_score);

        // Combined entry score (0.0 to 1.0)
        let combined_score = (rl_score * 0.4 + timing_score * 0.4 + risk_score * 0.2).clamp(
            0.0,
            1.0
        );

        EntryAnalysis {
            rl_score,
            timing_score,
            risk_score,
            combined_score,
            price_analysis: price_analysis.clone(),
            recommendation: self.get_entry_recommendation(combined_score, &price_analysis),
            confidence: self.calculate_confidence(&price_analysis, rl_score),
        }
    }

    /// Analyze price patterns for entry timing
    fn analyze_price_patterns(
        &self,
        price_history: &[(DateTime<Utc>, f64)],
        current_price: f64
    ) -> PriceAnalysis {
        if price_history.len() < 6 {
            return PriceAnalysis::default_for_price(current_price);
        }

        let current_time = Utc::now();
        let mut price_5min_ago = current_price;
        let mut price_10min_ago = current_price;
        let mut price_30min_ago = current_price;
        let mut price_1hour_ago = current_price;

        // Find historical prices at specific intervals
        for (timestamp, price) in price_history.iter().rev() {
            let minutes_ago = (current_time - *timestamp).num_minutes() as f64;

            if minutes_ago >= 4.0 && minutes_ago <= 6.0 {
                price_5min_ago = *price;
            }
            if minutes_ago >= 9.0 && minutes_ago <= 11.0 {
                price_10min_ago = *price;
            }
            if minutes_ago >= 28.0 && minutes_ago <= 32.0 {
                price_30min_ago = *price;
            }
            if minutes_ago >= 58.0 && minutes_ago <= 62.0 {
                price_1hour_ago = *price;
                break;
            }
        }

        // Calculate price changes with zero-division protection
        let price_change_5min = if price_5min_ago != 0.0 {
            ((current_price - price_5min_ago) / price_5min_ago) * 100.0
        } else {
            0.0
        };
        let price_change_10min = if price_10min_ago != 0.0 {
            ((current_price - price_10min_ago) / price_10min_ago) * 100.0
        } else {
            0.0
        };
        let price_change_30min = if price_30min_ago != 0.0 {
            ((current_price - price_30min_ago) / price_30min_ago) * 100.0
        } else {
            0.0
        };
        let price_change_1hour = if price_1hour_ago != 0.0 {
            ((current_price - price_1hour_ago) / price_1hour_ago) * 100.0
        } else {
            0.0
        };

        // Find recent high and low
        let mut recent_high = current_price;
        let mut recent_low = current_price;
        let mut volatility_sum = 0.0;
        let mut volatility_count = 0;
        let mut previous_price = None;

        for (timestamp, price) in price_history.iter().rev() {
            let minutes_ago = (current_time - *timestamp).num_minutes() as f64;
            if minutes_ago <= 30.0 {
                recent_high = recent_high.max(*price);
                recent_low = recent_low.min(*price);

                // Calculate volatility (price change between consecutive points)
                if let Some(prev_price) = previous_price {
                    volatility_sum += ((price - prev_price) / prev_price).abs();
                    volatility_count += 1;
                }
                previous_price = Some(*price);
            }
        }

        let volatility = if volatility_count > 0 {
            volatility_sum / (volatility_count as f64)
        } else {
            0.0
        };

        // Calculate drop from recent high with zero-division protection
        let drop_percentage = if recent_high > current_price && recent_high != 0.0 {
            ((recent_high - current_price) / recent_high) * 100.0
        } else {
            0.0
        };

        // Calculate position within recent range with zero-division protection
        let range_position = if recent_high > recent_low && recent_high - recent_low != 0.0 {
            (current_price - recent_low) / (recent_high - recent_low)
        } else {
            0.5
        };

        // Calculate momentum score (acceleration/deceleration)
        let momentum_score = self.calculate_momentum_score(
            price_change_5min,
            price_change_10min,
            price_change_30min,
            price_change_1hour
        );

        // Pool price will be updated by the caller
        let pool_price = current_price;

        PriceAnalysis {
            current_price,
            price_change_5min,
            price_change_10min,
            price_change_30min,
            price_change_1hour,
            recent_high,
            recent_low,
            drop_percentage,
            range_position,
            volatility,
            momentum_score,
            pool_price,
        }
    }

    /// Calculate momentum score based on price acceleration
    fn calculate_momentum_score(
        &self,
        change_5min: f64,
        change_10min: f64,
        change_30min: f64,
        change_1hour: f64
    ) -> f64 {
        // Look for deceleration in downward movement (good entry signal)
        if change_5min < 0.0 && change_10min < 0.0 && change_30min < 0.0 {
            // Downtrend, but check if it's slowing down
            let acceleration_5_10 = change_5min - change_10min;
            let acceleration_10_30 = change_10min - change_30min;

            // If recent drops are smaller (deceleration), it's a good entry signal
            if acceleration_5_10 > 0.0 && acceleration_10_30 > 0.0 {
                return 0.8; // Strong deceleration signal
            } else if acceleration_5_10 > 0.0 || acceleration_10_30 > 0.0 {
                return 0.6; // Moderate deceleration
            }
        }

        // Look for reversal patterns
        if change_5min > 0.0 && change_10min < 0.0 && change_30min < 0.0 {
            return 0.7; // Recent reversal after decline
        }

        // Stable decline (good for catching falling knife)
        if change_5min < -2.0 && change_10min < -5.0 && change_30min < -10.0 {
            return 0.65; // Consistent decline, potential bottom
        }

        0.5 // Neutral momentum
    }

    /// Calculate timing score based on price analysis
    fn calculate_timing_score(&self, price_analysis: &PriceAnalysis) -> f64 {
        let mut timing_score: f64 = 0.5;

        // Reward drops from recent high (entry opportunity)
        if price_analysis.drop_percentage >= 10.0 {
            timing_score += 0.2; // Good drop for entry
        } else if price_analysis.drop_percentage >= 5.0 {
            timing_score += 0.1; // Moderate drop
        }

        // Reward being in lower part of range
        if price_analysis.range_position <= 0.3 {
            timing_score += 0.15; // Near recent low
        } else if price_analysis.range_position <= 0.5 {
            timing_score += 0.1; // Below midrange
        }

        // Reward positive momentum (deceleration in decline)
        if price_analysis.momentum_score >= 0.7 {
            timing_score += 0.15; // Strong positive momentum
        } else if price_analysis.momentum_score >= 0.6 {
            timing_score += 0.1; // Good momentum
        }

        // Penalize high volatility
        if price_analysis.volatility > 0.15 {
            timing_score -= 0.1; // High volatility risk
        }

        // Penalize uptrends (avoid FOMO)
        if price_analysis.price_change_5min > 5.0 && price_analysis.price_change_10min > 10.0 {
            timing_score -= 0.2; // Likely FOMO territory
        }

        timing_score.clamp(0.0, 1.0)
    }

    /// Calculate risk score based on market conditions
    fn calculate_risk_score(
        &self,
        liquidity_usd: f64,
        volume_24h: f64,
        rugcheck_score: Option<f64>
    ) -> f64 {
        let mut risk_score: f64 = 0.5;

        // Reward high liquidity
        if liquidity_usd >= 50000.0 {
            risk_score += 0.2; // High liquidity
        } else if liquidity_usd >= 10000.0 {
            risk_score += 0.1; // Good liquidity
        } else if liquidity_usd < 5000.0 {
            risk_score -= 0.15; // Low liquidity risk
        }

        // Reward high volume
        if volume_24h >= 500000.0 {
            risk_score += 0.15; // High volume
        } else if volume_24h >= 100000.0 {
            risk_score += 0.1; // Good volume
        }

        // Penalize high rugcheck risk
        if let Some(risk) = rugcheck_score {
            if risk <= 20.0 {
                risk_score += 0.15; // Low risk
            } else if risk <= 50.0 {
                // Neutral
            } else if risk <= 80.0 {
                risk_score -= 0.1; // Medium-high risk
            } else {
                risk_score -= 0.2; // High risk
            }
        }

        risk_score.clamp(0.0, 1.0)
    }

    /// Get entry recommendation based on score
    fn get_entry_recommendation(
        &self,
        combined_score: f64,
        price_analysis: &PriceAnalysis
    ) -> EntryRecommendation {
        if combined_score >= 0.8 {
            EntryRecommendation::StrongBuy
        } else if combined_score >= 0.65 {
            EntryRecommendation::Buy
        } else if combined_score >= 0.55 {
            EntryRecommendation::WeakBuy
        } else if combined_score >= 0.45 {
            EntryRecommendation::Hold
        } else if combined_score >= 0.35 {
            EntryRecommendation::WeakSell
        } else {
            EntryRecommendation::Sell
        }
    }

    /// Calculate confidence in the analysis
    fn calculate_confidence(&self, price_analysis: &PriceAnalysis, rl_score: f64) -> f64 {
        let mut confidence: f64 = 0.5;

        // Higher confidence with more data points and clear patterns
        if price_analysis.drop_percentage > 0.0 && price_analysis.momentum_score > 0.6 {
            confidence += 0.2; // Clear pattern
        }

        // Higher confidence if RL model is trained and agrees
        if self.is_model_ready() {
            if rl_score >= 0.7 || rl_score <= 0.3 {
                confidence += 0.15; // Strong RL signal
            }
        }

        // Lower confidence with high volatility
        if price_analysis.volatility > 0.2 {
            confidence -= 0.15; // High uncertainty
        }

        confidence.clamp(0.0, 1.0)
    }
}

// Global singleton for the learning system
use std::sync::LazyLock;
static GLOBAL_TRADING_LEARNER: LazyLock<TradingLearner> = LazyLock::new(|| {
    TradingLearner::new()
});

/// Get the global trading learner instance
pub fn get_trading_learner() -> &'static TradingLearner {
    &GLOBAL_TRADING_LEARNER
}

/// Main entry point for RL-assisted entry decisions
/// This function should be called from entry.rs to get RL guidance
pub async fn get_rl_entry_score(
    token_mint: &str,
    current_price: f64,
    liquidity_usd: f64,
    volume_24h: f64,
    market_cap: Option<f64>,
    rugcheck_score: Option<f64>
) -> Result<EntryAnalysis, String> {
    let learner = get_trading_learner();

    let analysis = learner.analyze_entry_opportunity(
        token_mint,
        current_price,
        liquidity_usd,
        volume_24h,
        market_cap,
        rugcheck_score
    ).await;

    if is_debug_rl_learn_enabled() {
        log(
            LogTag::RlLearn,
            "ENTRY_ANALYSIS",
            &format!(
                "🎯 {} Entry Analysis: {} {:.1}% (RL:{:.2}, Timing:{:.2}, Risk:{:.2}, Conf:{:.2})",
                token_mint.chars().take(8).collect::<String>(),
                analysis.recommendation.emoji(),
                analysis.combined_score * 100.0,
                analysis.rl_score,
                analysis.timing_score,
                analysis.risk_score,
                analysis.confidence
            )
        );

        // Detailed breakdown if strong signal
        if analysis.combined_score >= 0.7 || analysis.combined_score <= 0.3 {
            log(
                LogTag::RlLearn,
                "ENTRY_DETAILS",
                &format!(
                    "📊 Price Details: {:.12} SOL, Drop:{:.1}%, Range:{:.1}%, Momentum:{:.2}, Vol:{:.3}",
                    analysis.price_analysis.current_price,
                    analysis.price_analysis.drop_percentage,
                    analysis.price_analysis.range_position * 100.0,
                    analysis.price_analysis.momentum_score,
                    analysis.price_analysis.volatility
                )
            );
        }
    }

    Ok(analysis)
}

/// Simple entry score function for quick integration (returns 0.0-1.0)
pub async fn get_simple_entry_score(
    token_mint: &str,
    current_price: f64,
    liquidity_usd: f64,
    volume_24h: f64,
    market_cap: Option<f64>,
    rugcheck_score: Option<f64>
) -> f64 {
    match
        get_rl_entry_score(
            token_mint,
            current_price,
            liquidity_usd,
            volume_24h,
            market_cap,
            rugcheck_score
        ).await
    {
        Ok(analysis) => analysis.combined_score,
        Err(_) => 0.5, // Neutral score on error
    }
}

/// Check if entry is recommended based on RL analysis
pub async fn is_rl_entry_recommended(
    token_mint: &str,
    current_price: f64,
    liquidity_usd: f64,
    volume_24h: f64,
    market_cap: Option<f64>,
    rugcheck_score: Option<f64>,
    threshold: f64 // Minimum score to recommend (e.g., 0.6)
) -> bool {
    let score = get_simple_entry_score(
        token_mint,
        current_price,
        liquidity_usd,
        volume_24h,
        market_cap,
        rugcheck_score
    ).await;
    score >= threshold
}

/// Background learning service that periodically retrains the model
pub async fn start_learning_service(shutdown_notify: Arc<Notify>) {
    let mut retrain_interval = interval(Duration::from_secs(300)); // Retrain every 5 minutes

    log(
        LogTag::RlLearn,
        "SERVICE_START",
        "🚀 Starting reinforcement learning background service (5-minute retraining cycle)"
    );

    loop {
        tokio::select! {
            _ = retrain_interval.tick() => {
                let learner = get_trading_learner();
                let record_count = learner.get_record_count();
                
                if record_count >= learner.min_records_for_training {
                    if let Err(e) = learner.train_model().await {
                        if is_debug_rl_learn_enabled() {
                            log(
                                LogTag::RlLearn,
                                "TRAINING_ERROR",
                                &format!("❌ Failed to retrain model: {}", e)
                            );
                        }
                    }
                } else {
                    if is_debug_rl_learn_enabled() {
                        log(
                            LogTag::RlLearn,
                            "TRAINING_WAIT",
                            &format!(
                                "⏳ Waiting for more data: {}/{} records",
                                record_count, learner.min_records_for_training
                            )
                        );
                    }
                }
            }
            _ = shutdown_notify.notified() => {
                log(
                    LogTag::RlLearn,
                    "SERVICE_STOP",
                    "🛑 Reinforcement learning service stopping gracefully"
                );
                break;
            }
        }
    }
}

/// Background auto-save service for RL data persistence
pub async fn start_rl_auto_save_service(shutdown_notify: Arc<Notify>) {
    let mut save_interval = interval(Duration::from_secs(300)); // Auto-save every 5 minutes

    log(
        LogTag::RlLearn,
        "AUTOSAVE_START",
        "💾 Starting RL auto-save background service (5-minute intervals)"
    );

    loop {
        tokio::select! {
            _ = save_interval.tick() => {
                let learner = get_trading_learner();
                
                // Check if there's been enough time since last save to warrant a save
                let should_save = {
                    if let Ok(last_save_guard) = learner.last_save_time.lock() {
                        let time_since_save = Utc::now() - *last_save_guard;
                        time_since_save.num_minutes() >= 4 // Save if 4+ minutes since last save
                    } else {
                        true // Save if can't check last save time
                    }
                };

                if should_save {
                    if let Err(e) = learner.save_to_disk() {
                        if is_debug_rl_learn_enabled() {
                            log(
                                LogTag::RlLearn,
                                "AUTOSAVE_ERROR",
                                &format!("❌ Auto-save failed: {}", e)
                            );
                        }
                    } else {
                        if is_debug_rl_learn_enabled() {
                            log(
                                LogTag::RlLearn,
                                "AUTOSAVE_SUCCESS",
                                "✅ Auto-saved RL data successfully"
                            );
                        }
                    }
                }
            }
            _ = shutdown_notify.notified() => {
                // Final save before shutdown
                let learner = get_trading_learner();
                if let Err(e) = learner.save_to_disk() {
                    log(
                        LogTag::RlLearn,
                        "SHUTDOWN_SAVE_ERROR",
                        &format!("❌ Failed to save RL data during shutdown: {}", e)
                    );
                } else {
                    log(
                        LogTag::RlLearn,
                        "SHUTDOWN_SAVE_SUCCESS",
                        "💾 Final RL data save completed successfully"
                    );
                }

                log(
                    LogTag::RlLearn,
                    "AUTOSAVE_STOP",
                    "🛑 RL auto-save service stopping gracefully"
                );
                break;
            }
        }
    }
}

/// Helper function to collect market data for learning (called from trading logic)
pub async fn collect_market_features(
    token_mint: &str,
    token_symbol: &str,
    current_price: f64,
    liquidity_usd: f64,
    volume_24h: f64,
    market_cap: Option<f64>,
    rugcheck_score: Option<f64>
) -> Option<(f64, f64, f64, f64, f64, f64)> {
    // Get pool price history for price changes
    let pool_service = get_pool_service();
    let price_history = pool_service.get_recent_price_history(token_mint).await;

    if price_history.len() < 6 {
        return None; // Need at least 6 data points for 30min history
    }

    // Calculate price changes (most recent vs older prices)
    let current_time = Utc::now();
    let mut price_5min_ago = current_price;
    let mut price_10min_ago = current_price;
    let mut price_30min_ago = current_price;

    for (timestamp, price) in price_history.iter().rev() {
        let minutes_ago = (current_time - *timestamp).num_minutes() as f64;

        if minutes_ago >= 4.0 && minutes_ago <= 6.0 {
            price_5min_ago = *price;
        }
        if minutes_ago >= 9.0 && minutes_ago <= 11.0 {
            price_10min_ago = *price;
        }
        if minutes_ago >= 28.0 && minutes_ago <= 32.0 {
            price_30min_ago = *price;
            break;
        }
    }

    let price_change_5min = ((current_price - price_5min_ago) / price_5min_ago) * 100.0;
    let price_change_10min = ((current_price - price_10min_ago) / price_10min_ago) * 100.0;
    let price_change_30min = ((current_price - price_30min_ago) / price_30min_ago) * 100.0;

    // Get pool price
    let pool_price = match pool_service.get_pool_price(token_mint, None).await {
        Some(price_result) => price_result.price_sol.unwrap_or(current_price),
        None => current_price, // Fallback to current price
    };

    // Calculate price drop detection (current vs highest in last 10 minutes)
    let mut highest_recent = current_price;
    for (timestamp, price) in price_history.iter().rev() {
        let minutes_ago = (current_time - *timestamp).num_minutes() as f64;
        if minutes_ago <= 10.0 {
            highest_recent = highest_recent.max(*price);
        }
    }

    let price_drop_detected = if highest_recent > current_price {
        ((highest_recent - current_price) / highest_recent) * 100.0
    } else {
        0.0
    };

    Some((
        price_change_5min,
        price_change_10min,
        price_change_30min,
        pool_price,
        price_drop_detected,
        confidence_score_placeholder(), // This would come from existing entry logic
    ))
}

/// Placeholder for confidence score - this should be integrated with existing entry logic
fn confidence_score_placeholder() -> f64 {
    0.5 // Default neutral confidence
}

/// Record a completed trade for learning (called when position is closed)
pub async fn record_completed_trade(
    token_mint: &str,
    token_symbol: &str,
    entry_price: f64,
    exit_price: f64,
    entry_time: DateTime<Utc>,
    exit_time: DateTime<Utc>,
    liquidity_usd: f64,
    volume_24h: f64,
    market_cap: Option<f64>,
    rugcheck_score: Option<f64>
) {
    // Calculate trade outcome
    let profit_percent = ((exit_price - entry_price) / entry_price) * 100.0;
    let hold_duration_minutes = (exit_time - entry_time).num_minutes() as f64;
    let success = profit_percent > 0.0;

    // Collect market features at entry time (this would ideally be stored at entry)
    if
        let Some(
            (
                price_change_5min,
                price_change_10min,
                price_change_30min,
                pool_price,
                price_drop_detected,
                confidence_score,
            ),
        ) = collect_market_features(
            token_mint,
            token_symbol,
            entry_price,
            liquidity_usd,
            volume_24h,
            market_cap,
            rugcheck_score
        ).await
    {
        let record = LearningRecord {
            timestamp: entry_time,
            token_mint: token_mint.to_string(),
            token_symbol: token_symbol.to_string(),
            current_price: entry_price,
            price_change_5min,
            price_change_10min,
            price_change_30min,
            liquidity_usd,
            volume_24h,
            market_cap,
            rugcheck_score,
            pool_price,
            price_drop_detected,
            confidence_score,
            actual_profit_percent: profit_percent,
            hold_duration_minutes,
            success,
        };

        let learner = get_trading_learner();
        learner.add_learning_record(record);

        if is_debug_rl_learn_enabled() {
            log(
                LogTag::RlLearn,
                "TRADE_RECORDED",
                &format!(
                    "📈 Recorded completed trade: {} {:.2}% profit in {:.1}min (entry: {:.12} SOL, exit: {:.12} SOL)",
                    token_symbol,
                    profit_percent,
                    hold_duration_minutes,
                    entry_price,
                    exit_price
                )
            );
        }
    }
}
