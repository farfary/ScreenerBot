use axum::{
    extract::Json,
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::paths;
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

// =============================================================================
// TYPES
// =============================================================================

/// UI state stored in JSON file - key-value map
type UiStateStore = HashMap<String, serde_json::Value>;

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

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Load the UI state store from disk
fn load_store() -> UiStateStore {
    let path = paths::get_ui_state_path();

    if !path.exists() {
        return HashMap::new();
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

/// Save the UI state store to disk
fn save_store(store: &UiStateStore) -> Result<(), String> {
    let path = paths::get_ui_state_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let content =
        serde_json::to_string_pretty(store).map_err(|e| format!("Failed to serialize: {}", e))?;

    std::fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

// =============================================================================
// ROUTE HANDLERS
// =============================================================================

/// GET /api/ui-state/all - Load ALL state (for initial page load)
async fn load_all_state() -> Response {
    let store = load_store();
    success_response(store)
}

/// POST /api/ui-state/save - Save a single key-value pair
async fn save_state(Json(req): Json<SaveStateRequest>) -> Response {
    let mut store = load_store();
    store.insert(req.key.clone(), req.value);

    if let Err(e) = save_store(&store) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "save_failed",
            &format!("Failed to save state: {}", e),
            None,
        );
    }

    success_response(SaveStateResponse {
        key: req.key,
        saved: true,
    })
}

/// POST /api/ui-state/batch-save - Save multiple key-value pairs at once
async fn batch_save_state(Json(req): Json<BatchSaveRequest>) -> Response {
    let mut store = load_store();
    let count = req.entries.len();

    for (key, value) in req.entries {
        store.insert(key, value);
    }

    if let Err(e) = save_store(&store) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "save_failed",
            &format!("Failed to save state: {}", e),
            None,
        );
    }

    success_response(BatchSaveResponse { saved: count })
}

/// POST /api/ui-state/load - Load a single key's value
async fn load_state(Json(req): Json<LoadStateRequest>) -> Response {
    let store = load_store();
    let value = store.get(&req.key).cloned();

    success_response(LoadStateResponse {
        key: req.key,
        value,
    })
}

/// POST /api/ui-state/remove - Remove a single key
async fn remove_state(Json(req): Json<RemoveStateRequest>) -> Response {
    let mut store = load_store();
    let existed = store.remove(&req.key).is_some();

    if existed {
        if let Err(e) = save_store(&store) {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "save_failed",
                &format!("Failed to save state: {}", e),
                None,
            );
        }
    }

    success_response(RemoveStateResponse {
        key: req.key,
        removed: existed,
    })
}

/// POST /api/ui-state/clear - Clear all UI state
async fn clear_state() -> Response {
    let store = HashMap::new();

    if let Err(e) = save_store(&store) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "clear_failed",
            &format!("Failed to clear state: {}", e),
            None,
        );
    }

    success_response(serde_json::json!({ "cleared": true }))
}

// =============================================================================
// ROUTES
// =============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/ui-state/all", get(load_all_state))
        .route("/ui-state/save", post(save_state))
        .route("/ui-state/batch-save", post(batch_save_state))
        .route("/ui-state/load", post(load_state))
        .route("/ui-state/remove", post(remove_state))
        .route("/ui-state/clear", post(clear_state))
}
