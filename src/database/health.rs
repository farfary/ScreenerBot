// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The health of every store the storage phase opened: healthy, rebuilt, restored or unavailable with the failure behind it, logged on every change and read by the status route and the services that own the stores.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, PoisonError, RwLock};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};

use super::catalog::{StoreClass, StoreSpec};
use super::recovery::{FailureKind, RecoveryAction};

/// The state of one store file.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum StoreHealth {
    Healthy,
    /// The store is being moved aside and created afresh.
    Rebuilding {
        since: DateTime<Utc>,
        phase: String,
    },
    /// The damaged store was replaced by a copy of its snapshot taken at `snapshot_at`.
    Restored {
        snapshot_at: Option<DateTime<Utc>>,
        quarantine: PathBuf,
    },
    /// The store was moved to `quarantine` and created afresh.
    Recovered {
        quarantine: PathBuf,
    },
    /// The store could not be opened and was left as it is.
    Unavailable {
        failure: FailureKind,
        isolated: bool,
        detail: String,
    },
}

impl StoreHealth {
    /// The state of a store left unopened after `error`. `action` is what recovery
    /// decided: an isolated store is kept in place by policy, any other was blocked by
    /// its surroundings.
    pub fn unavailable(error: &DatabaseError, action: RecoveryAction) -> Self {
        StoreHealth::Unavailable {
            failure: FailureKind::of(error),
            isolated: action == RecoveryAction::Isolate,
            detail: error.to_string(),
        }
    }

    /// Whether the store is open.
    pub fn is_open(&self) -> bool {
        !matches!(
            self,
            StoreHealth::Unavailable { .. } | StoreHealth::Rebuilding { .. }
        )
    }

    /// Whether the store's contents are older than the moment it failed: a snapshot copy
    /// or an empty file standing in for the damaged one.
    pub fn is_replacement(&self) -> bool {
        matches!(
            self,
            StoreHealth::Restored { .. } | StoreHealth::Recovered { .. }
        )
    }

    /// The serialized name of the state.
    pub fn as_str(&self) -> &'static str {
        match self {
            StoreHealth::Healthy => "healthy",
            StoreHealth::Rebuilding { .. } => "rebuilding",
            StoreHealth::Restored { .. } => "restored",
            StoreHealth::Recovered { .. } => "recovered",
            StoreHealth::Unavailable { .. } => "unavailable",
        }
    }
}

/// One store's entry in the registry.
#[derive(Debug, Clone, Serialize)]
pub struct StoreStatus {
    pub store: &'static str,
    pub path: PathBuf,
    pub health: StoreHealth,
    pub changed_at: DateTime<Utc>,
    /// False while a Ledger store holds replacement contents that the wallet history has
    /// not yet been reconciled against; money decisions that read the store wait for it.
    pub reconciled: bool,
}

impl StoreStatus {
    /// Whether the store is open and its contents may drive money decisions.
    pub fn is_settled(&self) -> bool {
        self.health.is_open() && self.reconciled
    }
}

static REGISTRY: LazyLock<RwLock<BTreeMap<PathBuf, StoreStatus>>> =
    LazyLock::new(|| RwLock::new(BTreeMap::new()));

/// Record `health` for the store `spec` opens at `path`, logging every change of state.
pub fn publish(spec: &StoreSpec, path: &Path, health: StoreHealth) {
    let mut registry = REGISTRY.write().unwrap_or_else(PoisonError::into_inner);
    if registry
        .get(path)
        .is_some_and(|current| current.health == health)
    {
        return;
    }
    log_transition(spec, path, &health);
    let reconciled = !(spec.class == StoreClass::Ledger && health.is_replacement());
    registry.insert(
        path.to_path_buf(),
        StoreStatus {
            store: spec.id,
            path: path.to_path_buf(),
            health,
            changed_at: Utc::now(),
            reconciled,
        },
    );
}

/// Record that the replacement contents of the store at `path` were reconciled against
/// the wallet history. The state is kept, so the restore stays visible. Returns whether
/// the store was waiting for it.
pub fn mark_reconciled(path: &Path) -> bool {
    let mut registry = REGISTRY.write().unwrap_or_else(PoisonError::into_inner);
    let Some(status) = registry.get_mut(path).filter(|status| !status.reconciled) else {
        return false;
    };
    status.reconciled = true;
    logger::info(
        LogTag::System,
        &format!(
            "{} ({}): {} contents reconciled with the wallet history",
            status.store,
            path.display(),
            status.health.as_str()
        ),
    );
    true
}

fn log_transition(spec: &StoreSpec, path: &Path, health: &StoreHealth) {
    let store = format!("{} ({})", spec.id, path.display());
    match health {
        StoreHealth::Healthy => logger::debug(LogTag::System, &format!("{store}: healthy")),
        StoreHealth::Rebuilding { phase, .. } => {
            logger::warning(LogTag::System, &format!("{store}: rebuilding, {phase}"))
        }
        StoreHealth::Restored {
            snapshot_at,
            quarantine,
        } => logger::warning(
            LogTag::System,
            &format!(
                "{store}: restored from the snapshot taken {}; damaged file kept in {}",
                snapshot_at.map_or_else(|| "at an unknown time".to_owned(), |at| at.to_rfc3339()),
                quarantine.display()
            ),
        ),
        StoreHealth::Recovered { quarantine } => logger::warning(
            LogTag::System,
            &format!(
                "{store}: created afresh; previous file kept in {}",
                quarantine.display()
            ),
        ),
        StoreHealth::Unavailable {
            failure,
            isolated,
            detail,
            ..
        } => logger::error(
            LogTag::System,
            &format!(
                "{store}: unavailable ({}, {}): {detail}",
                failure.as_str(),
                if *isolated {
                    "left in place"
                } else {
                    "nothing changed"
                }
            ),
        ),
    }
}

/// The recorded state of the store at `path`, if the phase reached it.
pub fn status(path: &Path) -> Option<StoreStatus> {
    REGISTRY
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .get(path)
        .cloned()
}

/// The recorded state of the first file of the store `spec` that is not open, `None`
/// while every file the phase reached is open.
pub fn first_unavailable(spec: &StoreSpec) -> Option<StoreStatus> {
    spec.paths()
        .iter()
        .filter_map(|path| status(path))
        .find(|status| !status.health.is_open())
}

/// Every store's recorded state, in path order.
pub fn all() -> Vec<StoreStatus> {
    REGISTRY
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .values()
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::catalog::{POSITIONS, RPC_STATS};

    #[test]
    fn a_change_of_state_is_recorded_and_a_repeat_is_not() {
        let path = PathBuf::from("/health-test/rpc_stats.db");
        publish(&RPC_STATS, &path, StoreHealth::Healthy);
        let first = status(&path).expect("recorded");
        publish(&RPC_STATS, &path, StoreHealth::Healthy);
        assert_eq!(
            status(&path).expect("recorded").changed_at,
            first.changed_at
        );

        let error = DatabaseError::DiskFull {
            operation: "open".to_owned(),
            message: "full".to_owned(),
        };
        publish(
            &RPC_STATS,
            &path,
            StoreHealth::unavailable(&error, RecoveryAction::Unavailable),
        );
        let recorded = status(&path).expect("recorded");
        assert!(!recorded.health.is_open());
        let json = serde_json::to_value(&recorded).expect("serialize");
        assert_eq!(json["health"]["state"], "unavailable");
        assert_eq!(json["health"]["failure"], "disk_full");
        assert_eq!(json["health"]["isolated"], false);
    }

    #[test]
    fn a_replaced_ledger_store_waits_for_reconciliation_and_keeps_its_state() {
        let restored = StoreHealth::Restored {
            snapshot_at: None,
            quarantine: PathBuf::from("/health-test/quarantine"),
        };
        let recovered = StoreHealth::Recovered {
            quarantine: PathBuf::from("/health-test/quarantine"),
        };
        for (index, health) in [restored, recovered].into_iter().enumerate() {
            let path = PathBuf::from(format!("/health-test/positions-{index}.db"));
            publish(&POSITIONS, &path, health.clone());
            let recorded = status(&path).expect("recorded");
            assert!(recorded.health.is_open());
            assert!(!recorded.reconciled);
            assert!(!recorded.is_settled());

            assert!(mark_reconciled(&path));
            let recorded = status(&path).expect("recorded");
            assert!(recorded.reconciled);
            assert!(recorded.is_settled());
            assert_eq!(recorded.health, health);
            assert!(!mark_reconciled(&path), "a second mark changes nothing");
        }
    }

    #[test]
    fn only_a_replaced_ledger_store_starts_unreconciled() {
        let path = PathBuf::from("/health-test/positions-healthy.db");
        publish(&POSITIONS, &path, StoreHealth::Healthy);
        assert!(status(&path).expect("recorded").reconciled);
        assert!(!mark_reconciled(&path));

        let cache = PathBuf::from("/health-test/rpc_stats-recovered.db");
        publish(
            &RPC_STATS,
            &cache,
            StoreHealth::Recovered {
                quarantine: PathBuf::from("/health-test/quarantine"),
            },
        );
        assert!(status(&cache).expect("recorded").is_settled());
        assert!(!mark_reconciled(&PathBuf::from(
            "/health-test/never-opened.db"
        )));
    }
}
