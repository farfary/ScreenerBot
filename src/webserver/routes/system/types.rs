// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! System route types — response structs for system endpoints.

use crate::services::startup::StartupServiceStatus;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RebootResponse {
    pub success: bool,
    /// Identity of the process accepting the restart request. Clients wait for
    /// `/api/health` to report a different value before reloading.
    pub instance_id: String,
}

#[derive(Debug, Serialize)]
pub struct BootStatusResponse {
    pub timestamp: String,
    pub initialization_required: bool,
    pub initialization_complete: bool,
    pub explore_mode: bool,
    pub onboarding_complete: bool,
    pub core_services_ready: bool,
    pub ui_ready: bool,
    pub ready_for_requests: bool,
    pub pending_services: Vec<&'static str>,
    pub services_total: usize,
    pub services_running: usize,
    pub connectivity_ready: bool,
    pub tokens_ready: bool,
    pub positions_ready: bool,
    pub pools_ready: bool,
    pub transactions_ready: bool,
    pub boot_progress: Vec<StartupServiceStatus>,
    pub wallet_snapshot_ready: bool,
    pub wallet_last_updated: Option<String>,
    pub uptime_seconds: u64,
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct PathsResponse {
    pub base_directory: String,
    pub data_directory: String,
    pub logs_directory: String,
    pub cache_pool_directory: String,
    pub analysis_exports_directory: String,
    pub config_path: String,
}

#[derive(Debug, Serialize)]
pub struct OpenPathResponse {
    pub opened: bool,
    pub path: String,
}

/// Stable id of each database in the storage overview. The dashboard labels it
/// through `DATABASE_LABELS` (ui/settings/data_tab.js).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseId {
    Tokens,
    Transactions,
    Positions,
    Events,
    Ohlcv,
    Wallet,
    Pools,
    Strategies,
    Actions,
}

impl DatabaseId {
    pub const ALL: [DatabaseId; 9] = [
        DatabaseId::Tokens,
        DatabaseId::Transactions,
        DatabaseId::Positions,
        DatabaseId::Events,
        DatabaseId::Ohlcv,
        DatabaseId::Wallet,
        DatabaseId::Pools,
        DatabaseId::Strategies,
        DatabaseId::Actions,
    ];
}

#[derive(Debug, Serialize)]
pub struct DatabaseStats {
    pub id: DatabaseId,
    pub path: String,
    pub size_bytes: u64,
    pub size_mb: f64,
    pub exists: bool,
}

#[derive(Debug, Serialize)]
pub struct DataStatsResponse {
    pub databases: Vec<DatabaseStats>,
    pub total_size_mb: f64,
    pub config_path: String,
    pub config_size_bytes: u64,
    pub data_directory: String,
    pub timestamp: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpenUrlRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct OpenUrlResponse {
    pub opened: bool,
    pub url: String,
}

/// Payload the dashboard frontend sends once it has fully loaded and started.
/// All fields optional so the signal works even from a minimal caller.
#[derive(Debug, Default, serde::Deserialize)]
pub struct ClientReadyRequest {
    /// The page that was active when the UI became ready (default "home").
    #[serde(default)]
    pub page: Option<String>,
    /// Milliseconds from page navigation start to fully-ready, if measured.
    #[serde(default)]
    pub load_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ClientReadyResponse {
    pub acknowledged: bool,
    /// True only for the first report after boot (the one that was logged).
    pub first_report: bool,
    pub uptime_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::DatabaseId;

    /// Catalog key of each database label. The match is exhaustive, so a new
    /// variant fails to compile until it is mapped here and in
    /// `DATABASE_LABELS` (ui/settings/data_tab.js).
    fn database_key(id: DatabaseId) -> &'static str {
        match id {
            DatabaseId::Tokens => "settings-data-db-tokens",
            DatabaseId::Transactions => "settings-data-db-transactions",
            DatabaseId::Positions => "settings-data-db-positions",
            DatabaseId::Events => "settings-data-db-events",
            DatabaseId::Ohlcv => "settings-data-db-ohlcv",
            DatabaseId::Wallet => "settings-data-db-wallet",
            DatabaseId::Pools => "settings-data-db-pools",
            DatabaseId::Strategies => "settings-data-db-strategies",
            DatabaseId::Actions => "settings-data-db-actions",
        }
    }

    #[test]
    fn database_labels_exist_in_the_catalog() {
        for id in DatabaseId::ALL {
            let key = database_key(id);
            assert_ne!(crate::i18n::format_en(key, None), key, "missing {key}");
            assert_eq!(
                serde_json::to_value(id).unwrap(),
                key.trim_start_matches("settings-data-db-"),
                "{id:?} serializes to the id its label is keyed by"
            );
        }
    }
}
