// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The registered service graph resolves without a cycle, and the pool
//! pricing pipeline starts after the services it reads and before the trader.

use super::ServiceManager;

#[tokio::test]
async fn pool_pricing_starts_after_its_inputs_and_before_the_trader() {
    let mut manager = ServiceManager::new().await.expect("service manager");
    crate::run::services::register_all_services(&mut manager);

    let names = manager.get_all_service_names();
    let order = manager
        .resolve_startup_order(&names)
        .expect("the registered service graph must be acyclic");
    let position = |name: &str| {
        order
            .iter()
            .position(|n| *n == name)
            .unwrap_or_else(|| panic!("{name} missing from the startup order {order:?}"))
    };

    let pricing = position("pool_pricing");
    let dependencies = manager
        .get_service("pool_pricing")
        .expect("pool_pricing registered")
        .dependencies();
    for dependency in ["pools", "tokens", "filtering", "transactions"] {
        assert!(
            position(dependency) < pricing,
            "{dependency} must start before pool_pricing: {order:?}"
        );
    }
    for dependency in dependencies {
        assert!(
            position(dependency) < pricing,
            "{dependency} must start before pool_pricing: {order:?}"
        );
    }

    let trader = manager.get_service("trader").expect("trader registered");
    assert!(trader.dependencies().contains(&"pool_pricing"));
    assert!(
        pricing < position("trader"),
        "pool_pricing must start before the trader: {order:?}"
    );
}
