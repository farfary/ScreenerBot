use crate::logger::{self, LogTag};
use crate::strategies::conditions::ConditionRegistry;
use crate::strategies::types::{
    EvaluationContext, EvaluationResult, LogicalOperator, RuleTree, Strategy,
};
use crate::trader::STRATEGY_CACHE_MAX_ENTRIES;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};

/// Strategy evaluation engine
pub struct StrategyEngine {
    condition_registry: Arc<ConditionRegistry>,
    evaluation_cache: Arc<RwLock<HashMap<String, CachedEvaluation>>>,
    config: EngineConfig,
}

/// Engine configuration
#[derive(Clone)]
pub struct EngineConfig {
    pub evaluation_timeout_ms: u64,
    pub cache_ttl_seconds: u64,
    pub max_concurrent_evaluations: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            evaluation_timeout_ms: 50,
            cache_ttl_seconds: 5,
            max_concurrent_evaluations: 10,
        }
    }
}

/// Cached evaluation result
struct CachedEvaluation {
    result: bool,
    timestamp: Instant,
}

impl StrategyEngine {
    /// Create a new strategy engine
    pub fn new(config: EngineConfig) -> Self {
        Self {
            condition_registry: Arc::new(ConditionRegistry::new()),
            evaluation_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Evaluate a strategy against a context
    pub async fn evaluate_strategy(
        &self,
        strategy: &Strategy,
        context: &EvaluationContext,
    ) -> Result<EvaluationResult, String> {
        let start = Instant::now();

        // Check cache first (safe-scoped to a fingerprint of the evaluation context)
        let cache_enabled = self.config.cache_ttl_seconds > 0;
        let cache_key = if cache_enabled {
            let fp = context_fingerprint(context);
            Some(format!("{}:{}:{}", strategy.id, context.token_mint, fp))
        } else {
            None
        };

        if let (true, Some(key)) = (cache_enabled, cache_key.as_ref()) {
            if let Some(cached) = self.get_cached_evaluation(key).await {
                return Ok(EvaluationResult {
                    strategy_id: strategy.id.clone(),
                    result: cached,
                    confidence: 1.0,
                    execution_time_ms: 0,
                    details: HashMap::new(),
                });
            }
        }

        // Evaluate with timeout
        let timeout_duration = Duration::from_millis(self.config.evaluation_timeout_ms);
        let evaluation_future = self.evaluate_rule_tree(&strategy.rules, context);

        let result = match timeout(timeout_duration, evaluation_future).await {
            Ok(Ok(res)) => res,
            Ok(Err(e)) => {
                logger::error(
                    LogTag::System,
                    &format!(
                        "Strategy evaluation failed: strategy_id={}, error={}",
                        strategy.id, e
                    ),
                );
                return Err(e);
            }
            Err(_) => {
                logger::warning(
                    LogTag::System,
                    &format!(
                        "Strategy evaluation timeout: strategy_id={}, timeout_ms={}",
                        strategy.id, self.config.evaluation_timeout_ms
                    ),
                );
                return Err("Evaluation timeout".to_string());
            }
        };

        let execution_time = start.elapsed().as_millis() as u64;

        // Cache the result if enabled
        if let (true, Some(key)) = (cache_enabled, cache_key.as_ref()) {
            self.cache_evaluation(key, result).await;
        }

        logger::debug(
            LogTag::System,
            &format!(
                "Strategy evaluated: strategy_id={}, result={}, time_ms={}",
                strategy.id, result, execution_time
            ),
        );

        Ok(EvaluationResult {
            strategy_id: strategy.id.clone(),
            result,
            confidence: 1.0,
            execution_time_ms: execution_time,
            details: HashMap::new(),
        })
    }

    /// Evaluate a rule tree recursively
    fn evaluate_rule_tree<'a>(
        &'a self,
        rule_tree: &'a RuleTree,
        context: &'a EvaluationContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool, String>> + Send + 'a>>
    {
        Box::pin(async move {
            // Leaf node - evaluate condition
            if rule_tree.is_leaf() {
                if let Some(condition) = &rule_tree.condition {
                    let evaluator = self
                        .condition_registry
                        .get(&condition.condition_type)
                        .ok_or_else(|| {
                            format!("Unknown condition type: {}", condition.condition_type)
                        })?;

                    // Validate condition first
                    evaluator.validate(condition)?;

                    // Evaluate condition
                    return evaluator.evaluate(condition, context).await;
                } else {
                    return Err("Leaf node missing condition".to_string());
                }
            }

            // Branch node - evaluate operator
            if rule_tree.is_branch() {
                let operator = rule_tree
                    .operator
                    .ok_or_else(|| "Branch node missing operator".to_string())?;

                let conditions = rule_tree
                    .conditions
                    .as_ref()
                    .ok_or_else(|| "Branch node missing conditions".to_string())?;

                return match operator {
                    LogicalOperator::And => {
                        // Short-circuit AND: return false on first false
                        for child in conditions {
                            let result = self.evaluate_rule_tree(child, context).await?;
                            if !result {
                                return Ok(false);
                            }
                        }
                        Ok(true)
                    }
                    LogicalOperator::Or => {
                        // Short-circuit OR: return true on first true
                        for child in conditions {
                            let result = self.evaluate_rule_tree(child, context).await?;
                            if result {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    }
                    LogicalOperator::Not => {
                        if conditions.len() != 1 {
                            return Err("NOT operator must have exactly one child".to_string());
                        }
                        let result = self.evaluate_rule_tree(&conditions[0], context).await?;
                        Ok(!result)
                    }
                };
            }

            Err("Invalid rule tree structure".to_string())
        })
    }

    /// Get cached evaluation if still valid
    async fn get_cached_evaluation(&self, cache_key: &str) -> Option<bool> {
        let cache = self.evaluation_cache.read().await;
        if let Some(cached) = cache.get(cache_key) {
            let age = cached.timestamp.elapsed().as_secs();
            if age < self.config.cache_ttl_seconds {
                return Some(cached.result);
            }
        }
        None
    }

    /// Cache evaluation result
    async fn cache_evaluation(&self, cache_key: &str, result: bool) {
        let mut cache = self.evaluation_cache.write().await;
        cache.insert(
            cache_key.to_string(),
            CachedEvaluation {
                result,
                timestamp: Instant::now(),
            },
        );

        // Cleanup old entries if cache is too large
        if cache.len() > STRATEGY_CACHE_MAX_ENTRIES {
            cache.retain(|_, v| v.timestamp.elapsed().as_secs() < self.config.cache_ttl_seconds);
        }
    }

    /// Clear evaluation cache
    pub async fn clear_cache(&self) {
        let mut cache = self.evaluation_cache.write().await;
        cache.clear();
        logger::info(LogTag::System, "Strategy evaluation cache cleared");
    }

    /// Get condition registry for UI/debugging
    pub fn get_condition_registry(&self) -> &ConditionRegistry {
        &self.condition_registry
    }

    /// Validate a strategy without evaluating
    pub fn validate_strategy(&self, strategy: &Strategy) -> Result<(), String> {
        self.validate_rule_tree(&strategy.rules)
    }

    /// Validate a rule tree recursively
    fn validate_rule_tree(&self, rule_tree: &RuleTree) -> Result<(), String> {
        // Leaf node - validate condition
        if rule_tree.is_leaf() {
            if let Some(condition) = &rule_tree.condition {
                let evaluator = self
                    .condition_registry
                    .get(&condition.condition_type)
                    .ok_or_else(|| {
                        format!("Unknown condition type: {}", condition.condition_type)
                    })?;

                return evaluator.validate(condition);
            } else {
                return Err("Leaf node missing condition".to_string());
            }
        }

        // Branch node - validate operator and children
        if rule_tree.is_branch() {
            let operator = rule_tree
                .operator
                .ok_or_else(|| "Branch node missing operator".to_string())?;

            let conditions = rule_tree
                .conditions
                .as_ref()
                .ok_or_else(|| "Branch node missing conditions".to_string())?;

            if conditions.is_empty() {
                return Err("Branch node must have at least one child".to_string());
            }

            if operator == LogicalOperator::Not && conditions.len() != 1 {
                return Err("NOT operator must have exactly one child".to_string());
            }

            // Validate all children recursively
            for child in conditions {
                self.validate_rule_tree(child)?;
            }

            return Ok(());
        }

        Err("Invalid rule tree structure".to_string())
    }
}

/// Build a stable fingerprint for an evaluation context. The goal is correctness first:
/// include all inputs that can affect a condition result so cached decisions are never
/// reused across materially different contexts.
fn context_fingerprint(ctx: &EvaluationContext) -> u64 {
    let mut s = std::collections::hash_map::DefaultHasher::new();

    // Always include token and current price (high precision formatting)
    ctx.token_mint.hash(&mut s);
    if let Some(p) = ctx.current_price {
        // 12+ decimals to avoid accidental collisions for tiny price changes
        format!("{:.12}", p).hash(&mut s);
    } else {
        "no_price".hash(&mut s);
    }

    // Position-scoped inputs (exit strategies)
    if let Some(pos) = &ctx.position_data {
        // Use time and price; age is derived but include to be safe
        pos.entry_time.timestamp().hash(&mut s);
        format!("{:.12}", pos.entry_price).hash(&mut s);
        format!("{:.6}", pos.position_age_hours).hash(&mut s);
        format!("{:.6}", pos.current_size_sol).hash(&mut s);
        if let Some(pct) = pos.unrealized_profit_pct {
            format!("{:.6}", pct).hash(&mut s);
        } else {
            "no_profit".hash(&mut s);
        }
    } else {
        "no_position".hash(&mut s);
    }

    // Market data inputs commonly used by conditions
    if let Some(m) = &ctx.market_data {
        if let Some(v) = m.liquidity_sol {
            format!("{:.6}", v).hash(&mut s);
        } else {
            "nlq".hash(&mut s);
        }
        if let Some(v) = m.volume_24h {
            format!("{:.2}", v).hash(&mut s);
        } else {
            "nvol".hash(&mut s);
        }
        if let Some(v) = m.market_cap {
            format!("{:.2}", v).hash(&mut s);
        } else {
            "nmc".hash(&mut s);
        }
        if let Some(v) = m.holder_count {
            v.hash(&mut s);
        } else {
            0u32.hash(&mut s);
        }
        if let Some(v) = m.token_age_hours {
            format!("{:.2}", v).hash(&mut s);
        } else {
            "ntag".hash(&mut s);
        }
    } else {
        "no_market".hash(&mut s);
    }

    // TimeframeBundle: hash bundle timestamp for cache invalidation
    if let Some(bundle) = &ctx.timeframe_bundle {
        bundle.timestamp.timestamp().hash(&mut s);
        bundle.cache_age_seconds.hash(&mut s);
    } else {
        "no_ohlcv".hash(&mut s);
    }

    s.finish()
}
