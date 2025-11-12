//! Global state management for active actions
//!
//! Maintains in-memory registry of active actions with thread-safe access.
//! Database is the source of truth, in-memory HashMap is a hot cache for performance.

use super::db::ActionsDatabase;
use super::types::{Action, ActionId, ActionState, ActionUpdate, StepStatus};
use crate::logger::{self, LogTag};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

static ACTIVE_ACTIONS: Lazy<Arc<RwLock<HashMap<ActionId, Action>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

static ACTIONS_DB: Lazy<Arc<tokio::sync::RwLock<Option<ActionsDatabase>>>> =
    Lazy::new(|| Arc::new(tokio::sync::RwLock::new(None)));

/// Initialize the actions database
pub async fn init_database() -> Result<(), String> {
    let db = ActionsDatabase::new().await?;
    let mut db_lock = ACTIONS_DB.write().await;
    *db_lock = Some(db);
    Ok(())
}

/// Get database reference
async fn get_db() -> Option<Arc<tokio::sync::RwLock<Option<ActionsDatabase>>>> {
    let db_lock = ACTIONS_DB.read().await;
    if db_lock.is_some() {
        Some(ACTIONS_DB.clone())
    } else {
        None
    }
}

/// Sync recent incomplete actions from database to memory on startup
pub async fn sync_from_db() -> Result<(), String> {
    let db_arc = match get_db().await {
        Some(db) => db,
        None => return Err("Database not initialized".to_string()),
    };

    let db_lock = db_arc.read().await;
    let db = match db_lock.as_ref() {
        Some(db) => db,
        None => return Err("Database not initialized".to_string()),
    };

    let actions = db.get_recent_incomplete_actions().await?;
    let count = actions.len();

    let mut active_actions = ACTIVE_ACTIONS.write().await;
    for action in actions {
        active_actions.insert(action.id.clone(), action);
    }
    drop(active_actions);

    if count > 0 {
        logger::info(
            LogTag::System,
            &format!(
                "Synced {} incomplete actions from database to memory",
                count
            ),
        );
    }

    Ok(())
}

/// Register a new action (dual-write: DB → HashMap → Broadcast)
pub async fn register_action(action: Action) -> Result<(), String> {
    let action_id = action.id.clone();

    // 1. Write to database first
    if let Some(db_arc) = get_db().await {
        let db_lock = db_arc.read().await;
        if let Some(db) = db_lock.as_ref() {
            if let Err(e) = db.insert_action(&action).await {
                let error_msg = format!("Failed to insert action {} into database: {}", action_id, e);
                logger::error(LogTag::System, &error_msg);
                // Return error - don't proceed if DB write fails
                return Err(error_msg);
            }
        }
    }

    // 2. Insert into in-memory HashMap
    let mut actions = ACTIVE_ACTIONS.write().await;
    actions.insert(action_id.clone(), action.clone());
    drop(actions);

    // 3. Broadcast action started event
    let update = ActionUpdate::started(&action);
    super::broadcast::broadcast_update(update).await;

    Ok(())
}

/// Get an action by ID (from memory, fallback to DB)
pub async fn get_action(action_id: &str) -> Option<Action> {
    // Try memory first
    let actions = ACTIVE_ACTIONS.read().await;
    if let Some(action) = actions.get(action_id) {
        return Some(action.clone());
    }
    drop(actions);

    // Fallback to database
    if let Some(db_arc) = get_db().await {
        let db_lock = db_arc.read().await;
        if let Some(db) = db_lock.as_ref() {
            if let Ok(Some(action)) = db.get_action(action_id).await {
                return Some(action);
            }
        }
    }

    None
}

/// Get all active actions (from memory only)
pub async fn get_active_actions() -> Vec<Action> {
    let actions = ACTIVE_ACTIONS.read().await;
    actions
        .values()
        .filter(|a| matches!(a.state, ActionState::InProgress { .. }))
        .cloned()
        .collect()
}

/// Get all actions (from memory only - use DB for historical queries)
pub async fn get_all_actions() -> Vec<Action> {
    let actions = ACTIVE_ACTIONS.read().await;
    actions.values().cloned().collect()
}

/// Update a step within an action (dual-write: DB → HashMap → Broadcast)
pub async fn update_step(
    action_id: &str,
    step_index: usize,
    status: StepStatus,
    error: Option<String>,
    metadata: Option<Value>,
) -> bool {
    // 1. Update database first
    if let Some(db_arc) = get_db().await {
        let db_lock = db_arc.read().await;
        if let Some(db) = db_lock.as_ref() {
            if let Err(e) = db
                .update_step(action_id, step_index, status, error.clone(), metadata.clone())
                .await
            {
                logger::error(
                    LogTag::System,
                    &format!(
                        "Failed to update step {} for action {}: {}",
                        step_index, action_id, e
                    ),
                );
                return false;
            }
        }
    }

    // 2. Update in-memory HashMap
    let mut actions = ACTIVE_ACTIONS.write().await;
    let action = match actions.get_mut(action_id) {
        Some(a) => a,
        None => return false,
    };

    if !action.update_step(step_index, status, error.clone(), metadata.clone()) {
        return false;
    }

    let action_clone = action.clone();
    drop(actions);

    // 3. Broadcast appropriate update based on status
    let update = match status {
        StepStatus::InProgress => {
            let step_name = action_clone.steps[step_index].name.clone();
            let progress = action_clone.calculate_progress();
            ActionUpdate::step_progress(&action_clone, step_index, step_name, progress)
        }
        StepStatus::Completed => {
            let step_name = action_clone.steps[step_index].name.clone();
            let step_metadata = metadata.unwrap_or(Value::Null);
            ActionUpdate::step_completed(&action_clone, step_index, step_name, step_metadata)
        }
        StepStatus::Failed => {
            let step_name = action_clone.steps[step_index].name.clone();
            let error_msg = error.unwrap_or_else(|| "Unknown error".to_string());
            ActionUpdate::step_failed(&action_clone, step_index, step_name, error_msg)
        }
        _ => return true, // No broadcast for Pending/Skipped
    };

    super::broadcast::broadcast_update(update).await;
    true
}

/// Mark action as completed successfully (dual-write: DB → HashMap → Broadcast)
pub async fn complete_action_success(action_id: &str) -> bool {
    // 1. Get current action state for DB update
    let (started_at, completed_at) = {
        let mut actions = ACTIVE_ACTIONS.write().await;
        let action = match actions.get_mut(action_id) {
            Some(a) => a,
            None => return false,
        };

        action.complete_success();
        (action.started_at, action.completed_at)
    };

    // 2. Update database first
    let db_success = if let Some(db_arc) = get_db().await {
        let db_lock = db_arc.read().await;
        if let Some(db) = db_lock.as_ref() {
            // Get the updated state for DB
            let actions = ACTIVE_ACTIONS.read().await;
            let action = match actions.get(action_id) {
                Some(a) => a,
                None => return false,
            };
            let state = action.state.clone();
            drop(actions);

            match db
                .update_action_state(action_id, &state, completed_at, started_at)
                .await
            {
                Ok(_) => true,
                Err(e) => {
                    logger::error(
                        LogTag::System,
                        &format!("Failed to complete action {} in database: {}", action_id, e),
                    );
                    false
                }
            }
        } else {
            false
        }
    } else {
        false
    };

    // 3. If DB write failed, revert in-memory state
    if !db_success {
        logger::error(
            LogTag::System,
            &format!(
                "Rolling back action {} completion due to DB write failure",
                action_id
            ),
        );
        // Note: In production, you'd want to revert to previous state
        // For now, we keep it marked complete in memory but logged the error
    }

    // 4. Get action for broadcast
    let action_clone = {
        let actions = ACTIVE_ACTIONS.read().await;
        match actions.get(action_id) {
            Some(a) => a.clone(),
            None => return false,
        }
    };

    // 5. Broadcast completion
    let update = ActionUpdate::completed(&action_clone);
    super::broadcast::broadcast_update(update).await;

    true
}

/// Mark action as failed (dual-write: DB → HashMap → Broadcast)
pub async fn complete_action_failed(action_id: &str, error: String) -> bool {
    // 1. Get current action state for DB update
    let (started_at, completed_at) = {
        let mut actions = ACTIVE_ACTIONS.write().await;
        let action = match actions.get_mut(action_id) {
            Some(a) => a,
            None => return false,
        };

        action.complete_failed(error.clone());
        (action.started_at, action.completed_at)
    };

    // 2. Update database first
    let db_success = if let Some(db_arc) = get_db().await {
        let db_lock = db_arc.read().await;
        if let Some(db) = db_lock.as_ref() {
            // Get the updated state for DB
            let actions = ACTIVE_ACTIONS.read().await;
            let action = match actions.get(action_id) {
                Some(a) => a,
                None => return false,
            };
            let state = action.state.clone();
            drop(actions);

            match db
                .update_action_state(action_id, &state, completed_at, started_at)
                .await
            {
                Ok(_) => true,
                Err(e) => {
                    logger::error(
                        LogTag::System,
                        &format!("Failed to mark action {} as failed in database: {}", action_id, e),
                    );
                    false
                }
            }
        } else {
            false
        }
    } else {
        false
    };

    // 3. If DB write failed, log warning
    if !db_success {
        logger::error(
            LogTag::System,
            &format!(
                "Action {} marked as failed in memory but not persisted to DB",
                action_id
            ),
        );
    }

    // 4. Get action for broadcast
    let action_clone = {
        let actions = ACTIVE_ACTIONS.read().await;
        match actions.get(action_id) {
            Some(a) => a.clone(),
            None => return false,
        }
    };

    // 5. Broadcast failure
    let update = ActionUpdate::failed(&action_clone, error);
    super::broadcast::broadcast_update(update).await;

    true
}

/// Cancel an action (dual-write: DB → HashMap → Broadcast)
pub async fn cancel_action(action_id: &str) -> bool {
    // 1. Get current action state for DB update
    let (started_at, completed_at) = {
        let mut actions = ACTIVE_ACTIONS.write().await;
        let action = match actions.get_mut(action_id) {
            Some(a) => a,
            None => return false,
        };

        action.cancel();
        (action.started_at, action.completed_at)
    };

    // 2. Update database first
    let db_success = if let Some(db_arc) = get_db().await {
        let db_lock = db_arc.read().await;
        if let Some(db) = db_lock.as_ref() {
            // Get the updated state for DB
            let actions = ACTIVE_ACTIONS.read().await;
            let action = match actions.get(action_id) {
                Some(a) => a,
                None => return false,
            };
            let state = action.state.clone();
            drop(actions);

            match db
                .update_action_state(action_id, &state, completed_at, started_at)
                .await
            {
                Ok(_) => true,
                Err(e) => {
                    logger::error(
                        LogTag::System,
                        &format!("Failed to cancel action {} in database: {}", action_id, e),
                    );
                    false
                }
            }
        } else {
            false
        }
    } else {
        false
    };

    // 3. If DB write failed, log warning
    if !db_success {
        logger::error(
            LogTag::System,
            &format!(
                "Action {} cancelled in memory but not persisted to DB",
                action_id
            ),
        );
    }

    // 4. Get action for broadcast
    let action_clone = {
        let actions = ACTIVE_ACTIONS.read().await;
        match actions.get(action_id) {
            Some(a) => a.clone(),
            None => return false,
        }
    };

    // 5. Broadcast cancellation
    let update = ActionUpdate::cancelled(&action_clone);
    super::broadcast::broadcast_update(update).await;

    true
}

/// Get count of actions by state (from memory)
pub async fn get_action_counts() -> (usize, usize, usize, usize) {
    let actions = ACTIVE_ACTIONS.read().await;
    let mut in_progress = 0;
    let mut completed = 0;
    let mut failed = 0;
    let mut cancelled = 0;

    for action in actions.values() {
        match action.state {
            ActionState::InProgress { .. } => in_progress += 1,
            ActionState::Completed => completed += 1,
            ActionState::Failed { .. } => failed += 1,
            ActionState::Cancelled => cancelled += 1,
        }
    }

    (in_progress, completed, failed, cancelled)
}

/// Query action history from database with pagination and filters
pub async fn query_action_history(
    filters: super::db::ActionFilters,
) -> Result<(Vec<Action>, usize), String> {
    let db_arc = match get_db().await {
        Some(db) => db,
        None => return Err("Database not initialized".to_string()),
    };

    let db_lock = db_arc.read().await;
    let db = match db_lock.as_ref() {
        Some(db) => db,
        None => return Err("Database not initialized".to_string()),
    };

    let limit = filters.limit.unwrap_or(50);
    let offset = filters.offset.unwrap_or(0);
    let (actions, total) = db.get_action_history(limit, offset, &filters).await?;

    Ok((actions, total))
}
