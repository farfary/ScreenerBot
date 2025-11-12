//! Action types for tracking in-progress operations
//!
//! Provides granular progress tracking for all bot operations (swaps, positions, etc.)
//! with step-level status updates for real-time user feedback.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Unique identifier for an action (UUID format)
pub type ActionId = String;

/// Complete action with all metadata and steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Unique action identifier
    pub id: ActionId,

    /// Type of action being performed
    pub action_type: ActionType,

    /// Entity this action is operating on (mint address, position ID, etc.)
    pub entity_id: String,

    /// Current action state
    pub state: ActionState,

    /// All steps in this action
    pub steps: Vec<ActionStep>,

    /// Index of currently executing step (0-based)
    pub current_step_index: usize,

    /// When action was started
    pub started_at: DateTime<Utc>,

    /// When action completed (success or failure)
    pub completed_at: Option<DateTime<Utc>>,

    /// Additional metadata (symbol, amounts, etc.)
    pub metadata: Value,
}

impl Action {
    /// Create new action with predefined steps
    pub fn new(
        id: ActionId,
        action_type: ActionType,
        entity_id: String,
        step_names: Vec<String>,
        metadata: Value,
    ) -> Self {
        let first_step = step_names.first().cloned().unwrap_or_default();
        let total_steps = step_names.len();

        let steps: Vec<ActionStep> = step_names
            .into_iter()
            .enumerate()
            .map(|(index, name)| ActionStep {
                step_id: format!("{}-step-{}", id, index),
                name,
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                error: None,
                metadata: Value::Null,
            })
            .collect();

        Self {
            id,
            action_type,
            entity_id,
            state: ActionState::InProgress {
                current_step: first_step,
                current_step_index: 0,
                total_steps,
                progress_pct: 0,
            },
            steps,
            current_step_index: 0,
            started_at: Utc::now(),
            completed_at: None,
            metadata,
        }
    }

    /// Calculate current progress percentage
    pub fn calculate_progress(&self) -> u8 {
        if self.steps.is_empty() {
            return 0;
        }

        let completed = self
            .steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Completed))
            .count();

        ((completed as f32 / self.steps.len() as f32) * 100.0) as u8
    }

    /// Update current step status and metadata
    pub fn update_step(
        &mut self,
        step_index: usize,
        status: StepStatus,
        error: Option<String>,
        metadata: Option<Value>,
    ) -> bool {
        if step_index >= self.steps.len() {
            return false;
        }

        let step = &mut self.steps[step_index];

        match status {
            StepStatus::InProgress if step.started_at.is_none() => {
                step.started_at = Some(Utc::now());
            }
            StepStatus::Completed | StepStatus::Failed | StepStatus::Skipped
                if step.completed_at.is_none() =>
            {
                step.completed_at = Some(Utc::now());
            }
            _ => {}
        }

        step.status = status;
        if let Some(err) = error {
            step.error = Some(err);
        }
        if let Some(meta) = metadata {
            step.metadata = meta;
        }

        let step_name = self.steps[step_index].name.clone();
        self.current_step_index = step_index;

        // Update action state
        let progress = self.calculate_progress();
        if let ActionState::InProgress { total_steps, .. } = &self.state {
            self.state = ActionState::InProgress {
                current_step: step_name,
                current_step_index: step_index,
                total_steps: *total_steps,
                progress_pct: progress,
            };
        }

        true
    }

    /// Mark action as completed successfully
    pub fn complete_success(&mut self) {
        self.state = ActionState::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark action as failed
    pub fn complete_failed(&mut self, error: String) {
        self.state = ActionState::Failed { error };
        self.completed_at = Some(Utc::now());
    }

    /// Mark action as cancelled
    pub fn cancel(&mut self) {
        self.state = ActionState::Cancelled;
        self.completed_at = Some(Utc::now());
    }
}

/// Type of action being performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Buying a token (swap SOL -> Token)
    SwapBuy,

    /// Selling a token (swap Token -> SOL)
    SwapSell,

    /// Opening a new position
    PositionOpen,

    /// Closing an existing position
    PositionClose,

    /// Adding to position (DCA)
    PositionDca,

    /// Partial exit from position
    PositionPartialExit,

    /// Manual order placement
    ManualOrder,
}

impl ActionType {
    /// Get human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SwapBuy => "Buying Token",
            Self::SwapSell => "Selling Token",
            Self::PositionOpen => "Opening Position",
            Self::PositionClose => "Closing Position",
            Self::PositionDca => "Adding to Position (DCA)",
            Self::PositionPartialExit => "Partial Exit",
            Self::ManualOrder => "Manual Order",
        }
    }

    /// Get icon emoji for UI
    pub fn icon(&self) -> &'static str {
        match self {
            Self::SwapBuy => "💰",
            Self::PositionOpen => "🔓",
            Self::SwapSell => "💸",
            Self::PositionClose => "🔒",
            Self::PositionDca => "📈",
            Self::PositionPartialExit => "📉",
            Self::ManualOrder => "🎯",
        }
    }
}

/// Current state of an action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ActionState {
    /// Action is currently executing
    InProgress {
        current_step: String,
        current_step_index: usize,
        total_steps: usize,
        progress_pct: u8,
    },

    /// Action completed successfully
    Completed,

    /// Action failed with error
    Failed { error: String },

    /// Action was cancelled by user or system
    Cancelled,
}

/// Individual step within an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    /// Unique step identifier
    pub step_id: String,

    /// Human-readable step name
    pub name: String,

    /// Current step status
    pub status: StepStatus,

    /// When step started executing
    pub started_at: Option<DateTime<Utc>>,

    /// When step completed (success or failure)
    pub completed_at: Option<DateTime<Utc>>,

    /// Error message if step failed
    pub error: Option<String>,

    /// Additional step metadata
    pub metadata: Value,
}

/// Status of a single step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Step has not started yet
    Pending,

    /// Step is currently executing
    InProgress,

    /// Step completed successfully
    Completed,

    /// Step failed with error
    Failed,

    /// Step was skipped (conditional logic)
    Skipped,
}

/// Action update event for broadcasting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionUpdate {
    /// Action being updated
    pub action_id: ActionId,

    /// Type of update
    pub update_type: UpdateType,

    /// When update occurred
    pub timestamp: DateTime<Utc>,

    /// Update payload
    pub data: Value,

    /// Snapshot of the action after this update (if available)
    pub action: Option<Action>,
}

impl ActionUpdate {
    /// Create action started update
    pub fn started(action: &Action) -> Self {
        Self {
            action_id: action.id.clone(),
            update_type: UpdateType::ActionStarted,
            timestamp: Utc::now(),
            data: serde_json::json!({
                "action_type": action.action_type,
                "entity_id": action.entity_id,
                "total_steps": action.steps.len(),
                "metadata": action.metadata,
            }),
            action: Some(action.clone()),
        }
    }

    /// Create step progress update
    pub fn step_progress(
        action: &Action,
        step_index: usize,
        step_name: String,
        progress_pct: u8,
    ) -> Self {
        Self {
            action_id: action.id.clone(),
            update_type: UpdateType::StepProgress,
            timestamp: Utc::now(),
            data: serde_json::json!({
                "step_index": step_index,
                "step_name": step_name,
                "progress_pct": progress_pct,
            }),
            action: Some(action.clone()),
        }
    }

    /// Create step completed update
    pub fn step_completed(
        action: &Action,
        step_index: usize,
        step_name: String,
        metadata: Value,
    ) -> Self {
        Self {
            action_id: action.id.clone(),
            update_type: UpdateType::StepCompleted,
            timestamp: Utc::now(),
            data: serde_json::json!({
                "step_index": step_index,
                "step_name": step_name,
                "metadata": metadata,
            }),
            action: Some(action.clone()),
        }
    }

    /// Create step failed update
    pub fn step_failed(
        action: &Action,
        step_index: usize,
        step_name: String,
        error: String,
    ) -> Self {
        Self {
            action_id: action.id.clone(),
            update_type: UpdateType::StepFailed,
            timestamp: Utc::now(),
            data: serde_json::json!({
                "step_index": step_index,
                "step_name": step_name,
                "error": error,
            }),
            action: Some(action.clone()),
        }
    }

    /// Create action completed update
    pub fn completed(action: &Action) -> Self {
        Self {
            action_id: action.id.clone(),
            update_type: UpdateType::ActionCompleted,
            timestamp: Utc::now(),
            data: serde_json::json!({
                "completed_at": action.completed_at,
                "duration_ms": action.completed_at.map(|end| (end - action.started_at).num_milliseconds()),
            }),
            action: Some(action.clone()),
        }
    }

    /// Create action failed update
    pub fn failed(action: &Action, error: String) -> Self {
        Self {
            action_id: action.id.clone(),
            update_type: UpdateType::ActionFailed,
            timestamp: Utc::now(),
            data: serde_json::json!({
                "error": error,
                "completed_at": action.completed_at,
            }),
            action: Some(action.clone()),
        }
    }

    /// Create action cancelled update
    pub fn cancelled(action: &Action) -> Self {
        Self {
            action_id: action.id.clone(),
            update_type: UpdateType::ActionCancelled,
            timestamp: Utc::now(),
            data: serde_json::json!({
                "completed_at": action.completed_at,
            }),
            action: Some(action.clone()),
        }
    }
}

/// Type of action update
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateType {
    /// Action has started
    ActionStarted,

    /// Step is progressing
    StepProgress,

    /// Step completed successfully
    StepCompleted,

    /// Step failed
    StepFailed,

    /// Action completed successfully
    ActionCompleted,

    /// Action failed
    ActionFailed,

    /// Action was cancelled
    ActionCancelled,
}
