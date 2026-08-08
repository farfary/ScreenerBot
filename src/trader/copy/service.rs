//! Runtime paper consumer of the shared wallet-observation broadcast.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::Notify;

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::wallets::watch::{subscribe_activity, ActivityKind, WalletActivity, WatchSource};

use super::{
    run_paper_pipeline, CopyDatabase, CopyOutcome, CopySkip, PaperCosts, PipelinePolicy,
    RiskContext, SpendState,
};

pub async fn run(shutdown: Arc<Notify>, database: CopyDatabase) {
    let mut receiver = subscribe_activity();
    loop {
        tokio::select! {
            _ = shutdown.notified() => return,
            received = receiver.recv() => match received {
                Ok(activity) => {
                    if let Err(error) = process_activity(&database, &activity).await {
                        logger::warning(LogTag::Trader, &format!("Paper copy activity failed: {error}"));
                    }
                }
                Err(RecvError::Lagged(count)) => logger::warning(
                    LogTag::Trader,
                    &format!("Paper copy consumer lagged by {count} wallet activities"),
                ),
                Err(RecvError::Closed) => return,
            }
        }
    }
}

async fn process_activity(
    database: &CopyDatabase,
    activity: &WalletActivity,
) -> Result<(), String> {
    let (copy_enabled, require_filter_pass) = with_config(|config| {
        (
            config.copy_trading.enabled,
            config.copy_trading.require_filter_pass,
        )
    });
    if !copy_enabled {
        return Ok(());
    }
    if !activity
        .sources
        .iter()
        .any(|source| matches!(source, WatchSource::Copy { .. }))
    {
        return Ok(());
    }
    let ActivityKind::Swap {
        mint, price_sol, ..
    } = &activity.kind
    else {
        return Ok(());
    };
    let tasks = database
        .enabled_tasks_for_subject(&activity.subject)
        .await?;
    if tasks.is_empty() {
        return Ok(());
    }
    if let Err(block) = crate::trader::admission::check_entry_admission(mint, &["rpc"]).await {
        for task in tasks {
            database
                .record_outcome(CopyOutcome::Skipped {
                    task_id: task.id,
                    signature: activity.signature.clone(),
                    mint: Some(mint.clone()),
                    reason: CopySkip::EntryBlocked {
                        block: block.clone(),
                    },
                    decided_at: Utc::now(),
                })
                .await?;
        }
        return Ok(());
    }
    let filter_passed = if require_filter_pass {
        crate::filtering::get_filtered_token_mints()
            .await?
            .iter()
            .any(|passed| passed == mint)
    } else {
        true
    };
    let mut spend = HashMap::new();
    let mut risk = HashMap::new();
    let own_addresses = crate::wallets::list_wallets(true)
        .await?
        .into_iter()
        .map(|wallet| wallet.address)
        .collect::<Vec<_>>();
    for task in &tasks {
        spend.insert(task.id, database.spend_state(task.id, mint).await?);
        risk.insert(
            task.id,
            RiskContext {
                is_self_wallet: own_addresses
                    .iter()
                    .any(|address| address == &task.target_address),
                mint_blacklisted: crate::trader::safety::is_blacklisted(mint).await,
                filter_passed,
                ..RiskContext::default()
            },
        );
    }
    let decision_price = crate::pools::get_pool_price(mint)
        .map(|price| price.price_sol)
        .or(*price_sol)
        .unwrap_or(f64::NAN);
    let (trade_size_sol, priority_lamports) = with_config(|config| {
        (
            config.trader.trade_size_sol,
            config.swaps.jupiter.default_priority_fee,
        )
    });
    let outcomes = run_paper_pipeline(
        activity,
        &tasks,
        &spend,
        &risk,
        PipelinePolicy {
            require_filter_pass,
            engine_trade_size_sol: trade_size_sol,
        },
        decision_price,
        PaperCosts {
            network_fee_sol: 0.000005,
            priority_fee_sol: crate::utils::lamports_to_sol(priority_lamports),
        },
        Utc::now(),
    );
    for outcome in outcomes {
        database.record_outcome(outcome).await?;
    }
    Ok(())
}
