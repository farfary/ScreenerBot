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

use super::catalog::StoreSpec;
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
}

/// One store's entry in the registry.
#[derive(Debug, Clone, Serialize)]
pub struct StoreStatus {
    pub store: &'static str,
    pub path: PathBuf,
    pub health: StoreHealth,
    pub changed_at: DateTime<Utc>,
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
    registry.insert(
        path.to_path_buf(),
        StoreStatus {
            store: spec.id,
            path: path.to_path_buf(),
            health,
            changed_at: Utc::now(),
        },
    );
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
    use crate::database::catalog::RPC_STATS;

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
}
