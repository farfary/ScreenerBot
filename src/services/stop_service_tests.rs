// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Stopping one service while the others run: the service's own tasks end, the shared
//! shutdown signal is never sent, a stopped service can start again, and a service that
//! cannot stop alone or that a running service depends on is refused.

use super::{Service, ServiceManager};
use crate::errors::ServiceError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// A service with one task that runs until its own stop signal or the shared shutdown.
struct TaskService {
    name: &'static str,
    dependencies: Vec<&'static str>,
    stops_alone: bool,
    own_stop: Arc<Notify>,
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
        let own_stop = self.own_stop.clone();
        let ended_by_shared_shutdown = self.ended_by_shared_shutdown.clone();
        Ok(vec![tokio::spawn(async move {
            tokio::select! {
                _ = own_stop.notified() => {}
                _ = shutdown.notified() => {
                    ended_by_shared_shutdown.fetch_add(1, Ordering::SeqCst);
                }
            }
        })])
    }

    async fn stop(&mut self) -> crate::Result<()> {
        self.own_stop.notify_waiters();
        Ok(())
    }
}

fn task_service(
    name: &'static str,
    dependencies: &[&'static str],
    stops_alone: bool,
    shared: &Arc<AtomicUsize>,
) -> Box<dyn Service> {
    Box::new(TaskService {
        name,
        dependencies: dependencies.to_vec(),
        stops_alone,
        own_stop: Arc::new(Notify::new()),
        ended_by_shared_shutdown: shared.clone(),
    })
}

#[tokio::test]
async fn one_service_stops_and_starts_again_while_the_others_run() {
    let shared = Arc::new(AtomicUsize::new(0));
    let mut manager = ServiceManager::new().await.expect("service manager");
    manager.register(task_service("telegram", &[], true, &shared));
    manager.register(task_service("pools", &[], false, &shared));
    manager.start_all().await.expect("start");

    assert!(manager.stop_service("telegram").await.expect("stop"));
    assert!(!manager.is_service_running("telegram"));
    assert!(manager.is_service_running("pools"));
    assert_eq!(
        shared.load(Ordering::SeqCst),
        0,
        "stopping one service must not send the shared shutdown signal"
    );

    assert!(!manager.stop_service("telegram").await.expect("second stop"));

    let report = manager
        .start_enabled_service("telegram")
        .await
        .expect("restart");
    assert_eq!(report.started, vec!["telegram"]);
    assert!(manager.is_service_running("telegram"));
}

#[tokio::test]
async fn a_service_that_cannot_stop_alone_or_has_running_dependents_is_refused() {
    let shared = Arc::new(AtomicUsize::new(0));
    let mut manager = ServiceManager::new().await.expect("service manager");
    manager.register(task_service("pools", &[], false, &shared));
    manager.register(task_service("notifier", &[], true, &shared));
    manager.register(task_service("reader", &["notifier"], false, &shared));
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
