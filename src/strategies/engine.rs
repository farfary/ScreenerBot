// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategy evaluation engine — evaluates condition trees against live market data.

use crate::logger::{self, LogTag};
use crate::strategies::conditions::ConditionRegistry;
use crate::strategies::types::{
    Condition, EvaluationContext, EvaluationResult, LiquidityPool, LogicalOperator, MarketData,
    PositionData, RuleOutcome, RuleTree, RuleVerdict, Strategy, StrategyType, TransactionCounts,
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

/// Truth value of a rule-tree node under three-valued (Kleene) logic.
///
/// A condition whose data is absent, too short, stale or unusable is `Unknown`, carrying
/// the error that made it so. It is never read as true or false: an `And` it belongs to is
/// still decided by a false sibling, an `Or` by a true one, and anything left undecided
/// reports the data error instead of a signal.
#[derive(Debug)]
enum Truth {
    True,
    False,
    Unknown(Error),
}

impl From<bool> for Truth {
    fn from(value: bool) -> Self {
        if value {
            Truth::True
        } else {
            Truth::False
        }
    }
}

impl Truth {
    fn negate(self) -> Self {
        match self {
            Truth::True => Truth::False,
            Truth::False => Truth::True,
            unknown @ Truth::Unknown(_) => unknown,
        }
    }

    /// The outcome of a node that reached this value.
    fn outcome(
        &self,
        operator: Option<LogicalOperator>,
        children: Vec<RuleOutcome>,
    ) -> RuleOutcome {
        let (verdict, reason) = match self {
            Truth::True => (RuleVerdict::Pass, None),
            Truth::False => (RuleVerdict::Fail, None),
            Truth::Unknown(e) => (RuleVerdict::Unknown, Some(e.ui_text())),
        };
        RuleOutcome {
            verdict,
            operator,
            condition_type: None,
            reason,
            children,
        }
    }
}

/// The value of an And or Or group, folded one child at a time under three-valued logic.
struct GroupFold {
    /// The child value that decides the group on its own: true for Or, false for And.
    decisive: bool,
    decided: bool,
    first_unknown: Option<Error>,
}

impl GroupFold {
    fn new(operator: LogicalOperator) -> Self {
        Self {
            decisive: operator == LogicalOperator::Or,
            decided: false,
            first_unknown: None,
        }
    }

    /// Fold one child's value; true once the group is decided whatever the children
    /// still to come are.
    fn push(&mut self, child: Truth) -> bool {
        match child {
            Truth::True if self.decisive => self.decided = true,
            Truth::False if !self.decisive => self.decided = true,
            Truth::Unknown(e) => {
                self.first_unknown.get_or_insert(e);
            }
            Truth::True | Truth::False => {}
        }
        self.decided
    }

    fn finish(self) -> Truth {
        if self.decided {
            return Truth::from(self.decisive);
        }
        match self.first_unknown {
            Some(e) => Truth::Unknown(e),
            None => Truth::from(!self.decisive),
        }
    }
}

/// The shape of one rule tree node, checked.
enum NodeKind<'a> {
    Leaf(&'a Condition),
    Group(LogicalOperator, &'a [RuleTree]),
    Not(&'a RuleTree),
}

fn node_kind(rule_tree: &RuleTree) -> Result<NodeKind<'_>> {
    if rule_tree.is_leaf() {
        return rule_tree
            .condition
            .as_ref()
            .map(NodeKind::Leaf)
            .ok_or(Error::InvalidRuleTree {
                reason: "leaf node missing condition",
            });
    }
    if !rule_tree.is_branch() {
        return Err(Error::InvalidRuleTree {
            reason: "invalid rule tree structure",
        });
    }
    let operator = rule_tree.operator.ok_or(Error::InvalidRuleTree {
        reason: "branch node missing operator",
    })?;
    let conditions = rule_tree
        .conditions
        .as_deref()
        .ok_or(Error::InvalidRuleTree {
            reason: "branch node missing conditions",
        })?;
    match operator {
        LogicalOperator::And | LogicalOperator::Or => Ok(NodeKind::Group(operator, conditions)),
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
            Ok(NodeKind::Not(&conditions[0]))
        }
    }
}

/// The outcome of a switched-off node and of every node under it: off, not evaluated.
fn switched_off(rule_tree: &RuleTree) -> RuleOutcome {
    RuleOutcome {
        verdict: RuleVerdict::Off,
        operator: rule_tree.operator,
        condition_type: rule_tree
            .condition
            .as_ref()
            .map(|condition| condition.condition_type.clone()),
        reason: None,
        children: rule_tree
            .conditions
            .iter()
            .flatten()
            .map(switched_off)
            .collect(),
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
            Ok(Ok(Truth::True)) => true,
            Ok(Ok(Truth::False)) => false,
            // An undecided root is no signal, reported as the first data error it reached.
            Ok(Ok(Truth::Unknown(e))) | Ok(Err(e)) => {
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

    /// Evaluate every node of a strategy's rule tree, switched-off nodes reported as off,
    /// to show why the strategy does or does not signal.
    ///
    /// The root's verdict is the one [`Self::evaluate_strategy`] reaches: a group folds its
    /// children with the same three-valued rule, only without stopping at the first
    /// decisive child, so every condition reports its own verdict. Nothing is cached.
    pub async fn explain_strategy(
        &self,
        strategy: &Strategy,
        context: &EvaluationContext,
    ) -> Result<RuleOutcome> {
        if strategy.rules.disabled {
            return Err(Error::InvalidRuleTree {
                reason: "the root group cannot be switched off",
            });
        }
        let timeout_duration = Duration::from_millis(self.config.evaluation_timeout_ms);
        match timeout(
            timeout_duration,
            self.explain_rule_tree(&strategy.rules, context),
        )
        .await
        {
            Ok(result) => result.map(|(outcome, _)| outcome),
            Err(_) => Err(Error::EvaluationTimeout {
                timeout_ms: self.config.evaluation_timeout_ms,
            }),
        }
    }

    /// Evaluate a rule tree recursively under three-valued logic.
    ///
    /// A leaf whose condition fails for lack of data ([`Error::is_data_unavailable`]) is
    /// `Unknown`; any other error is a fault in the strategy and propagates at once.
    /// - And: any False is False; else any Unknown is Unknown; else True.
    /// - Or: any True is True; else any Unknown is Unknown; else False.
    /// - Not: Unknown stays Unknown.
    ///
    /// A group stops at its first decisive child (False for And, True for Or) and never at
    /// an Unknown one, so a later child can still decide it. An undecided group carries
    /// the first data error it reached.
    fn evaluate_rule_tree<'a>(
        &'a self,
        rule_tree: &'a RuleTree,
        context: &'a EvaluationContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Truth>> + Send + 'a>> {
        Box::pin(async move {
            match node_kind(rule_tree)? {
                NodeKind::Leaf(condition) => self.evaluate_leaf(condition, context).await,
                NodeKind::Group(operator, conditions) => {
                    let mut fold = GroupFold::new(operator);
                    for child in enabled_children(conditions)? {
                        if fold.push(self.evaluate_rule_tree(child, context).await?) {
                            break;
                        }
                    }
                    Ok(fold.finish())
                }
                NodeKind::Not(child) => Ok(self.evaluate_rule_tree(child, context).await?.negate()),
            }
        })
    }

    /// [`Self::evaluate_rule_tree`] for every enabled node, recording each node's outcome.
    fn explain_rule_tree<'a>(
        &'a self,
        rule_tree: &'a RuleTree,
        context: &'a EvaluationContext,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(RuleOutcome, Truth)>> + Send + 'a>,
    > {
        Box::pin(async move {
            match node_kind(rule_tree)? {
                NodeKind::Leaf(condition) => {
                    let truth = self.evaluate_leaf(condition, context).await?;
                    let outcome = RuleOutcome {
                        condition_type: Some(condition.condition_type.clone()),
                        ..truth.outcome(None, Vec::new())
                    };
                    Ok((outcome, truth))
                }
                NodeKind::Group(operator, conditions) => {
                    enabled_children(conditions)?;
                    let mut fold = GroupFold::new(operator);
                    let mut children = Vec::with_capacity(conditions.len());
                    for child in conditions {
                        if child.disabled {
                            children.push(switched_off(child));
                            continue;
                        }
                        let (outcome, truth) = self.explain_rule_tree(child, context).await?;
                        fold.push(truth);
                        children.push(outcome);
                    }
                    let truth = fold.finish();
                    Ok((truth.outcome(Some(operator), children), truth))
                }
                NodeKind::Not(child) => {
                    let (outcome, truth) = self.explain_rule_tree(child, context).await?;
                    let truth = truth.negate();
                    Ok((
                        truth.outcome(Some(LogicalOperator::Not), vec![outcome]),
                        truth,
                    ))
                }
            }
        })
    }

    /// Evaluate one condition: a lack of data is `Unknown`, any other error a fault.
    async fn evaluate_leaf(
        &self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> Result<Truth> {
        let evaluator = self
            .condition_registry
            .get(&condition.condition_type)
            .ok_or_else(|| Error::InvalidConditionValue {
                field: "condition type",
                value: condition.condition_type.clone(),
            })?;

        evaluator.validate(condition)?;

        match evaluator.evaluate(condition, context).await {
            Ok(value) => Ok(Truth::from(value)),
            Err(e) if e.is_data_unavailable() => Ok(Truth::Unknown(e)),
            Err(e) => Err(e),
        }
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
///
/// Every context struct is destructured without `..`, so a field added to the context
/// does not compile until it is hashed here (or named and deliberately skipped).
fn context_fingerprint(ctx: &EvaluationContext) -> u64 {
    let mut s = std::collections::hash_map::DefaultHasher::new();
    let EvaluationContext {
        token_mint,
        current_price,
        position_data,
        market_data,
        timeframe_bundle,
        strategy_timeframe,
        evaluated_at,
    } = ctx;

    // Always include token and current price (high precision formatting)
    token_mint.hash(&mut s);
    // 12+ decimals to avoid accidental collisions for tiny price changes
    hash_number(*current_price, 12, &mut s);
    strategy_timeframe.hash(&mut s);
    // Time-derived conditions are exact to the minute; the cache TTL bounds the rest.
    evaluated_at.timestamp().div_euclid(60).hash(&mut s);

    // Position-scoped inputs (exit strategies)
    if let Some(PositionData {
        entry_price,
        entry_time,
        current_size_sol,
        unrealized_profit_pct,
        position_age_hours,
        peak_price,
        dca_count,
        partial_exit_count,
        remaining_value_native,
        entry_liquidity_native,
        entry_liquidity_pool,
    }) = position_data
    {
        entry_time.timestamp().hash(&mut s);
        hash_number(Some(*entry_price), 12, &mut s);
        hash_number(Some(*position_age_hours), 6, &mut s);
        hash_number(Some(*current_size_sol), 6, &mut s);
        hash_number(*unrealized_profit_pct, 6, &mut s);
        hash_number(*peak_price, 12, &mut s);
        dca_count.hash(&mut s);
        partial_exit_count.hash(&mut s);
        hash_number(*remaining_value_native, 9, &mut s);
        hash_number(*entry_liquidity_native, 6, &mut s);
        entry_liquidity_pool.hash(&mut s);
    } else {
        "no_position".hash(&mut s);
    }

    // Market data inputs commonly used by conditions
    if let Some(MarketData {
        liquidity_sol,
        liquidity_pool,
        transactions,
        snapshot_at,
        token_created_at,
        reference_size_native,
    }) = market_data
    {
        hash_number(*liquidity_sol, 6, &mut s);
        if let Some(LiquidityPool {
            address,
            virtual_reserves,
        }) = liquidity_pool
        {
            address.hash(&mut s);
            virtual_reserves.hash(&mut s);
        } else {
            "no_pool".hash(&mut s);
        }
        if let Some(TransactionCounts { m5, h1, h6, h24 }) = transactions {
            for window in [m5, h1, h6, h24] {
                hash_window(*window, &mut s);
            }
        } else {
            "no_txns".hash(&mut s);
        }
        snapshot_at.map(|at| at.timestamp()).hash(&mut s);
        token_created_at.map(|at| at.timestamp()).hash(&mut s);
        hash_number(*reference_size_native, 9, &mut s);
    } else {
        "no_market".hash(&mut s);
    }

    // TimeframeBundle: hash bundle timestamp for cache invalidation
    if let Some(bundle) = timeframe_bundle {
        bundle.timestamp.timestamp().hash(&mut s);
        bundle.cache_age_seconds.hash(&mut s);
    } else {
        "no_ohlcv".hash(&mut s);
    }

    s.finish()
}

/// Hash a number at a fixed precision; an absent number is told apart from every value.
fn hash_number(value: Option<f64>, decimals: usize, hasher: &mut impl Hasher) {
    match value {
        Some(value) => format!("{value:.decimals$}").hash(hasher),
        None => "absent".hash(hasher),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ohlcvs::TimeframeBundle;
    use chrono::{DateTime, Duration, Utc};

    /// A minute boundary, so seconds can be added without leaving the minute.
    fn minute_start() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_040, 0).expect("valid instant")
    }

    /// A context with every field present.
    fn full_context() -> EvaluationContext {
        EvaluationContext {
            token_mint: "mint".to_owned(),
            current_price: Some(1.5),
            position_data: Some(PositionData {
                entry_price: 1.0,
                entry_time: minute_start() - Duration::hours(2),
                current_size_sol: 0.5,
                unrealized_profit_pct: Some(50.0),
                position_age_hours: 2.0,
                peak_price: Some(1.8),
                dca_count: 1,
                partial_exit_count: 1,
                remaining_value_native: Some(0.6),
                entry_liquidity_native: Some(140.0),
                entry_liquidity_pool: Some("pool".to_owned()),
            }),
            market_data: Some(MarketData {
                liquidity_sol: Some(120.0),
                liquidity_pool: Some(LiquidityPool {
                    address: "pool".to_owned(),
                    virtual_reserves: Some(false),
                }),
                transactions: Some(TransactionCounts {
                    m5: Some(WindowCounts { buys: 5, sells: 3 }),
                    h1: Some(WindowCounts {
                        buys: 50,
                        sells: 30,
                    }),
                    h6: Some(WindowCounts {
                        buys: 300,
                        sells: 200,
                    }),
                    h24: Some(WindowCounts {
                        buys: 900,
                        sells: 800,
                    }),
                }),
                snapshot_at: Some(minute_start() - Duration::seconds(30)),
                token_created_at: Some(minute_start() - Duration::days(3)),
                reference_size_native: Some(0.1),
            }),
            timeframe_bundle: Some(TimeframeBundle::new("mint".to_owned(), "pool".to_owned())),
            strategy_timeframe: "5m".to_owned(),
            evaluated_at: minute_start(),
        }
    }

    fn position(ctx: &mut EvaluationContext) -> &mut PositionData {
        ctx.position_data.as_mut().expect("position data")
    }

    fn market(ctx: &mut EvaluationContext) -> &mut MarketData {
        ctx.market_data.as_mut().expect("market data")
    }

    /// Every input a condition can read changes the fingerprint, so a cached answer is
    /// never served for a context it was not computed for. With the exhaustive
    /// destructuring in `context_fingerprint`, a new field fails to compile there and
    /// earns a line here.
    #[test]
    fn every_context_field_changes_the_fingerprint() {
        type Change = fn(&mut EvaluationContext);
        let changes: [(&str, Change); 27] = [
            ("token_mint", |c| c.token_mint = "other".to_owned()),
            ("current_price", |c| {
                c.current_price = Some(1.500_000_000_001)
            }),
            ("position_data", |c| c.position_data = None),
            ("entry_price", |c| position(c).entry_price = 1.1),
            ("entry_time", |c| {
                position(c).entry_time += Duration::seconds(1)
            }),
            ("current_size_sol", |c| position(c).current_size_sol = 0.6),
            ("unrealized_profit_pct", |c| {
                position(c).unrealized_profit_pct = None
            }),
            ("position_age_hours", |c| {
                position(c).position_age_hours = 2.5
            }),
            ("peak_price", |c| position(c).peak_price = Some(1.9)),
            ("dca_count", |c| position(c).dca_count = 2),
            ("partial_exit_count", |c| position(c).partial_exit_count = 2),
            ("remaining_value_native", |c| {
                position(c).remaining_value_native = None
            }),
            ("entry_liquidity_native", |c| {
                position(c).entry_liquidity_native = None
            }),
            ("entry_liquidity_pool", |c| {
                position(c).entry_liquidity_pool = Some("other".to_owned())
            }),
            ("market_data", |c| c.market_data = None),
            ("liquidity_sol", |c| market(c).liquidity_sol = Some(121.0)),
            ("liquidity_pool", |c| market(c).liquidity_pool = None),
            ("liquidity pool address", |c| {
                market(c).liquidity_pool.as_mut().expect("pool").address = "other".to_owned()
            }),
            ("liquidity pool reserves", |c| {
                market(c)
                    .liquidity_pool
                    .as_mut()
                    .expect("pool")
                    .virtual_reserves = Some(true)
            }),
            ("transactions", |c| {
                market(c).transactions.as_mut().expect("counts").h1 = Some(WindowCounts {
                    buys: 51,
                    sells: 30,
                })
            }),
            ("snapshot_at", |c| {
                market(c).snapshot_at = Some(minute_start() - Duration::seconds(29))
            }),
            ("token_created_at", |c| market(c).token_created_at = None),
            ("reference_size_native", |c| {
                market(c).reference_size_native = Some(0.2)
            }),
            ("timeframe_bundle", |c| c.timeframe_bundle = None),
            ("bundle age", |c| {
                c.timeframe_bundle
                    .as_mut()
                    .expect("bundle")
                    .cache_age_seconds = 9
            }),
            ("strategy_timeframe", |c| {
                c.strategy_timeframe = "1h".to_owned()
            }),
            ("evaluated_at", |c| c.evaluated_at += Duration::minutes(1)),
        ];

        let base = context_fingerprint(&full_context());
        for (field, change) in changes {
            let mut ctx = full_context();
            change(&mut ctx);
            assert_ne!(
                context_fingerprint(&ctx),
                base,
                "{field} is not fingerprinted"
            );
        }
    }

    #[test]
    fn the_evaluation_instant_counts_to_the_minute() {
        let base = context_fingerprint(&full_context());
        let mut later = full_context();
        later.evaluated_at += Duration::seconds(59);
        assert_eq!(context_fingerprint(&later), base);
    }
}
