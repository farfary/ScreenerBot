/// Trading configuration constants
pub const PRICE_DROP_THRESHOLD_PERCENT: f64 = 7.5;
pub const PROFIT_THRESHOLD_PERCENT: f64 = 5.0;

pub const DEFAULT_FEE: f64 = 0.000006;
// pub const DEFAULT_FEE: f64 = 0.0;

pub const DEFAULT_FEE_SWAP: f64 = 0.000001;
pub const DEFAULT_SLIPPAGE: f64 = 3.0; // 5% slippage

pub const TRADE_SIZE_SOL: f64 = 0.0001;
pub const STOP_LOSS_PERCENT: f64 = -99.0;
pub const PRICE_HISTORY_HOURS: i64 = 24;
pub const NEW_ENTRIES_CHECK_INTERVAL_SECS: u64 = 5;
pub const OPEN_POSITIONS_CHECK_INTERVAL_SECS: u64 = 5;
pub const MAX_OPEN_POSITIONS: usize = 10;

/// ATA (Associated Token Account) management configuration
pub const CLOSE_ATA_AFTER_SELL: bool = true; // Set to false to disable ATA closing

use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };
use crate::global::*;
use crate::positions::{
    Position,
    calculate_position_pnl,
    update_position_tracking,
    get_open_positions_count,
    open_position,
    close_position,
    SAVED_POSITIONS,
};
use crate::summary::*;
use crate::utils::*;

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{ Arc as StdArc, Mutex as StdMutex };
use chrono::{ Utc, Duration as ChronoDuration, DateTime };
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;
use colored::Colorize;

/// Static global: price history for each token (mint), stores Vec<(timestamp, price)>
pub static PRICE_HISTORY_24H: Lazy<
    StdArc<StdMutex<HashMap<String, Vec<(DateTime<Utc>, f64)>>>>
> = Lazy::new(|| StdArc::new(StdMutex::new(HashMap::new())));

/// Static global: last known prices for each token
pub static LAST_PRICES: Lazy<StdArc<StdMutex<HashMap<String, f64>>>> = Lazy::new(|| {
    StdArc::new(StdMutex::new(HashMap::new()))
});

pub fn should_sell(pos: &Position, current_price: f64, now: DateTime<Utc>) -> f64 {
    // Calculate time held in seconds using total_seconds()
    let duration = now - pos.entry_time;
    let time_held_secs: f64 = duration.num_seconds() as f64;

    // Conservative settings for simplified logic
    const MIN_HOLD_TIME_SECS: f64 = 120.0; // Hold for at least 3 minutes
    const STOP_LOSS_PERCENT: f64 = -70.0; // Stop loss at -70%
    const PROFIT_TARGET_PERCENT: f64 = 25.0; // Take profit at +25%
    const MAX_HOLD_TIME_SECS: f64 = 3600.0; // Max 1 hour hold
    const TIME_DECAY_START_SECS: f64 = 1800.0; // Start time decay after 30 minutes

    // Don't sell too early unless it's a major loss
    if time_held_secs < MIN_HOLD_TIME_SECS {
        let (_, current_pnl_percent) = calculate_position_pnl(pos, Some(current_price));

        if current_pnl_percent <= STOP_LOSS_PERCENT {
            return 1.0; // Emergency exit for major losses
        } else {
            return 0.0; // Hold for minimum time
        }
    }

    // Calculate current P&L using unified function
    let (_, current_pnl_percent) = calculate_position_pnl(pos, Some(current_price));

    // Decision logic
    let stop_loss_triggered: bool = current_pnl_percent <= STOP_LOSS_PERCENT;
    let profit_target_reached: bool = current_pnl_percent >= PROFIT_TARGET_PERCENT;

    // Time decay factor
    let time_decay_factor: f64 = if time_held_secs > TIME_DECAY_START_SECS {
        let decay_duration = MAX_HOLD_TIME_SECS - TIME_DECAY_START_SECS;
        let excess_time = time_held_secs - TIME_DECAY_START_SECS;
        let time_decay = excess_time / decay_duration;
        f64::min(time_decay, 1.0)
    } else {
        0.0
    };

    // Calculate urgency
    let mut urgency: f64 = 0.0;

    if stop_loss_triggered {
        urgency = 1.0;
    } else if profit_target_reached {
        urgency = 0.8;
    } else {
        urgency = time_decay_factor * 0.4; // Reduced time pressure
    }

    // Less aggressive selling for positions with small losses
    if
        time_held_secs > TIME_DECAY_START_SECS &&
        current_pnl_percent <= 0.0 &&
        current_pnl_percent > -30.0
    {
        urgency = f64::max(urgency, 0.3); // Reduced urgency for small losses
    }

    urgency = f64::max(0.0, f64::min(urgency, 1.0));
    urgency
}

/// Get current price for a token from the global token list
pub fn get_current_token_price(mint: &str) -> Option<f64> {
    let tokens = LIST_TOKENS.read().unwrap();

    // Find the token by mint address
    for token in tokens.iter() {
        if token.mint == mint {
            // Try to get the best available price (prioritize DexScreener SOL price)
            if let Some(price) = token.price_dexscreener_sol {
                return Some(price);
            }
            // Fallback to other price sources
            if let Some(price) = token.price_geckoterminal_sol {
                return Some(price);
            }
            if let Some(price) = token.price_raydium_sol {
                return Some(price);
            }
            if let Some(price) = token.price_pool_sol {
                return Some(price);
            }
        }
    }

    None
}

/// Validates if a token has all required metadata for trading
pub fn validate_token(token: &Token) -> bool {
    !token.symbol.is_empty() &&
        !token.mint.is_empty() &&
        token.price_dexscreener_sol.is_some() &&
        token.liquidity.is_some()
}

/// Checks if entry is allowed based on historical position data for this token
/// Returns true only if current price is below both:
/// 1. Average entry price from past closed positions
/// 2. Maximum price this token has ever reached
pub fn is_entry_allowed_by_historical_data(mint: &str, current_price: f64) -> bool {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        // Find all closed positions for this token
        let token_positions: Vec<&Position> = positions
            .iter()
            .filter(|p| p.mint == mint && p.exit_price.is_some())
            .collect();

        // If no historical positions, allow entry (first time seeing this token)
        if token_positions.is_empty() {
            log(
                LogTag::Trader,
                "INFO",
                &format!(
                    "No historical positions found for token {}, allowing entry at {:.12}",
                    mint,
                    current_price
                )
            );
            return true;
        }

        // Calculate average entry price from past positions
        let total_entry_prices: f64 = token_positions
            .iter()
            .map(|p| p.effective_entry_price.unwrap_or(p.entry_price))
            .sum();
        let average_entry_price = total_entry_prices / (token_positions.len() as f64);

        // Find maximum price this token has ever reached
        let max_historical_price = token_positions
            .iter()
            .map(|p| p.price_highest)
            .fold(0.0, f64::max);

        // Log the analysis
        log(
            LogTag::Trader,
            "ANALYSIS",
            &format!(
                "Historical analysis for {}: Current: {:.12}, Avg Entry: {:.12}, Max Ever: {:.12}, Positions: {}",
                mint,
                current_price,
                average_entry_price,
                max_historical_price,
                token_positions.len()
            )
        );

        // Allow entry only if current price is below both thresholds
        let below_avg_entry = current_price < average_entry_price;
        let below_max_price = current_price < max_historical_price;

        if !below_avg_entry {
            log(
                LogTag::Trader,
                "BLOCK",
                &format!(
                    "Entry blocked: Current price {:.12} >= average entry price {:.12}",
                    current_price,
                    average_entry_price
                )
            );
        }

        if !below_max_price {
            log(
                LogTag::Trader,
                "BLOCK",
                &format!(
                    "Entry blocked: Current price {:.12} >= maximum historical price {:.12}",
                    current_price,
                    max_historical_price
                )
            );
        }

        if below_avg_entry && below_max_price {
            log(
                LogTag::Trader,
                "ALLOW",
                &format!(
                    "Entry allowed: Current price {:.12} < avg entry {:.12} and < max price {:.12}",
                    current_price,
                    average_entry_price,
                    max_historical_price
                )
            );
        }

        return below_avg_entry && below_max_price;
    } else {
        log(
            LogTag::Trader,
            "ERROR",
            "Could not acquire lock on SAVED_POSITIONS for historical analysis"
        );
        return false; // Conservative: don't allow entry if we can't analyze
    }
}

/// Background task to monitor new tokens for entry opportunities
pub async fn monitor_new_entries(shutdown: Arc<Notify>) {
    loop {
        // Add a maximum processing time for the entire token checking cycle
        let cycle_start = std::time::Instant::now();

        let mut tokens: Vec<_> = {
            if let Ok(tokens_guard) = LIST_TOKENS.read() {
                // Log total tokens available
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!("Total tokens in LIST_TOKENS: {}", tokens_guard.len())
                        .dimmed()
                        .to_string()
                );

                // Include all tokens - we want to trade on existing tokens with updated info
                // The discovery system ensures tokens are updated with fresh data before trading
                let all_tokens: Vec<_> = tokens_guard.iter().cloned().collect();

                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!(
                        "Using all {} tokens for trading (startup filter removed)",
                        all_tokens.len()
                    )
                        .dimmed()
                        .to_string()
                );

                // Count tokens with liquidity data
                let with_liquidity = all_tokens
                    .iter()
                    .filter(|token| {
                        token.liquidity
                            .as_ref()
                            .and_then(|l| l.usd)
                            .unwrap_or(0.0) > 0.0
                    })
                    .count();

                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!("Tokens with non-zero liquidity: {}", with_liquidity)
                        .dimmed()
                        .to_string()
                );

                all_tokens
            } else {
                log(LogTag::Trader, "ERROR", "Failed to acquire read lock on LIST_TOKENS");
                Vec::new()
            }
        };

        // Sort tokens by liquidity in descending order (highest liquidity first)
        tokens.sort_by(|a, b| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Safety check - if processing is taking too long, log it
        if cycle_start.elapsed() > Duration::from_secs(5) {
            log(
                LogTag::Trader,
                "WARN",
                &format!("Token sorting took too long: {:?}", cycle_start.elapsed())
            );
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!(
                "Checking {} tokens for entry opportunities (sorted by liquidity)",
                tokens.len()
            )
                .dimmed()
                .to_string()
        );

        // Count tokens with zero liquidity before filtering
        let zero_liquidity_count = tokens
            .iter()
            .filter(|token| {
                let liquidity_usd = token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);
                liquidity_usd == 0.0
            })
            .count();

        if zero_liquidity_count > 0 {
            log(
                LogTag::Trader,
                "WARN",
                &format!("Found {} tokens with zero liquidity USD", zero_liquidity_count)
                    .dimmed()
                    .to_string()
            );
        }

        // Filter out zero-liquidity tokens first
        tokens.retain(|token| {
            let liquidity_usd = token.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_usd > 0.0
        });

        log(
            LogTag::Trader,
            "INFO",
            &format!("Processing {} tokens with non-zero liquidity", tokens.len())
                .dimmed()
                .to_string()
        );

        // Early return if no tokens to process
        if tokens.is_empty() {
            log(LogTag::Trader, "INFO", "No tokens to process, skipping token checking cycle");

            // Calculate how long we've spent in this cycle
            let cycle_duration = cycle_start.elapsed();
            let wait_time = if
                cycle_duration >= Duration::from_secs(NEW_ENTRIES_CHECK_INTERVAL_SECS)
            {
                Duration::from_millis(100)
            } else {
                Duration::from_secs(NEW_ENTRIES_CHECK_INTERVAL_SECS) - cycle_duration
            };

            if check_shutdown_or_delay(&shutdown, wait_time).await {
                log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
                break;
            }
            continue;
        }

        // Use a semaphore to limit the number of concurrent token checks
        // This balances between parallelism and not overwhelming external APIs
        use tokio::sync::Semaphore;
        let semaphore = Arc::new(Semaphore::new(5)); // Reduced to 5 concurrent checks to avoid overwhelming

        log(
            LogTag::Trader,
            "INFO",
            &format!("Starting to spawn {} token checking tasks", tokens.len()).dimmed().to_string()
        );

        // Process all tokens in parallel with concurrent tasks
        let mut handles = Vec::new();

        // Get the total token count before starting the loop
        let total_tokens = tokens.len();

        // Note: tokens are still sorted by liquidity from highest to lowest
        for (index, token) in tokens.iter().enumerate() {
            // Check for shutdown before spawning tasks
            if check_shutdown_or_delay(&shutdown, Duration::from_millis(10)).await {
                log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
                return;
            }

            // Get permit from semaphore to limit concurrency with timeout
            let permit = match
                tokio::time::timeout(
                    Duration::from_secs(120),
                    semaphore.clone().acquire_owned()
                ).await
            {
                Ok(Ok(permit)) => permit,
                Ok(Err(e)) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Failed to acquire semaphore permit: {}", e)
                    );
                    continue;
                }
                Err(_) => {
                    log(LogTag::Trader, "WARN", "Semaphore acquire timed out after 10 seconds");
                    continue;
                }
            };

            // Clone necessary variables for the task
            let token = token.clone();
            let index = index; // Capture the index
            let total = total_tokens; // Capture the total

            // Spawn a new task for this token with overall timeout
            let handle = tokio::spawn(async move {
                // Keep the permit alive for the duration of this task
                let _permit = permit; // This will be automatically dropped when the task completes

                // Clone the symbol for error logging before moving token into timeout
                let token_symbol = token.symbol.clone();

                // Wrap the entire task logic in a timeout to prevent hanging
                match
                    tokio::time::timeout(Duration::from_secs(30), async {
                        if let Some(current_price) = token.price_dexscreener_sol {
                            if current_price <= 0.0 || !validate_token(&token) {
                                return None;
                            }

                            let liquidity_usd = token.liquidity
                                .as_ref()
                                .and_then(|l| l.usd)
                                .unwrap_or(0.0);

                            // log(
                            //     LogTag::Trader,
                            //     "DEBUG",
                            //     &format!(
                            //         "Checking token {}/{}: {} ({}) - Price: {:.12} SOL, Liquidity: ${:.2}",
                            //         index + 1,
                            //         total,
                            //         token.symbol,
                            //         token.mint,
                            //         current_price,
                            //         liquidity_usd
                            //     )
                            //         .dimmed()
                            //         .to_string()
                            // );

                            // Update price history with proper error handling and timeout
                            let now = Utc::now();
                            match
                                tokio::time::timeout(Duration::from_millis(500), async {
                                    PRICE_HISTORY_24H.try_lock()
                                }).await
                            {
                                Ok(Ok(mut hist)) => {
                                    let entry = hist
                                        .entry(token.mint.clone())
                                        .or_insert_with(Vec::new);
                                    entry.push((now, current_price));

                                    // Retain only last 24h
                                    let cutoff = now - ChronoDuration::hours(PRICE_HISTORY_HOURS);
                                    entry.retain(|(ts, _)| *ts >= cutoff);
                                }
                                Ok(Err(_)) | Err(_) => {
                                    // If we can't get the lock within 500ms, just log and continue
                                    log(
                                        LogTag::Trader,
                                        "WARN",
                                        &format!(
                                            "Could not acquire price history lock for {} within timeout",
                                            token.symbol
                                        )
                                    );
                                }
                            }

                            // Check for entry opportunity with timeout
                            let mut should_open_position = false;
                            let mut percent_change = 0.0;

                            // Use timeout for last prices mutex as well
                            match
                                tokio::time::timeout(Duration::from_millis(500), async {
                                    LAST_PRICES.try_lock()
                                }).await
                            {
                                Ok(Ok(mut last_prices)) => {
                                    if let Some(&prev_price) = last_prices.get(&token.mint) {
                                        if prev_price > 0.0 {
                                            let change = (current_price - prev_price) / prev_price;
                                            percent_change = change * 100.0;

                                            if percent_change <= -PRICE_DROP_THRESHOLD_PERCENT {
                                                // Check historical data before allowing entry
                                                if
                                                    is_entry_allowed_by_historical_data(
                                                        &token.mint,
                                                        current_price
                                                    )
                                                {
                                                    should_open_position = true;
                                                    log(
                                                        LogTag::Trader,
                                                        "OPPORTUNITY",
                                                        &format!(
                                                            "Entry opportunity detected for {} ({}): {:.2}% price drop, Liquidity: ${:.2}",
                                                            token.symbol,
                                                            token.mint,
                                                            percent_change,
                                                            liquidity_usd
                                                        )
                                                    );
                                                } else {
                                                    log(
                                                        LogTag::Trader,
                                                        "SKIP",
                                                        &format!(
                                                            "Entry blocked for {} ({}): Current price {:.12} not below historical thresholds",
                                                            token.symbol,
                                                            token.mint,
                                                            current_price
                                                        )
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    last_prices.insert(token.mint.clone(), current_price);
                                }
                                Ok(Err(_)) | Err(_) => {
                                    // If we can't get the lock within 500ms, just log and continue
                                    log(
                                        LogTag::Trader,
                                        "WARN",
                                        &format!(
                                            "Could not acquire last_prices lock for {} within timeout",
                                            token.symbol
                                        )
                                    );
                                }
                            }

                            // Return the token, price, and percent change if it's an opportunity
                            if should_open_position {
                                return Some((token, current_price, percent_change));
                            }
                        }
                        None
                    }).await
                {
                    Ok(result) => result,
                    Err(_) => {
                        // Task timed out
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Token check task for {} timed out after 10 seconds", token_symbol)
                        );
                        None
                    }
                }
            });

            handles.push(handle);
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!("Successfully spawned {} token checking tasks", handles.len())
                .dimmed()
                .to_string()
        );

        // Process the results of all tasks with overall timeout
        let collection_result = tokio::time::timeout(Duration::from_secs(120), async {
            // This maintains the priority of processing high-liquidity tokens first
            log(
                LogTag::Trader,
                "INFO",
                &format!("Waiting for {} token checks to complete", handles.len())
                    .dimmed()
                    .to_string()
            );

            let mut opportunities = Vec::new();

            // Collect all opportunities in the order they complete
            let mut completed = 0;
            let total_handles = handles.len();

            for handle in handles {
                // Skip any tasks that failed or if shutdown signal received
                if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                    log(
                        LogTag::Trader,
                        "INFO",
                        "new entries monitor shutting down during result collection..."
                    );
                    return opportunities; // Return what we have so far
                }

                // Add timeout for each handle to prevent getting stuck on a single task
                match tokio::time::timeout(Duration::from_secs(120), handle).await {
                    Ok(task_result) => {
                        match task_result {
                            Ok(Some((token, price, percent_change))) => {
                                opportunities.push((token, price, percent_change));
                            }
                            Ok(None) => {
                                // No opportunity found for this token, continue
                            }
                            Err(e) => {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Token check task failed: {}", e)
                                );
                            }
                        }
                    }
                    Err(_) => {
                        // Task timed out after 5 seconds
                        log(LogTag::Trader, "WARN", "Token check task timed out after 5 seconds");
                    }
                }

                completed += 1;
                if completed % 10 == 0 || completed == total_handles {
                    log(
                        LogTag::Trader,
                        "INFO",
                        &format!("Completed {}/{} token checks", completed, total_handles)
                            .dimmed()
                            .to_string()
                    );
                }
            }

            opportunities
        }).await;

        let mut opportunities = match collection_result {
            Ok(opportunities) => opportunities,
            Err(_) => {
                log(LogTag::Trader, "ERROR", "Token check collection timed out after 60 seconds");
                Vec::new() // Return empty if timeout
            }
        };

        // Sort opportunities by liquidity again to ensure priority
        opportunities.sort_by(|(a, _, _), (b, _, _)| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        log(
            LogTag::Trader,
            "INFO",
            &format!("Found {} potential entry opportunities", opportunities.len())
        );

        // Log the total time taken for the token checking cycle
        log(
            LogTag::Trader,
            "INFO",
            &format!("Token checking cycle completed in {:?}", cycle_start.elapsed())
                .dimmed()
                .to_string()
        );

        // Process opportunities concurrently while respecting position limits
        if !opportunities.is_empty() {
            let current_open_count = get_open_positions_count();
            let available_slots = MAX_OPEN_POSITIONS.saturating_sub(current_open_count);

            if available_slots == 0 {
                log(
                    LogTag::Trader,
                    "LIMIT",
                    &format!(
                        "Maximum open positions already reached ({}/{}). Skipping all opportunities.",
                        current_open_count,
                        MAX_OPEN_POSITIONS
                    )
                );
            } else {
                // Limit opportunities to available slots
                let opportunities_to_process = opportunities
                    .into_iter()
                    .take(available_slots)
                    .collect::<Vec<_>>();

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!(
                        "Processing {} opportunities concurrently (available slots: {}, current open: {})",
                        opportunities_to_process.len(),
                        available_slots,
                        current_open_count
                    )
                );

                // Use a semaphore to limit concurrent buy transactions
                use tokio::sync::Semaphore;
                let semaphore = Arc::new(Semaphore::new(3)); // Allow up to 3 concurrent buys

                let mut handles = Vec::new();

                // Process all buy orders concurrently
                for (token, price, percent_change) in opportunities_to_process {
                    // Check for shutdown before spawning tasks
                    if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                        log(
                            LogTag::Trader,
                            "INFO",
                            "new entries monitor shutting down during buy processing..."
                        );
                        break;
                    }

                    // Get permit from semaphore to limit concurrency with timeout
                    let permit = match
                        tokio::time::timeout(
                            Duration::from_secs(120),
                            semaphore.clone().acquire_owned()
                        ).await
                    {
                        Ok(Ok(permit)) => permit,
                        Ok(Err(e)) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Failed to acquire semaphore permit for buy: {}", e)
                            );
                            continue;
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "WARN",
                                "Semaphore acquire timed out for buy operation"
                            );
                            continue;
                        }
                    };

                    let handle = tokio::spawn(async move {
                        let _permit = permit; // Keep permit alive for duration of task

                        let token_symbol = token.symbol.clone();

                        // Wrap the buy operation in a timeout
                        match
                            tokio::time::timeout(Duration::from_secs(120), async {
                                open_position(&token, price, percent_change).await
                            }).await
                        {
                            Ok(_) => {
                                log(
                                    LogTag::Trader,
                                    "SUCCESS",
                                    &format!("Completed buy operation for {} in concurrent task", token_symbol)
                                );
                                true
                            }
                            Err(_) => {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Buy operation for {} timed out after 20 seconds", token_symbol)
                                );
                                false
                            }
                        }
                    });

                    handles.push(handle);
                }

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!("Spawned {} concurrent buy tasks", handles.len()).dimmed().to_string()
                );

                // Collect results from all concurrent buy operations with overall timeout
                let collection_result = tokio::time::timeout(Duration::from_secs(120), async {
                    let mut completed = 0;
                    let mut successful = 0;
                    let total_handles = handles.len();

                    for handle in handles {
                        // Skip if shutdown signal received
                        if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                            log(
                                LogTag::Trader,
                                "INFO",
                                "new entries monitor shutting down during buy result collection..."
                            );
                            break;
                        }

                        // Add timeout for each handle to prevent getting stuck
                        match tokio::time::timeout(Duration::from_secs(120), handle).await {
                            Ok(task_result) => {
                                match task_result {
                                    Ok(success) => {
                                        if success {
                                            successful += 1;
                                        }
                                    }
                                    Err(e) => {
                                        log(
                                            LogTag::Trader,
                                            "ERROR",
                                            &format!("Buy task failed: {}", e)
                                        );
                                    }
                                }
                            }
                            Err(_) => {
                                log(LogTag::Trader, "WARN", "Buy task timed out after 5 seconds");
                            }
                        }

                        completed += 1;
                        if completed % 2 == 0 || completed == total_handles {
                            log(
                                LogTag::Trader,
                                "INFO",
                                &format!("Completed {}/{} buy operations", completed, total_handles)
                                    .dimmed()
                                    .to_string()
                            );
                        }
                    }

                    (completed, successful)
                }).await;

                match collection_result {
                    Ok((completed, successful)) => {
                        let new_open_count = get_open_positions_count();
                        log(
                            LogTag::Trader,
                            "INFO",
                            &format!(
                                "Concurrent buy operations completed: {}/{} successful, new open positions: {}",
                                successful,
                                completed,
                                new_open_count
                            )
                        );
                    }
                    Err(_) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            "Buy operations collection timed out after 30 seconds"
                        );
                    }
                }
            }
        }

        // Calculate how long we've spent in this cycle
        let cycle_duration = cycle_start.elapsed();
        let wait_time = if cycle_duration >= Duration::from_secs(NEW_ENTRIES_CHECK_INTERVAL_SECS) {
            // If we've already spent more time than the interval, just wait a short time
            log(
                LogTag::Trader,
                "WARN",
                &format!("Token checking cycle took longer than interval: {:?}", cycle_duration)
            );
            Duration::from_millis(100)
        } else {
            // Otherwise wait for the remaining interval time
            Duration::from_secs(NEW_ENTRIES_CHECK_INTERVAL_SECS) - cycle_duration
        };

        if check_shutdown_or_delay(&shutdown, wait_time).await {
            log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
            break;
        }
    }
}

/// Background task to monitor open positions for exit opportunities
pub async fn monitor_open_positions(shutdown: Arc<Notify>) {
    loop {
        let mut positions_to_close = Vec::new();

        // Find open positions and check if they should be closed
        {
            if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                for (index, position) in positions.iter_mut().enumerate() {
                    if position.position_type == "buy" && position.exit_price.is_none() {
                        // Find current price for this token
                        if let Ok(tokens_guard) = LIST_TOKENS.read() {
                            if
                                let Some(token) = tokens_guard
                                    .iter()
                                    .find(|t| t.mint == position.mint)
                            {
                                if let Some(current_price) = token.price_dexscreener_sol {
                                    if current_price > 0.0 {
                                        // Update position tracking (extremes)
                                        update_position_tracking(position, current_price);

                                        // Calculate P&L using unified function
                                        let (pnl_sol, pnl_percent) = calculate_position_pnl(
                                            position,
                                            Some(current_price)
                                        );

                                        let now = Utc::now();

                                        // Calculate sell urgency using the advanced mathematical model
                                        let sell_urgency = should_sell(
                                            position,
                                            current_price,
                                            now
                                        );

                                        // Emergency exit conditions (keep original logic for safety)
                                        let emergency_exit = pnl_percent <= STOP_LOSS_PERCENT;

                                        // Urgency-based exit (sell if urgency > 70% or emergency)
                                        let should_exit = emergency_exit || sell_urgency > 0.7;

                                        if should_exit {
                                            log(
                                                LogTag::Trader,
                                                "SELL",
                                                &format!(
                                                    "Sell signal for {} ({}) - Urgency: {:.2}, P&L: {:.2}%, Emergency: {}",
                                                    position.symbol,
                                                    position.mint,
                                                    sell_urgency,
                                                    pnl_percent,
                                                    emergency_exit
                                                )
                                            );

                                            positions_to_close.push((
                                                index,
                                                position.clone(), // Include the full position data
                                                token.clone(),
                                                current_price,
                                                now,
                                            ));
                                        } else {
                                            log(
                                                LogTag::Trader,
                                                "HOLD",
                                                &format!(
                                                    "Holding {} ({}) - Urgency: {:.2}, P&L: {:.2}%",
                                                    position.symbol,
                                                    position.mint,
                                                    sell_urgency,
                                                    pnl_percent
                                                )
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Save updated positions with tracking data
                save_positions_to_file(&positions);
            }
        }

        // Close positions that need to be closed concurrently (outside of lock to avoid deadlock)
        if !positions_to_close.is_empty() {
            log(
                LogTag::Trader,
                "INFO",
                &format!("Processing {} positions for concurrent closing", positions_to_close.len())
            );

            // Use a semaphore to limit concurrent sell transactions to avoid overwhelming the network
            use tokio::sync::Semaphore;
            let semaphore = Arc::new(Semaphore::new(3)); // Allow up to 3 concurrent sells

            let mut handles = Vec::new();

            // Process all sell orders concurrently
            for (index, position, token, exit_price, exit_time) in positions_to_close {
                // Check for shutdown before spawning tasks
                if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                    log(
                        LogTag::Trader,
                        "INFO",
                        "open positions monitor shutting down during sell processing..."
                    );
                    break;
                }

                // Get permit from semaphore to limit concurrency with timeout
                let permit = match
                    tokio::time::timeout(
                        Duration::from_secs(5),
                        semaphore.clone().acquire_owned()
                    ).await
                {
                    Ok(Ok(permit)) => permit,
                    Ok(Err(e)) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Failed to acquire semaphore permit for sell: {}", e)
                        );
                        continue;
                    }
                    Err(_) => {
                        log(
                            LogTag::Trader,
                            "WARN",
                            "Semaphore acquire timed out for sell operation"
                        );
                        continue;
                    }
                };

                // We already have the position from the analysis phase, no need to look it up
                let handle = tokio::spawn(async move {
                    let _permit = permit; // Keep permit alive for duration of task

                    let mut position = position;
                    let token_symbol = token.symbol.clone();

                    // Wrap the sell operation in a timeout
                    match
                        tokio::time::timeout(Duration::from_secs(120), async {
                            close_position(&mut position, &token, exit_price, exit_time).await
                        }).await
                    {
                        Ok(success) => {
                            if success {
                                log(
                                    LogTag::Trader,
                                    "SUCCESS",
                                    &format!("Successfully closed position for {} in concurrent task", token_symbol)
                                );
                                Some((index, position))
                            } else {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Failed to close position for {} in concurrent task", token_symbol)
                                );
                                None
                            }
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Sell operation for {} timed out after 15 seconds", token_symbol)
                            );
                            None
                        }
                    }
                });

                handles.push(handle);
            }

            log(
                LogTag::Trader,
                "INFO",
                &format!("Spawned {} concurrent sell tasks", handles.len()).dimmed().to_string()
            );

            // Collect results from all concurrent sell operations with overall timeout
            // Increased timeout to 60 seconds to accommodate multiple 15-second sell operations
            let collection_result = tokio::time::timeout(Duration::from_secs(120), async {
                let mut completed_positions = Vec::new();
                let mut completed = 0;
                let total_handles = handles.len();

                for handle in handles {
                    // Skip if shutdown signal received
                    if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                        log(
                            LogTag::Trader,
                            "INFO",
                            "open positions monitor shutting down during sell result collection..."
                        );
                        break;
                    }

                    // Add timeout for each handle to prevent getting stuck
                    // Increased timeout to 15 seconds to allow for transaction verification and ATA closing
                    match tokio::time::timeout(Duration::from_secs(120), handle).await {
                        Ok(task_result) => {
                            match task_result {
                                Ok(Some((index, updated_position))) => {
                                    completed_positions.push((index, updated_position));
                                }
                                Ok(None) => {
                                    // Position failed to close, continue
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Trader,
                                        "ERROR",
                                        &format!("Sell task failed: {}", e)
                                    );
                                }
                            }
                        }
                        Err(_) => {
                            log(LogTag::Trader, "WARN", "Sell task timed out after 60 seconds");
                        }
                    }

                    completed += 1;
                    if completed % 2 == 0 || completed == total_handles {
                        log(
                            LogTag::Trader,
                            "INFO",
                            &format!("Completed {}/{} sell operations", completed, total_handles)
                                .dimmed()
                                .to_string()
                        );
                    }
                }

                completed_positions
            }).await;

            let completed_positions = match collection_result {
                Ok(positions) => positions,
                Err(_) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        "Sell operations collection timed out after 60 seconds"
                    );
                    Vec::new()
                }
            };

            // Update all successfully closed positions in the saved positions
            if !completed_positions.is_empty() {
                if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                    for (index, updated_position) in &completed_positions {
                        if let Some(saved_position) = positions.get_mut(*index) {
                            *saved_position = updated_position.clone();
                        }
                    }
                    save_positions_to_file(&positions);
                }

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!(
                        "Updated {} positions after concurrent sell operations",
                        completed_positions.len()
                    )
                );
            }
        }

        if
            check_shutdown_or_delay(
                &shutdown,
                Duration::from_secs(OPEN_POSITIONS_CHECK_INTERVAL_SECS)
            ).await
        {
            log(LogTag::Trader, "INFO", "open positions monitor shutting down...");
            break;
        }
    }
}

/// Main trader function that spawns both monitoring tasks
pub async fn trader(shutdown: Arc<Notify>) {
    log(LogTag::Trader, "INFO", "Starting trader with background tasks...");

    let shutdown_clone = shutdown.clone();
    let entries_task = tokio::spawn(async move {
        monitor_new_entries(shutdown_clone).await;
    });

    let shutdown_clone = shutdown.clone();
    let positions_task = tokio::spawn(async move {
        monitor_open_positions(shutdown_clone).await;
    });

    let shutdown_clone = shutdown.clone();
    let display_task = tokio::spawn(async move {
        monitor_positions_display(shutdown_clone).await;
    });

    // Wait for shutdown signal
    shutdown.notified().await;

    log(LogTag::Trader, "INFO", "Trader shutting down...");

    // Give tasks a chance to shutdown gracefully
    let graceful_timeout = tokio::time::timeout(Duration::from_secs(5), async {
        let _ = tokio::try_join!(entries_task, positions_task, display_task);
    });

    match graceful_timeout.await {
        Ok(_) => {
            log(LogTag::Trader, "INFO", "Trader tasks finished gracefully");
        }
        Err(_) => {
            log(LogTag::Trader, "WARN", "Trader tasks did not finish gracefully, aborting");
            // Force abort if graceful shutdown fails
            // entries_task.abort(); // These might already be finished
            // positions_task.abort();
            // display_task.abort();
        }
    }
}
