// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Startup order resolution: every declared dependency starts before its
//! dependent, priority and then name decide among ready services, the result
//! does not depend on registration order, and a cycle is a typed error.

use super::{Service, ServiceManager};
use crate::errors::ServiceError;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

struct StubService {
    name: &'static str,
    priority: i32,
    dependencies: Vec<&'static str>,
}

#[async_trait::async_trait]
impl Service for StubService {
    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn dependencies(&self) -> Vec<&'static str> {
        self.dependencies.clone()
    }

    async fn start(
        &mut self,
        _shutdown: Arc<Notify>,
        _monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        Ok(Vec::new())
    }
}

async fn stub_manager(stubs: &[(&'static str, i32, &[&'static str])]) -> ServiceManager {
    let mut manager = ServiceManager::new().await.expect("service manager");
    for &(name, priority, dependencies) in stubs {
        manager.register(Box::new(StubService {
            name,
            priority,
            dependencies: dependencies.to_vec(),
        }));
    }
    manager
}

async fn registered_startup_order() -> Vec<&'static str> {
    let mut manager = ServiceManager::new().await.expect("service manager");
    crate::run::services::register_all_services(&mut manager);
    let names = manager.get_all_service_names();
    manager
        .resolve_startup_order(&names)
        .expect("the registered service graph must be acyclic")
}

#[tokio::test]
async fn every_registered_dependency_starts_before_its_dependent() {
    let mut manager = ServiceManager::new().await.expect("service manager");
    crate::run::services::register_all_services(&mut manager);
    let names = manager.get_all_service_names();
    let order = manager
        .resolve_startup_order(&names)
        .expect("the registered service graph must be acyclic");

    let mut sorted_order = order.clone();
    sorted_order.sort_unstable();
    let mut sorted_names = names.clone();
    sorted_names.sort_unstable();
    assert_eq!(
        sorted_order, sorted_names,
        "the startup order must hold every registered service exactly once"
    );

    let position = |name: &str| order.iter().position(|n| *n == name);
    for &name in &names {
        let service = manager.get_service(name).expect("registered service");
        let own = position(name).expect("service in the startup order");
        for dependency in service.dependencies() {
            let dependency_position = position(dependency)
                .unwrap_or_else(|| panic!("{name} depends on unregistered {dependency}"));
            assert!(
                dependency_position < own,
                "{dependency} must start before {name}: {order:?}"
            );
        }
    }
}

#[tokio::test]
async fn registered_startup_order_is_deterministic() {
    let first = registered_startup_order().await;
    for _ in 0..8 {
        assert_eq!(registered_startup_order().await, first);
    }

    let mut manager = ServiceManager::new().await.expect("service manager");
    crate::run::services::register_all_services(&mut manager);
    let mut names = manager.get_all_service_names();
    names.sort_unstable();
    names.reverse();
    assert_eq!(
        manager
            .resolve_startup_order(&names)
            .expect("acyclic service graph"),
        first,
        "the order must not depend on the order services are requested in"
    );
}

#[tokio::test]
async fn ready_services_start_by_priority_then_name_after_their_dependencies() {
    let stubs: &[(&'static str, i32, &[&'static str])] = &[
        ("early_dependent", 10, &["late_base"]),
        ("late_base", 80, &[]),
        ("tie_b", 50, &[]),
        ("tie_a", 50, &[]),
        ("after_dependent", 20, &["early_dependent"]),
    ];
    let forward = stub_manager(stubs).await;
    let reversed: Vec<_> = stubs.iter().rev().copied().collect();
    let backward = stub_manager(&reversed).await;

    let expected = vec![
        "tie_a",
        "tie_b",
        "late_base",
        "early_dependent",
        "after_dependent",
    ];
    for manager in [&forward, &backward] {
        let names = manager.get_all_service_names();
        assert_eq!(
            manager
                .resolve_startup_order(&names)
                .expect("acyclic graph"),
            expected
        );
    }

    // Requesting one service pulls in its dependencies transitively.
    assert_eq!(
        forward
            .resolve_startup_order(&["after_dependent"])
            .expect("acyclic graph"),
        vec!["late_base", "early_dependent", "after_dependent"]
    );
}

#[tokio::test]
async fn a_dependency_cycle_is_a_typed_dependency_error() {
    let manager = stub_manager(&[
        ("alpha", 10, &["gamma"]),
        ("beta", 20, &["alpha"]),
        ("gamma", 30, &["beta"]),
        ("standalone", 5, &[]),
    ])
    .await;

    let names = manager.get_all_service_names();
    match manager.resolve_startup_order(&names) {
        Err(crate::Error::Service(ServiceError::Dependency {
            service,
            dependency,
            message,
        })) => {
            assert_eq!(service, "alpha");
            assert_eq!(dependency, "circular");
            assert!(
                message.contains("alpha, beta, gamma"),
                "the error must name the blocked services: {message}"
            );
        }
        other => panic!("expected a circular dependency error, got {other:?}"),
    }

    let self_dependent = stub_manager(&[("loop", 10, &["loop"])]).await;
    assert!(matches!(
        self_dependent.resolve_startup_order(&["loop"]),
        Err(crate::Error::Service(ServiceError::Dependency { .. }))
    ));
}

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
