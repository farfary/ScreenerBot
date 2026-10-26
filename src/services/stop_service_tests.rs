// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Stopping one service while the others run: the service's own tasks end on its own
//! signal, the shared shutdown signal is never sent, its metrics sampler ends with it, a
//! stopped service can start again, and a service that cannot stop alone or that a
//! running service depends on is refused.

use super::{startup, Service, ServiceManager};
use crate::errors::ServiceError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Notify};
use tokio::task::JoinHandle;

/// A service with one task that runs until its own stop signal or the shared shutdown.
/// The stop signal is level-triggered, as the real services' are, so a task that has not
/// reached its wait yet still sees it.
struct TaskService {
    name: &'static str,
    dependencies: Vec<&'static str>,
    stops_alone: bool,
    own_stop: watch::Sender<bool>,
    /// Counts tasks that ended because their own stop signal reached them.
    ended_by_own_stop: Arc<AtomicUsize>,
    /// Counts tasks that ended because the shared shutdown signal reached them.
    ended_by_shared_shutdown: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Service for TaskService {
    fn name(&self) -> &'static str {
        self.name
    }

    fn dependencies(&self) -> Vec<&'static str> {
        self.dependencies.clone()
    }

    fn stops_alone(&self) -> bool {
        self.stops_alone
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        _monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        self.own_stop = watch::Sender::new(false);
        let mut own_stop = self.own_stop.subscribe();
        let ended_by_own_stop = self.ended_by_own_stop.clone();
        let ended_by_shared_shutdown = self.ended_by_shared_shutdown.clone();
        Ok(vec![tokio::spawn(async move {
            tokio::select! {
                _ = own_stop.wait_for(|stopped| *stopped) => {
                    ended_by_own_stop.fetch_add(1, Ordering::SeqCst);
                }
                _ = shutdown.notified() => {
                    ended_by_shared_shutdown.fetch_add(1, Ordering::SeqCst);
                }
            }
        })])
    }

    async fn stop(&mut self) -> crate::Result<()> {
        self.own_stop.send_replace(true);
        Ok(())
    }
}

/// Counters the services of one test share.
#[derive(Default)]
struct Endings {
    own_stop: Arc<AtomicUsize>,
    shared_shutdown: Arc<AtomicUsize>,
}

fn task_service(
    name: &'static str,
    dependencies: &[&'static str],
    stops_alone: bool,
    endings: &Endings,
) -> Box<dyn Service> {
    Box::new(TaskService {
        name,
        dependencies: dependencies.to_vec(),
        stops_alone,
        own_stop: watch::Sender::new(false),
        ended_by_own_stop: endings.own_stop.clone(),
        ended_by_shared_shutdown: endings.shared_shutdown.clone(),
    })
}

/// Wait until `manager` has `expected` live metrics samplers; an aborted sampler is
/// dropped on the runtime's next turn.
async fn assert_live_samplers(manager: &ServiceManager, expected: usize) {
    let settled = tokio::time::timeout(Duration::from_secs(2), async {
        while manager.metrics_collector.live_samplers() != expected {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(
        settled.is_ok(),
        "expected {expected} live samplers, found {}",
        manager.metrics_collector.live_samplers()
    );
}

#[tokio::test]
async fn one_service_stops_and_starts_again_while_the_others_run() {
    let endings = Endings::default();
    let mut manager = ServiceManager::new().await.expect("service manager");
    manager.register(task_service("telegram", &[], true, &endings));
    manager.register(task_service("pools", &[], false, &endings));
    manager.start_all().await.expect("start");
    assert_live_samplers(&manager, 2).await;

    let stopping = manager
        .stop_service("telegram")
        .await
        .expect("stop")
        .expect("telegram was running");
    assert!(!manager.is_service_running("telegram"));
    assert!(manager.is_service_running("pools"));
    assert_eq!(
        stopping.finish().await,
        0,
        "the task must end on its own stop signal, not be aborted"
    );
    assert_eq!(endings.own_stop.load(Ordering::SeqCst), 1);
    assert_eq!(
        endings.shared_shutdown.load(Ordering::SeqCst),
        0,
        "stopping one service must not send the shared shutdown signal"
    );
    assert_live_samplers(&manager, 1).await;
    assert!(
        !startup::get_status("telegram").is_some_and(|status| status.ready),
        "a stopped service is not reported ready"
    );

    assert!(manager
        .stop_service("telegram")
        .await
        .expect("second stop")
        .is_none());

    let report = manager
        .start_enabled_service("telegram")
        .await
        .expect("restart");
    assert_eq!(report.started, vec!["telegram"]);
    assert!(manager.is_service_running("telegram"));
    assert!(startup::get_status("telegram").is_some_and(|status| status.ready));
    assert_live_samplers(&manager, 2).await;
}

#[tokio::test]
async fn repeated_restarts_keep_one_sampler_per_service() {
    let endings = Endings::default();
    let mut manager = ServiceManager::new().await.expect("service manager");
    manager.register(task_service("telegram", &[], true, &endings));
    manager.start_all().await.expect("start");

    for _ in 0..3 {
        let stopping = manager
            .stop_service("telegram")
            .await
            .expect("stop")
            .expect("telegram was running");
        assert_eq!(stopping.finish().await, 0);
        manager
            .start_enabled_service("telegram")
            .await
            .expect("restart");
    }
    assert_eq!(endings.own_stop.load(Ordering::SeqCst), 3);
    assert_live_samplers(&manager, 1).await;
}

#[tokio::test]
async fn a_service_that_cannot_stop_alone_or_has_running_dependents_is_refused() {
    let endings = Endings::default();
    let mut manager = ServiceManager::new().await.expect("service manager");
    manager.register(task_service("pools", &[], false, &endings));
    manager.register(task_service("notifier", &[], true, &endings));
    manager.register(task_service("reader", &["notifier"], false, &endings));
    manager.start_all().await.expect("start");

    match manager.stop_service("pools").await {
        Err(crate::Error::Service(ServiceError::Stop { service, .. })) => {
            assert_eq!(service, "pools")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    match manager.stop_service("notifier").await {
        Err(crate::Error::Service(ServiceError::Dependency {
            service,
            dependency,
            ..
        })) => {
            assert_eq!(service, "reader");
            assert_eq!(dependency, "notifier");
        }
        other => panic!("expected a dependency refusal, got {other:?}"),
    }
    assert!(manager.is_service_running("pools"));
    assert!(manager.is_service_running("notifier"));
}
