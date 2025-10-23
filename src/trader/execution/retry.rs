//! Retry mechanism for failed trades

use crate::logger::{log, LogTag};
use crate::trader::execution::{execute_buy, execute_sell};
use crate::trader::types::{TradeAction, TradeResult};
use tokio::time::{sleep, Duration};

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 2000;

/// Retry a failed trade
pub async fn retry_trade(result: &TradeResult) -> Result<TradeResult, String> {
    if result.success {
        return Ok(result.clone());
    }

    let mut retry_count = result.retry_count;

    if retry_count >= MAX_RETRIES {
        log(
            LogTag::Trader,
            "ERROR",
            &format!(
                "Max retries ({}) reached for trade: mint={}, action={:?}",
                MAX_RETRIES, result.decision.mint, result.decision.action
            ),
        );
        return Ok(result.clone());
    }

    retry_count += 1;

    log(
        LogTag::Trader,
        "INFO",
        &format!(
            "Retrying trade (attempt {}/{}): mint={}, action={:?}",
            retry_count, MAX_RETRIES, result.decision.mint, result.decision.action
        ),
    );

    // Wait before retrying
    sleep(Duration::from_millis(RETRY_DELAY_MS)).await;

    // Retry the trade based on action type
    let mut retry_result = match result.decision.action {
        TradeAction::Buy => execute_buy(&result.decision).await?,
        TradeAction::Sell => execute_sell(&result.decision).await?,
        TradeAction::DCA => execute_buy(&result.decision).await?, // DCA uses buy execution
    };

    // Update retry count
    retry_result.retry_count = retry_count;

    Ok(retry_result)
}
