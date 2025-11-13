//! Actions module for tracking in-progress operations
//!
//! Provides granular progress tracking for all bot operations with real-time updates.
//! Supports swaps, position operations, and any future async operations.
//!
//! # Architecture
//!
//! - **types**: Action, Step, and Update type definitions
//! - **state**: Global in-memory registry of active actions (hot cache)
//! - **db**: SQLite database for persistent storage (source of truth)
//! - **broadcast**: Pub/sub channel for real-time updates via SSE
//!
//! # Usage Example
//!
//! ```rust,no_run
//! use crate::actions::*;
//! use uuid::Uuid;
//!
//! async fn my_operation() -> Result<(), String> {
//!     let action_id = Uuid::new_v4().to_string();
//!     let action = Action::new(
//!         action_id.clone(),
//!         ActionType::SwapBuy,
//!         "token_mint".to_string(),
//!         vec!["Step 1".to_string(), "Step 2".to_string()],
//!         serde_json::json!({"symbol": "TOKEN"}),
//!     );
//!     
//!     state::register_action(action).await?;
//!     
//!     state::update_step(&action_id, 0, StepStatus::InProgress, None, None).await;
//!     // ... do work ...
//!     state::update_step(&action_id, 0, StepStatus::Completed, None, None).await;
//!     
//!     state::update_step(&action_id, 1, StepStatus::InProgress, None, None).await;
//!     // ... do work ...
//!     state::update_step(&action_id, 1, StepStatus::Completed, None, None).await;
//!     
//!     state::complete_action_success(&action_id).await;
//!     Ok(())
//! }
//! ```

pub mod broadcast;
pub mod db;
pub mod state;
pub mod types;

// Re-export commonly used types
pub use types::{
    Action, ActionId, ActionState, ActionStep, ActionType, ActionUpdate, StepStatus, UpdateType,
};

// Re-export state functions
pub use state::{
    cancel_action, complete_action_failed, complete_action_success, get_action, get_action_counts,
    get_active_actions, get_all_actions, init_database, query_action_history, register_action,
    sync_from_db, update_step,
};

// Re-export broadcast functions
pub use broadcast::{broadcast_update, subscribe, subscriber_count};

// Re-export database types
pub use db::{ActionFilters, ActionsDatabase};
