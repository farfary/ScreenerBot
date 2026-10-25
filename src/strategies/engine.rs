// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategy evaluation engine — evaluates condition trees against live market data.

use crate::logger::{self, LogTag};
use crate::strategies::conditions::ConditionRegistry;
use crate::strategies::types::{
    EvaluationContext, EvaluationResult, LogicalOperator, RuleTree, Strategy, StrategyType,
    WindowCounts,
};
use crate::strategies::{Error, Result};
use crate::trader::STRATEGY_CACHE_MAX_ENTRIES;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};

/// Deepest level a rule tree may reach: the root group is level 1, a node directly under
/// it level 2.
const MAX_RULE_DEPTH: usize = 4;

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
    ) -> Result<EvaluationResult> {
        let start = Instant::now();

        if strategy.rules.disabled {
            return Err(Error::InvalidRuleTree {
                reason: "the root group cannot be switched off",
            });
        }

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
                return Err(Error::EvaluationTimeout {
                    timeout_ms: self.config.evaluation_timeout_ms,
                });
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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool>> + Send + 'a>> {
        Box::pin(async move {
            // Leaf node - evaluate condition
            if rule_tree.is_leaf() {
                if let Some(condition) = &rule_tree.condition {
                    let evaluator = self
                        .condition_registry
                        .get(&condition.condition_type)
                        .ok_or_else(|| Error::InvalidConditionValue {
                            field: "condition type",
                            value: condition.condition_type.clone(),
                        })?;

                    // Validate condition first
                    evaluator.validate(condition)?;

                    // Evaluate condition
                    return evaluator.evaluate(condition, context).await;
                } else {
                    return Err(Error::InvalidRuleTree {
                        reason: "leaf node missing condition",
                    });
                }
            }

            // Branch node - evaluate operator
            if rule_tree.is_branch() {
                let operator = rule_tree.operator.ok_or_else(|| Error::InvalidRuleTree {
                    reason: "branch node missing operator",
                })?;

                let conditions =
                    rule_tree
                        .conditions
                        .as_ref()
                        .ok_or_else(|| Error::InvalidRuleTree {
                            reason: "branch node missing conditions",
                        })?;

                return match operator {
                    LogicalOperator::And => {
                        // Short-circuit AND: return false on first false
                        for child in enabled_children(conditions)? {
                            let result = self.evaluate_rule_tree(child, context).await?;
                            if !result {
                                return Ok(false);
                            }
                        }
                        Ok(true)
                    }
                    LogicalOperator::Or => {
                        // Short-circuit OR: return true on first true
                        for child in enabled_children(conditions)? {
                            let result = self.evaluate_rule_tree(child, context).await?;
                            if result {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    }
                    LogicalOperator::Not => {
                        if conditions.len() != 1 {
                            return Err(Error::InvalidRuleTree {
                                reason: "NOT operator must have exactly one child",
                            });
                        }
                        if conditions[0].disabled {
                            return Err(Error::InvalidRuleTree {
                                reason: "NOT operator cannot have a switched-off child",
                            });
                        }
                        let result = self.evaluate_rule_tree(&conditions[0], context).await?;
                        Ok(!result)
                    }
                };
            }

            Err(Error::InvalidRuleTree {
                reason: "invalid rule tree structure",
            })
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
    pub fn validate_strategy(&self, strategy: &Strategy) -> Result<()> {
        self.validate_rule_tree(&strategy.rules, 1, strategy.strategy_type)
    }

    /// Validate a rule tree recursively. `depth` is the level of `rule_tree`: the root is 1
    /// and a node directly under it is 2.
    fn validate_rule_tree(
        &self,
        rule_tree: &RuleTree,
        depth: usize,
        strategy_type: StrategyType,
    ) -> Result<()> {
        if depth > MAX_RULE_DEPTH {
            return Err(Error::InvalidRuleTree {
                reason: "rules nest deeper than four levels",
            });
        }

        if depth == 1 && rule_tree.disabled {
            return Err(Error::InvalidRuleTree {
                reason: "the root group cannot be switched off",
            });
        }

        // Leaf node - validate condition
        if rule_tree.is_leaf() {
            if let Some(condition) = &rule_tree.condition {
                let evaluator = self
                    .condition_registry
                    .get(&condition.condition_type)
                    .ok_or_else(|| Error::InvalidConditionValue {
                        field: "condition type",
                        value: condition.condition_type.clone(),
                    })?;

                if !evaluator.applies_to().contains(&strategy_type) {
                    return Err(Error::ConditionNotApplicable {
                        condition_type: condition.condition_type.clone(),
                        strategy_type,
                    });
                }

                return evaluator.validate(condition);
            } else {
                return Err(Error::InvalidRuleTree {
                    reason: "leaf node missing condition",
                });
            }
        }

        // Branch node - validate operator and children
        if rule_tree.is_branch() {
            let operator = rule_tree.operator.ok_or_else(|| Error::InvalidRuleTree {
                reason: "branch node missing operator",
            })?;

            let conditions =
                rule_tree
                    .conditions
                    .as_ref()
                    .ok_or_else(|| Error::InvalidRuleTree {
                        reason: "branch node missing conditions",
                    })?;

            if conditions.is_empty() {
                return Err(Error::InvalidRuleTree {
                    reason: "branch node must have at least one child",
                });
            }

            if operator == LogicalOperator::Not {
                if conditions.len() != 1 {
                    return Err(Error::InvalidRuleTree {
                        reason: "NOT operator must have exactly one child",
                    });
                }
                if conditions[0].disabled {
                    return Err(Error::InvalidRuleTree {
                        reason: "NOT operator cannot have a switched-off child",
                    });
                }
            }

            if conditions.iter().all(|child| child.disabled) {
                return Err(Error::InvalidRuleTree {
                    reason: "a group needs at least one condition switched on",
                });
            }

            // Validate all children recursively, switched-off ones included: a condition
            // that is saved but off must still be well-formed for when it is switched on.
            for child in conditions {
                self.validate_rule_tree(child, depth + 1, strategy_type)?;
            }

            return Ok(());
        }

        Err(Error::InvalidRuleTree {
            reason: "invalid rule tree structure",
        })
    }
}

/// The children of a group that are switched on. A group with none has nothing to decide
/// it: an empty AND would be true and an empty OR false, so it is refused instead.
fn enabled_children(children: &[RuleTree]) -> Result<Vec<&RuleTree>> {
    let enabled: Vec<&RuleTree> = children.iter().filter(|child| !child.disabled).collect();
    if enabled.is_empty() {
        return Err(Error::InvalidRuleTree {
            reason: "a group needs at least one condition switched on",
        });
    }
    Ok(enabled)
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
        if let Some(peak) = pos.peak_price {
            format!("{:.12}", peak).hash(&mut s);
        } else {
            "no_peak".hash(&mut s);
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
        if let Some(counts) = &m.transactions {
            for window in [counts.m5, counts.h1, counts.h6, counts.h24] {
                hash_window(window, &mut s);
            }
        } else {
            "no_txns".hash(&mut s);
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

/// Hash one transaction window; a window never reported is told apart from zero counts.
fn hash_window(window: Option<WindowCounts>, hasher: &mut impl Hasher) {
    match window {
        Some(counts) => {
            counts.buys.hash(hasher);
            counts.sells.hash(hasher);
        }
        None => "no_window".hash(hasher),
    }
}
