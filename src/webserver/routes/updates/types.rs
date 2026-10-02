// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Request and response types for the update routes - VersionResponse, UpdateStatusResponse and DownloadRequest.

use crate::i18n::UiText;
use crate::version::{ReleaseSummary, StagedCore, UpdateInfo, UpdateState};
use serde::{Deserialize, Serialize};

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub platform: String,
    pub build_number: String,
    /// Revision of the Electron shell hosting this core, when it reported one.
    pub shell_revision: Option<String>,
    /// True when this core was activated by a silent update rather than shipped
    /// with the installed application.
    pub core_staged: bool,
}

#[derive(Debug, Serialize)]
pub struct AcknowledgeResponse {
    pub acknowledged: bool,
}

#[derive(Debug, Serialize)]
pub struct UpdateCheckResponse {
    pub update_available: bool,
    pub current_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<UpdateInfo>,
    pub last_check: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateStatusResponse {
    pub state: UpdateState,
    /// The verified core waiting to be activated, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staged_core: Option<StagedCore>,
    /// Why a ready update is not installing itself right now.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<UiText>,
    /// Whether the current phase needs an explicit choice from the operator.
    pub requires_user_action: bool,
    /// Whether this process can download and install the advertised update
    /// itself; false for a desktop release seen by a headless installation.
    pub self_install: bool,
    /// The version this process is running.
    pub current_version: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ReleaseHistoryResponse {
    /// Every published release that has notes, newest first.
    pub releases: Vec<ReleaseSummary>,
    /// The version this process is running, so the list can mark it.
    pub current_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadRequest {
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct DownloadResponse {
    pub started: bool,
    pub text: UiText,
}

#[derive(Debug, Serialize)]
pub struct ApplyResponse {
    pub applying: bool,
    pub text: UiText,
}

#[derive(Debug, Serialize)]
pub struct InstallResponse {
    pub opened: bool,
    pub text: UiText,
}
