// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet watch service — the observation layer `TransactionsService` and the
//! Telegram alert consumer both depend on: WebSocket + poll fallback + gap-fill
//! detection for the own wallet and every watched target, feeding one shared
//! `WalletActivity` broadcast (see `wallets::watch`).

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::i18n::{ids, UiText};
use crate::services::{Service, ServiceHealth, ServiceMetrics};

pub struct WalletWatchService;

#[async_trait]
impl Service for WalletWatchService {
    fn name(&self) -> &'static str {
        "wallet_watch"
    }

    fn priority(&self) -> i32 {
        // Immediately after `transactions` (80): decode/persistence requires the
        // global TransactionDatabase initialized by that service. Observation then
        // waits for the wallet history load, and the transactions consumer
        // subscribes on the same task that marks the history loaded, with no await
        // in between, so its subscription precedes the first broadcast.
        81
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["connectivity", "transactions"]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        _monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let handle = crate::wallets::watch::start(shutdown).await?;

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        // Observation begins once the own wallet's history has loaded.
        if !crate::global::TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::Relaxed) {
            ServiceHealth::Starting
        } else if crate::wallets::watch::is_healthy() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Degraded(UiText::new(ids::SERVICES_HEALTH_WALLET_WATCH_POLLING_ONLY))
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }
}
