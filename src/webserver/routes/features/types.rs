// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Serialization types for the dashboard feature-check route, defining
//! FeatureCheckResponse with a feature's status, availability and visibility.

use serde::Serialize;

use crate::features::FeatureStatus;

/// Response for checking a specific feature
#[derive(Serialize)]
pub struct FeatureCheckResponse {
    pub id: String,
    pub status: FeatureStatus,
    pub available: bool,
    pub visible: bool,
}
