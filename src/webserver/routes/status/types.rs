// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Response types for the status and health-check API routes.

use serde::Serialize;

/// Simple health check response
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub instance_id: String,
}

/// Whether the process may be restarted now without interrupting trading.
#[derive(Debug, Clone, Serialize)]
pub struct RestartSafetyResponse {
    pub safe_to_restart: bool,
}
