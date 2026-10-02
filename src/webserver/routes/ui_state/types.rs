// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Request and response types for the UI state store - save, load, remove and batch persistence of dashboard key-value entries.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// UI state stored in JSON file - key-value map
pub type UiStateStore = HashMap<String, serde_json::Value>;

#[derive(Debug, Deserialize)]
pub struct SaveStateRequest {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SaveStateResponse {
    pub key: String,
    pub saved: bool,
}

#[derive(Debug, Deserialize)]
pub struct LoadStateRequest {
    pub key: String,
}

#[derive(Debug, Serialize)]
pub struct LoadStateResponse {
    pub key: String,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RemoveStateRequest {
    pub key: String,
}

#[derive(Debug, Serialize)]
pub struct RemoveStateResponse {
    pub key: String,
    pub removed: bool,
}

#[derive(Debug, Deserialize)]
pub struct BatchSaveRequest {
    pub entries: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct BatchSaveResponse {
    pub saved: usize,
}
