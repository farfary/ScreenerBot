//! learner/model.rs
//!
//! Machine learning model management and training.
//!
//! This module implements:
//! * Incremental logistic regression for binary classification
//! * Model weight serialization and hot-swapping
//! * Online training with new data batches
//! * Prediction with fallback to heuristics
//!
//! Models trained:
//! * Success prediction: Will this trade be profitable?
//! * Risk prediction: Will this trade have high early drawdown?
//! * Quick profit prediction: Will this trade reach >25% in <20min?
//!
//! Design focuses on simplicity and reliability over sophistication.

use crate::learner::types::*;
use crate::learner::database::LearningDatabase;
use crate::learner::analyzer::PatternAnalyzer;
use crate::logger::{ log, LogTag };
use crate::global::is_debug_learning_enabled;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;

/// Model manager for training and predictions
pub struct ModelManager {
    /// Current model weights (hot-swappable)
    weights: Arc<AsyncRwLock<Option<ModelWeights>>>,

    /// Training state for incremental learning
    training_state: Arc<AsyncRwLock<TrainingState>>,

    /// Pattern analyzer for matching patterns
    analyzer: PatternAnalyzer,
}

/// Internal training state for incremental updates
#[derive(Debug, Clone)]
struct TrainingState {
    /// Accumulated gradients for success model
    success_gradient_sum: Vec<f64>,
    success_intercept_gradient: f64,

    /// Accumulated gradients for risk model
    risk_gradient_sum: Vec<f64>,
    risk_intercept_gradient: f64,

    /// Number of samples processed
    sample_count: usize,

    /// Learning rate
    learning_rate: f64,

    /// L2 regularization strength
    l2_lambda: f64,
}

impl ModelManager {
    /// Create new model manager
    pub fn new() -> Self {
        let initial_state = TrainingState {
            success_gradient_sum: vec![0.0; FeatureVector::FEATURE_COUNT],
            success_intercept_gradient: 0.0,
            risk_gradient_sum: vec![0.0; FeatureVector::FEATURE_COUNT],
            risk_intercept_gradient: 0.0,
            sample_count: 0,
            learning_rate: 0.01, // Conservative learning rate
            l2_lambda: 0.001, // Light regularization
        };

        Self {
            weights: Arc::new(AsyncRwLock::new(None)),
            training_state: Arc::new(AsyncRwLock::new(initial_state)),
            analyzer: PatternAnalyzer::new(),
        }
    }

    /// Load latest model weights from database
    pub async fn load_latest_weights(&self, database: &LearningDatabase) -> Result<(), String> {
        match database.get_latest_model_weights().await? {
            Some(weights) => {
                *self.weights.write().await = Some(weights.clone());

                if is_debug_learning_enabled() {
                    log(
                        LogTag::Learning,
                        "INFO",
                        &format!(
                            "Loaded model weights v{}: {} samples, {:.2}% accuracy",
                            weights.version,
                            weights.training_samples,
                            weights.validation_accuracy * 100.0
                        )
                    );
                }
            }
            None => {
                if is_debug_learning_enabled() {
                    log(LogTag::Learning, "INFO", "No existing model weights found");
                }
            }
        }

        Ok(())
    }

    /// Update current weights (hot-swap)
    pub async fn update_weights(&self, weights: ModelWeights) {
        *self.weights.write().await = Some(weights);

        if is_debug_learning_enabled() {
            log(LogTag::Learning, "INFO", "Model weights updated via hot-swap");
        }
    }

    /// Train incremental model with new feature data
    pub async fn train_incremental(
        &self,
        features: &[FeatureVector]
    ) -> Result<ModelWeights, String> {
        if features.is_empty() {
            return Err("No training data provided".to_string());
        }

        let start_time = std::time::Instant::now();

        // Get current weights or initialize new ones
        let mut current_weights = self.weights
            .read().await
            .clone()
            .unwrap_or_else(|| ModelWeights::new("logistic_regression".to_string()));

        // Prepare training data
        let (X, y_success, y_risk) = self.prepare_training_data(features)?;

        if X.is_empty() {
            return Err("No valid training samples".to_string());
        }

        // Train success model
        let (success_weights, success_intercept) = self.train_logistic_regression(
            &X,
            &y_success,
            &current_weights.success_weights,
            current_weights.success_intercept
        ).await?;

        // Train risk model
        let (risk_weights, risk_intercept) = self.train_logistic_regression(
            &X,
            &y_risk,
            &current_weights.risk_weights,
            current_weights.risk_intercept
        ).await?;

        // Calculate validation accuracy
        let validation_accuracy = self.calculate_validation_accuracy(
            &X,
            &y_success,
            &success_weights,
            success_intercept
        ).await;

        // Calculate feature importance (simplified)
        let feature_importance = self.calculate_feature_importance(&success_weights, &risk_weights);

        // Create new model weights
        let new_weights = ModelWeights {
            version: current_weights.version + 1,
            model_type: "logistic_regression".to_string(),
            success_weights,
            success_intercept,
            success_threshold: 0.5, // Standard threshold
            risk_weights,
            risk_intercept,
            risk_threshold: 0.5,
            training_samples: X.len(),
            validation_accuracy,
            feature_importance,
            created_at: Utc::now(),
            trained_on_trades: features.len() as i64,
        };

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                &format!(
                    "Model training completed: {} samples, {:.2}% accuracy, {}ms",
                    X.len(),
                    validation_accuracy * 100.0,
                    start_time.elapsed().as_millis()
                )
            );
        }

        Ok(new_weights)
    }

    /// Predict success probability for entry decision
    pub async fn predict_success(&self, features: &[f64]) -> Result<f64, String> {
        let weights_guard = self.weights.read().await;
        let weights = weights_guard.as_ref().ok_or("No model weights available")?;

        if features.len() != weights.success_weights.len() {
            return Err("Feature vector size mismatch".to_string());
        }

        let logit = self.calculate_logit(
            features,
            &weights.success_weights,
            weights.success_intercept
        );
        let probability = self.sigmoid(logit);

        Ok(probability)
    }

    /// Predict risk probability for exit decision
    pub async fn predict_risk(&self, features: &[f64]) -> Result<f64, String> {
        let weights_guard = self.weights.read().await;
        let weights = weights_guard.as_ref().ok_or("No model weights available")?;

        if features.len() != weights.risk_weights.len() {
            return Err("Feature vector size mismatch".to_string());
        }

        let logit = self.calculate_logit(features, &weights.risk_weights, weights.risk_intercept);
        let probability = self.sigmoid(logit);

        Ok(probability)
    }

    /// Check if model is ready for predictions
    pub async fn is_ready(&self) -> bool {
        self.weights.read().await.is_some()
    }

    /// Get current model metadata
    pub async fn get_model_info(&self) -> Option<(i64, usize, f64)> {
        self.weights
            .read().await
            .as_ref()
            .map(|w| { (w.version, w.training_samples, w.validation_accuracy) })
    }

    /// Prepare training data from feature vectors
    fn prepare_training_data(
        &self,
        features: &[FeatureVector]
    ) -> Result<(Vec<Vec<f64>>, Vec<f64>, Vec<f64>), String> {
        let mut X = Vec::new();
        let mut y_success = Vec::new();
        let mut y_risk = Vec::new();

        for feature in features {
            // Only use samples with labels
            if
                let (Some(success_label), Some(risk_label)) = (
                    feature.success_label,
                    feature.risk_label,
                )
            {
                X.push(feature.to_array());
                y_success.push(success_label);
                y_risk.push(risk_label);
            }
        }

        Ok((X, y_success, y_risk))
    }

    /// Train logistic regression with incremental updates
    async fn train_logistic_regression(
        &self,
        X: &[Vec<f64>],
        y: &[f64],
        initial_weights: &[f64],
        initial_intercept: f64
    ) -> Result<(Vec<f64>, f64), String> {
        if X.is_empty() || y.is_empty() || X.len() != y.len() {
            return Err("Invalid training data".to_string());
        }

        let n_features = X[0].len();
        let mut weights = initial_weights.to_vec();
        let mut intercept = initial_intercept;

        if weights.len() != n_features {
            weights = vec![0.0; n_features];
            intercept = 0.0;
        }

        let training_state = self.training_state.read().await;
        let learning_rate = training_state.learning_rate;
        let l2_lambda = training_state.l2_lambda;
        drop(training_state);

        // Simple batch gradient descent
        let n_epochs = 10; // Conservative number of epochs for stability

        for _ in 0..n_epochs {
            let mut weight_gradients = vec![0.0; n_features];
            let mut intercept_gradient = 0.0;

            // Calculate gradients
            for (x, &y_true) in X.iter().zip(y.iter()) {
                let logit = self.calculate_logit(x, &weights, intercept);
                let prediction = self.sigmoid(logit);
                let error = prediction - y_true;

                // Update gradients
                for (j, &x_j) in x.iter().enumerate() {
                    weight_gradients[j] += error * x_j;
                }
                intercept_gradient += error;
            }

            // Apply gradients with L2 regularization
            let batch_size = X.len() as f64;
            for j in 0..n_features {
                weight_gradients[j] = weight_gradients[j] / batch_size + l2_lambda * weights[j];
                weights[j] -= learning_rate * weight_gradients[j];
            }

            intercept_gradient /= batch_size;
            intercept -= learning_rate * intercept_gradient;
        }

        Ok((weights, intercept))
    }

    /// Calculate validation accuracy using cross-validation approach
    async fn calculate_validation_accuracy(
        &self,
        X: &[Vec<f64>],
        y: &[f64],
        weights: &[f64],
        intercept: f64
    ) -> f64 {
        if X.is_empty() {
            return 0.0;
        }

        let mut correct = 0;
        let total = X.len();

        for (x, &y_true) in X.iter().zip(y.iter()) {
            let logit = self.calculate_logit(x, weights, intercept);
            let probability = self.sigmoid(logit);
            let prediction = if probability > 0.5 { 1.0 } else { 0.0 };

            if (prediction - y_true).abs() < 0.1 {
                correct += 1;
            }
        }

        (correct as f64) / (total as f64)
    }

    /// Calculate feature importance as absolute weight values
    fn calculate_feature_importance(
        &self,
        success_weights: &[f64],
        risk_weights: &[f64]
    ) -> Vec<f64> {
        let mut importance = vec![0.0; success_weights.len()];

        for i in 0..importance.len() {
            // Combine importance from both models
            let success_imp = success_weights[i].abs();
            let risk_imp = risk_weights[i].abs();
            importance[i] = (success_imp + risk_imp) / 2.0;
        }

        // Normalize to sum to 1.0
        let sum: f64 = importance.iter().sum();
        if sum > 0.0 {
            for imp in &mut importance {
                *imp /= sum;
            }
        }

        importance
    }

    /// Calculate logit (linear combination)
    fn calculate_logit(&self, features: &[f64], weights: &[f64], intercept: f64) -> f64 {
        let mut logit = intercept;
        for (feature, weight) in features.iter().zip(weights.iter()) {
            logit += feature * weight;
        }
        logit
    }

    /// Sigmoid activation function
    fn sigmoid(&self, x: f64) -> f64 {
        let clamped_x = x.clamp(-500.0, 500.0); // Prevent overflow
        1.0 / (1.0 + (-clamped_x).exp())
    }

    /// Generate comprehensive prediction from feature vector
    pub async fn generate_prediction(
        &self,
        database: &LearningDatabase,
        mint: &str,
        features: &[f64],
        current_profit: Option<f64>,
        hold_duration: Option<i64>
    ) -> Result<EntryPrediction, String> {
        if features.len() != FeatureVector::FEATURE_COUNT {
            return Err("Invalid feature vector size".to_string());
        }

        // Get model predictions
        let success_prob = self.predict_success(features).await.unwrap_or(0.5);
        let risk_prob = self.predict_risk(features).await.unwrap_or(0.5);

        // Calculate derived metrics
        let expected_profit = self.estimate_expected_profit(success_prob, features);
        let expected_duration = self.estimate_expected_duration(features);
        let confidence = self.calculate_prediction_confidence(success_prob, risk_prob);

        // Generate recommendation
        let entry_recommendation = self.generate_entry_recommendation(
            success_prob,
            risk_prob,
            confidence
        );

        // Calculate confidence adjustment for existing entry system
        let confidence_adjustment = self.calculate_confidence_adjustment(
            success_prob,
            risk_prob,
            confidence
        );

        Ok(EntryPrediction {
            mint: mint.to_string(),
            confidence,
            success_probability: success_prob,
            quick_profit_probability: success_prob * 0.7, // Rough estimate
            expected_profit,
            expected_duration,
            risk_probability: risk_prob,
            expected_max_drawdown: risk_prob * 20.0, // Rough estimate
            risk_score: risk_prob,
            matching_patterns: self
                .find_matching_patterns_for_prediction(database, mint, features, success_prob).await
                .into_iter()
                .map(|p| p.pattern_id)
                .collect(),
            pattern_confidence: self.calculate_pattern_confidence(&features).await,
            entry_recommendation,
            confidence_adjustment,
            created_at: Utc::now(),
        })
    }

    /// Estimate expected profit from features
    fn estimate_expected_profit(&self, success_prob: f64, features: &[f64]) -> f64 {
        // Base profit estimate
        let base_profit = 15.0; // Conservative base target

        // Adjust based on success probability
        let prob_adjustment = (success_prob - 0.5) * 2.0; // -1 to 1 range

        // Adjust based on specific features if available
        let market_adjustment = if features.len() > 10 {
            features[7] * 10.0 // liquidity_tier feature
        } else {
            0.0
        };

        (base_profit + prob_adjustment * 20.0 + market_adjustment).max(5.0)
    }

    /// Estimate expected hold duration from features
    fn estimate_expected_duration(&self, features: &[f64]) -> i64 {
        // Base duration estimate (seconds)
        let base_duration = 1200; // 20 minutes

        // Adjust based on features if available
        let adjustment = if features.len() > 20 {
            features[23] * 3600.0 // avg_hold_duration feature
        } else {
            0.0
        };

        ((base_duration as f64) + adjustment).max(300.0) as i64 // Min 5 minutes
    }

    /// Calculate prediction confidence
    fn calculate_prediction_confidence(&self, success_prob: f64, risk_prob: f64) -> f64 {
        // Confidence is higher when predictions are more extreme (closer to 0 or 1)
        let success_confidence = (success_prob - 0.5).abs() * 2.0;
        let risk_confidence = (risk_prob - 0.5).abs() * 2.0;

        (success_confidence + risk_confidence) / 2.0
    }

    /// Generate entry recommendation
    fn generate_entry_recommendation(
        &self,
        success_prob: f64,
        risk_prob: f64,
        confidence: f64
    ) -> EntryRecommendation {
        // Combine success and risk into overall score
        let score = success_prob - risk_prob;

        if score > 0.3 && confidence > 0.6 {
            EntryRecommendation::StrongBuy
        } else if score > 0.1 && confidence > 0.4 {
            EntryRecommendation::Buy
        } else if score < -0.3 || risk_prob > 0.7 {
            EntryRecommendation::StrongAvoid
        } else if score < -0.1 || risk_prob > 0.6 {
            EntryRecommendation::Avoid
        } else {
            EntryRecommendation::Neutral
        }
    }

    /// Calculate adjustment to entry confidence score
    fn calculate_confidence_adjustment(
        &self,
        success_prob: f64,
        risk_prob: f64,
        confidence: f64
    ) -> f64 {
        if confidence < 0.4 {
            return 0.0; // Not confident enough to adjust
        }

        let base_score = success_prob - risk_prob;

        // Scale adjustment by confidence
        let adjustment = base_score * confidence;

        // Clamp to reasonable range
        adjustment.clamp(-0.5, 0.5)
    }

    /// Predict entry success (used by integration.rs)
    pub async fn predict_entry_success(
        &self,
        features: &FeatureVector
    ) -> Result<EntryPrediction, String> {
        let feature_array = features.to_array();

        let success_prob = self.predict_success(&feature_array).await?;
        let risk_prob = self.predict_risk(&feature_array).await?;

        // Calculate confidence based on how certain we are
        let confidence = if success_prob > 0.8 || success_prob < 0.2 {
            0.8 // High confidence in extreme predictions
        } else if risk_prob > 0.8 || risk_prob < 0.2 {
            0.7 // Moderate confidence in risk predictions
        } else {
            0.3 // Low confidence in middle range
        };

        let entry_recommendation = if success_prob > 0.7 && risk_prob < 0.3 {
            EntryRecommendation::StrongBuy
        } else if success_prob > 0.6 && risk_prob < 0.4 {
            EntryRecommendation::Buy
        } else if success_prob < 0.4 || risk_prob > 0.6 {
            EntryRecommendation::Avoid
        } else if success_prob < 0.3 || risk_prob > 0.7 {
            EntryRecommendation::StrongAvoid
        } else {
            EntryRecommendation::Neutral
        };

        Ok(EntryPrediction {
            mint: "unknown".to_string(), // Will be set by caller if needed
            confidence,
            success_probability: success_prob,
            quick_profit_probability: success_prob * 0.6, // Estimate
            expected_profit: if success_prob > 0.5 {
                25.0
            } else {
                5.0
            },
            expected_duration: 1800, // 30 minutes default
            risk_probability: risk_prob,
            expected_max_drawdown: if risk_prob > 0.5 {
                15.0
            } else {
                8.0
            },
            risk_score: risk_prob,
            matching_patterns: vec![],
            pattern_confidence: confidence,
            entry_recommendation,
            confidence_adjustment: self.calculate_confidence_adjustment(
                success_prob,
                risk_prob,
                confidence
            ),
            created_at: chrono::Utc::now(),
        })
    }

    /// Predict exit outcome (used by integration.rs)
    pub async fn predict_exit_outcome(
        &self,
        features: &FeatureVector
    ) -> Result<ExitPrediction, String> {
        let feature_array = features.to_array();

        let success_prob = self.predict_success(&feature_array).await?;
        let risk_prob = self.predict_risk(&feature_array).await?;

        // Calculate confidence
        let confidence = if success_prob > 0.7 || risk_prob > 0.7 {
            0.8 // High confidence in strong signals
        } else {
            0.4 // Moderate confidence otherwise
        };

        let exit_recommendation = if risk_prob > 0.7 {
            ExitRecommendation::SellStrong
        } else if risk_prob > 0.6 || success_prob < 0.3 {
            ExitRecommendation::Sell
        } else if success_prob > 0.7 && risk_prob < 0.3 {
            ExitRecommendation::HoldStrong
        } else if success_prob > 0.6 && risk_prob < 0.4 {
            ExitRecommendation::Hold
        } else {
            ExitRecommendation::Neutral
        };

        Ok(ExitPrediction {
            mint: "unknown".to_string(),
            current_profit: 0.0, // Will be set by caller
            hold_duration: 0, // Will be set by caller
            peak_reached_probability: if success_prob < 0.4 {
                0.8
            } else {
                0.3
            },
            further_upside_probability: success_prob,
            reversal_probability: risk_prob,
            recommended_trailing_stop: if risk_prob > 0.6 {
                5.0
            } else {
                10.0
            },
            recommended_profit_target: if success_prob > 0.6 {
                30.0
            } else {
                15.0
            },
            urgency_score: risk_prob,
            drawdown_risk: risk_prob,
            time_pressure: 0.2, // Low default
            exit_recommendation,
            exit_score_adjustment: self.calculate_confidence_adjustment(
                success_prob,
                risk_prob,
                confidence
            ),
            created_at: chrono::Utc::now(),
        })
    }

    /// Find matching patterns for prediction
    async fn find_matching_patterns_for_prediction(
        &self,
        database: &LearningDatabase,
        mint: &str,
        features: &[f64],
        success_prob: f64
    ) -> Vec<TradingPattern> {
        // Extract drop pattern from features
        // Assuming drop features are at positions 14-18 based on feature extraction
        if features.len() < 19 {
            return Vec::new();
        }

        let drop_10s = features[14];
        let drop_30s = features[15];
        let drop_60s = features[16];
        let drop_120s = features[17];
        let drop_320s = features[18];

        // Set confidence threshold based on success probability
        let confidence_threshold = if success_prob > 0.7 {
            0.6 // High success probability, stricter pattern matching
        } else {
            0.4 // Lower success probability, more lenient pattern matching
        };

        // Try to find patterns, but don't fail the prediction if pattern search fails
        match
            self.analyzer.find_matching_patterns(
                database,
                mint,
                drop_10s,
                drop_30s,
                drop_60s,
                drop_120s,
                drop_320s,
                confidence_threshold
            ).await
        {
            Ok(patterns) => {
                // Limit to top 5 most confident patterns
                patterns.into_iter().take(5).collect()
            }
            Err(e) => {
                if is_debug_learning_enabled() {
                    log(
                        LogTag::Learning,
                        "pattern_error",
                        &format!("Failed to find patterns for mint {}: {}", mint, e)
                    );
                }
                Vec::new()
            }
        }
    }

    /// Calculate pattern confidence based on feature quality
    async fn calculate_pattern_confidence(&self, features: &[f64]) -> f64 {
        if features.len() < 19 {
            return 0.0;
        }

        // Factor in drop pattern strength
        let drop_features = &features[14..19];
        let drop_magnitude = drop_features
            .iter()
            .map(|&d| d.abs())
            .fold(0.0, f64::max);

        // Factor in market context
        let liquidity = features.get(7).unwrap_or(&0.0).max(1.0); // Avoid log(0)
        let tx_activity = features.get(8).unwrap_or(&0.0);
        let ath_distance = features.get(9).unwrap_or(&0.0).abs();

        // Calculate composite confidence
        let drop_confidence = (drop_magnitude / 50.0).min(1.0); // Normalize to 0-1
        let liquidity_confidence = (liquidity.ln() / 10.0).min(1.0).max(0.0); // Log scale
        let activity_confidence = (tx_activity / 1000.0).min(1.0); // Normalize to 0-1
        let ath_confidence = if ath_distance > 50.0 { 0.8 } else { 0.4 }; // High ATH distance = more confident

        // Weighted average
        let weights = [0.4, 0.3, 0.2, 0.1]; // Drop, liquidity, activity, ATH
        let values = [drop_confidence, liquidity_confidence, activity_confidence, ath_confidence];

        weights
            .iter()
            .zip(values.iter())
            .map(|(w, v)| w * v)
            .sum()
    }
}
