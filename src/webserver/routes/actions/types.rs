// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Request and response types for the actions API - ActiveActionsResponse, ActionHistoryResponse, ActionHistoryQuery and ActionMutationResponse.

use serde::{Deserialize, Serialize};

/// Active actions response
#[derive(Debug, Serialize)]
pub struct ActiveActionsResponse {
    pub actions: Vec<crate::actions::Action>,
    pub count: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
    pub unread: usize,
}

/// One listed action with the settled result of its swap, once booked.
#[derive(Debug, Serialize)]
pub struct ActionHistoryRow {
    #[serde(flatten)]
    pub action: crate::actions::Action,
    pub outcome: Option<super::outcome::ActionOutcome>,
    /// The action completed with a swap signature that no position booked, and the
    /// positions verifier has stopped trying: the swap may never have landed.
    pub unrecorded: bool,
}

/// Action history response with pagination
#[derive(Debug, Serialize)]
pub struct ActionHistoryResponse {
    pub actions: Vec<ActionHistoryRow>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

/// Action history query parameters
#[derive(Debug, Deserialize)]
pub struct ActionHistoryQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    /// A trade side (`buy`, `sell`, `dca`) or one action type id.
    pub action_type: Option<String>,
    pub entity_id: Option<String>,
    pub state: Option<String>,
    pub started_after: Option<String>,
    pub started_before: Option<String>,
}

fn default_limit() -> usize {
    50
}

/// Subscriber count response
#[derive(Debug, Serialize)]
pub struct SubscriberCountResponse {
    pub subscriber_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ActionMutationResponse {
    pub success: bool,
    pub updated: usize,
    pub unread: usize,
}
