//! The wallet profile: what this bot knows about a wallet before (or while)
//! copying it.

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::super::control::open_database;
use super::super::{InsightRange, TargetObservations};
use super::{compare, TaskComparison};
use crate::trader::{Error, Result};

#[derive(Debug, Serialize)]
pub struct WalletWatch {
    pub label: Option<String>,
    pub enabled: bool,
    pub sources: usize,
    pub subscribed: bool,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

/// What this bot knows about a wallet before (or while) copying it.
#[derive(Debug, Serialize)]
pub struct WalletProfile {
    pub address: String,
    pub own_wallet: bool,
    pub watch: Option<WalletWatch>,
    pub observations: TargetObservations,
    pub tasks: Vec<TaskComparison>,
}

pub async fn wallet_profile(address: &str) -> Result<WalletProfile> {
    crate::chains::adapter()
        .validate_address(address)
        .map_err(|e| Error::CopyValidation {
            detail: format!("invalid wallet address: {e}"),
        })?;
    let db = open_database().await?;
    let tasks = db
        .list_tasks()
        .await?
        .into_iter()
        .filter(|task| task.target_address == address)
        .collect::<Vec<_>>();
    let observations = db
        .target_observations(tasks.iter().map(|task| task.id).collect())
        .await?;
    let watch = match crate::wallets::watch::get_target_by_address(address).await {
        Ok(Some(target)) => {
            let status = match target.id {
                Some(id) => crate::wallets::watch::get_status(id).await.ok(),
                None => None,
            };
            Some(WalletWatch {
                label: target.label.clone(),
                enabled: target.enabled,
                sources: target.sources.len(),
                subscribed: status.as_ref().is_some_and(|status| status.subscribed),
                last_activity_at: status.as_ref().and_then(|status| status.last_activity_at),
                last_error: status.and_then(|status| status.last_error),
            })
        }
        _ => None,
    };
    let own_wallet = crate::wallets::list_wallets(true)
        .await
        .map(|wallets| wallets.into_iter().any(|wallet| wallet.address == address))
        .unwrap_or(false);
    Ok(WalletProfile {
        address: address.to_owned(),
        own_wallet,
        watch,
        observations,
        tasks: compare(&tasks, InsightRange::default()).await?,
    })
}
