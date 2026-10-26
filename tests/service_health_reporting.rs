// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! How the service manager reports a service its config turns off: `Disabled` and not
//! enabled, the same state every config-gated service reports, never a healthy service
//! that never runs.

use crate::common;

use screenerbot::services::implementations::TelegramService;
use screenerbot::services::{ServiceHealth, ServiceManager};

async fn telegram_report() -> (bool, ServiceHealth) {
    let mut manager = ServiceManager::new()
        .await
        .expect("ServiceManager::new must not fail offline");
    manager.register(Box::new(TelegramService::new()));
    let health = manager
        .get_health()
        .await
        .remove("telegram")
        .expect("the registered service reports a health");
    (manager.is_service_enabled("telegram"), health)
}

#[tokio::test]
async fn telegram_disabled_in_config_reports_disabled() {
    let _cfg = common::config_guard();
    common::set_config(|cfg| cfg.telegram.enabled = false);

    assert_eq!(telegram_report().await, (false, ServiceHealth::Disabled));
}

#[tokio::test]
async fn telegram_enabled_in_config_reports_its_own_health() {
    let _cfg = common::config_guard();
    common::set_config(|cfg| {
        cfg.telegram.enabled = true;
        cfg.telegram.bot_token = String::new();
    });

    let (enabled, health) = telegram_report().await;
    assert!(enabled, "an enabled Telegram config enables the service");
    assert_ne!(health, ServiceHealth::Disabled);
}
