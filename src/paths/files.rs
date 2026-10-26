// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Configuration, state and identity file path resolution.

use super::get_data_directory;
use std::path::PathBuf;

/// Returns the main configuration file path.
pub fn get_config_path() -> PathBuf {
    get_data_directory().join("config.toml")
}

/// Returns the UI state file path (for persisting table state, etc.).
pub fn get_ui_state_path() -> PathBuf {
    get_data_directory().join("ui_state.json")
}

/// Returns the process lock file path.
pub fn get_process_lock_path() -> PathBuf {
    get_data_directory().join(".screenerbot.lock")
}

/// Returns the persisted update state file path: the release found, the download
/// progress and the installer awaiting its apply.
pub fn get_update_state_path() -> PathBuf {
    get_data_directory().join("update-state.json")
}

/// Returns the pending trading-wallet replacement marker path. The marker exists
/// between the request that stages a replacement and the boot that applies it.
pub fn get_wallet_replacement_marker_path() -> PathBuf {
    get_data_directory().join("wallet_replacement.json")
}

/// Returns the agent runtime file path, which advertises the local webserver origin
/// to co-located tooling.
pub fn get_agent_runtime_path() -> PathBuf {
    get_data_directory().join("agent-runtime.json")
}

/// Returns the encrypted screenerbot.io account session file path.
pub fn get_account_store_path() -> PathBuf {
    get_data_directory().join("account.dat")
}

/// Returns the device identifier file path, used where the platform offers no
/// machine identifier.
pub fn get_device_id_path() -> PathBuf {
    get_data_directory().join(".device_id")
}
