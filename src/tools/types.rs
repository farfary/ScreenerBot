//! Common types for tools module

use serde::{Deserialize, Serialize};

/// Result type for tool operations
pub type ToolResult<T> = Result<T, String>;

/// Status of a tool execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    /// Tool is initialized and ready to run
    Ready,
    /// Tool is currently executing
    Running,
    /// Tool completed successfully
    Completed,
    /// Tool failed with an error
    Failed,
    /// Tool was aborted by user
    Aborted,
}

impl std::fmt::Display for ToolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolStatus::Ready => write!(f, "ready"),
            ToolStatus::Running => write!(f, "running"),
            ToolStatus::Completed => write!(f, "completed"),
            ToolStatus::Failed => write!(f, "failed"),
            ToolStatus::Aborted => write!(f, "aborted"),
        }
    }
}

impl Default for ToolStatus {
    fn default() -> Self {
        ToolStatus::Ready
    }
}
