// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The trader settings PATCH refuses while promotional fixtures are shown.

use axum::{http::StatusCode, Json};

use super::getters::patch_any_config;
use crate::config;
use crate::webserver::promo;

/// The trader form is filled from the overlaid sample settings during capture, so its save
/// is refused before any merge; the process-wide flag is safe because nextest runs each
/// test in its own process.
#[tokio::test]
async fn a_trader_settings_save_is_refused_during_promo_capture() {
    promo::enable_promo_fixtures();
    let response = patch_any_config::<config::TraderConfig>(Json(
        serde_json::json!({ "max_open_positions": 10 }),
    ))
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
