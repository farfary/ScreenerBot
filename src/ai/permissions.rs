use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::ai::tools::ToolCategory;
use crate::config::{update_config_section, with_config};

/// Permission level for tool execution
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionLevel {
    /// Auto-execute without asking
    Allow,
    /// Show confirmation dialog before executing
    AskUser,
    /// Never execute, return error
    Deny,
}

impl Default for PermissionLevel {
    fn default() -> Self {
        PermissionLevel::AskUser
    }
}

impl PermissionLevel {
    /// Parse permission level from string (allow, ask_user, deny)
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "allow" => PermissionLevel::Allow,
            "deny" => PermissionLevel::Deny,
            _ => PermissionLevel::AskUser, // Default to ask_user for safety
        }
    }

    /// Convert permission level to string
    pub fn to_str(&self) -> &'static str {
        match self {
            PermissionLevel::Allow => "allow",
            PermissionLevel::AskUser => "ask_user",
            PermissionLevel::Deny => "deny",
        }
    }
}

/// Tool permissions mapping (matches config schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissions {
    pub analysis: PermissionLevel,
    pub portfolio: PermissionLevel,
    pub trading: PermissionLevel,
    pub config: PermissionLevel,
    pub system: PermissionLevel,
}

impl ToolPermissions {
    /// Returns sensible defaults for tool permissions
    pub fn default() -> Self {
        Self {
            analysis: PermissionLevel::Allow,
            portfolio: PermissionLevel::Allow,
            trading: PermissionLevel::AskUser,
            config: PermissionLevel::AskUser,
            system: PermissionLevel::Allow,
        }
    }

    /// Get permission level for a specific tool category
    pub fn get_permission(&self, category: &ToolCategory) -> PermissionLevel {
        match category {
            ToolCategory::Analysis => self.analysis,
            ToolCategory::Portfolio => self.portfolio,
            ToolCategory::Trading => self.trading,
            ToolCategory::Config => self.config,
            ToolCategory::System => self.system,
        }
    }

    /// Check if a tool category can auto-execute without user confirmation
    pub fn can_auto_execute(&self, category: &ToolCategory) -> bool {
        self.get_permission(category) == PermissionLevel::Allow
    }

    /// Check if a tool category is denied
    pub fn is_denied(&self, category: &ToolCategory) -> bool {
        self.get_permission(category) == PermissionLevel::Deny
    }

    /// Check if a tool category requires user confirmation
    pub fn requires_confirmation(&self, category: &ToolCategory) -> bool {
        self.get_permission(category) == PermissionLevel::AskUser
    }
}

impl Default for ToolPermissions {
    fn default() -> Self {
        Self::default()
    }
}

/// Pending confirmation for a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingConfirmation {
    /// Unique confirmation ID
    pub id: String,
    /// AI session ID
    pub session_id: i64,
    /// Telegram message ID
    pub message_id: i64,
    /// Tool name being executed
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: serde_json::Value,
    /// Unix timestamp when created
    pub created_at: i64,
    /// Unix timestamp when expires (auto-deny)
    pub expires_at: i64,
}

impl PendingConfirmation {
    /// Create a new pending confirmation
    pub fn new(
        session_id: i64,
        message_id: i64,
        tool_name: String,
        tool_input: serde_json::Value,
        timeout_seconds: i64,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            message_id,
            tool_name,
            tool_input,
            created_at: now,
            expires_at: now + timeout_seconds,
        }
    }

    /// Check if this confirmation has expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        now >= self.expires_at
    }

    /// Get remaining time in seconds before expiry
    pub fn remaining_seconds(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        (self.expires_at - now).max(0)
    }
}

/// Default timeout for confirmations (60 seconds)
pub const DEFAULT_CONFIRMATION_TIMEOUT: i64 = 60;

/// Manager for pending tool confirmations
#[derive(Clone)]
pub struct ConfirmationManager {
    pending: Arc<DashMap<String, PendingConfirmation>>,
}

impl ConfirmationManager {
    /// Create a new confirmation manager
    pub fn new() -> Self {
        Self {
            pending: Arc::new(DashMap::new()),
        }
    }

    /// Add a pending confirmation and return its ID
    pub fn add_pending(&self, confirmation: PendingConfirmation) -> String {
        let id = confirmation.id.clone();
        self.pending.insert(id.clone(), confirmation);
        id
    }

    /// Get a pending confirmation by ID
    pub fn get_pending(&self, id: &str) -> Option<PendingConfirmation> {
        self.pending.get(id).map(|entry| entry.value().clone())
    }

    /// Confirm a pending tool execution (returns the confirmation if found)
    pub fn confirm(&self, id: &str) -> Option<PendingConfirmation> {
        self.pending
            .remove(id)
            .map(|(_, confirmation)| confirmation)
    }

    /// Deny a pending tool execution (returns the confirmation if found)
    pub fn deny(&self, id: &str) -> Option<PendingConfirmation> {
        self.pending
            .remove(id)
            .map(|(_, confirmation)| confirmation)
    }

    /// List all pending confirmations
    pub fn list_pending(&self) -> Vec<PendingConfirmation> {
        self.pending
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// List pending confirmations for a specific session
    pub fn list_pending_for_session(&self, session_id: i64) -> Vec<PendingConfirmation> {
        self.pending
            .iter()
            .filter(|entry| entry.value().session_id == session_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Cleanup expired confirmations
    pub fn cleanup_expired(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.pending
            .retain(|_, confirmation| now < confirmation.expires_at);
    }

    /// Get count of pending confirmations
    pub fn count(&self) -> usize {
        self.pending.len()
    }

    /// Clear all pending confirmations
    pub fn clear(&self) {
        self.pending.clear();
    }
}

impl Default for ConfirmationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global confirmation manager instance
static CONFIRMATION_MANAGER: once_cell::sync::Lazy<ConfirmationManager> =
    once_cell::sync::Lazy::new(|| ConfirmationManager::new());

/// Get the global confirmation manager
pub fn get_confirmation_manager() -> &'static ConfirmationManager {
    &CONFIRMATION_MANAGER
}

/// Get current tool permissions from config
pub fn get_tool_permissions() -> ToolPermissions {
    with_config(|cfg| ToolPermissions {
        analysis: PermissionLevel::from_str(&cfg.ai.tool_permissions_analysis),
        portfolio: PermissionLevel::from_str(&cfg.ai.tool_permissions_portfolio),
        trading: PermissionLevel::from_str(&cfg.ai.tool_permissions_trading),
        config: PermissionLevel::from_str(&cfg.ai.tool_permissions_config),
        system: PermissionLevel::from_str(&cfg.ai.tool_permissions_system),
    })
}

/// Update tool permissions in config
pub fn update_tool_permissions(permissions: ToolPermissions) -> Result<(), String> {
    update_config_section(
        |cfg| {
            cfg.ai.tool_permissions_analysis = permissions.analysis.to_str().to_string();
            cfg.ai.tool_permissions_portfolio = permissions.portfolio.to_str().to_string();
            cfg.ai.tool_permissions_trading = permissions.trading.to_str().to_string();
            cfg.ai.tool_permissions_config = permissions.config.to_str().to_string();
            cfg.ai.tool_permissions_system = permissions.system.to_str().to_string();
        },
        true, // Save to disk
    )
}

/// Check if a tool can be executed based on permissions
pub fn check_tool_permission(category: &ToolCategory) -> PermissionLevel {
    let permissions = get_tool_permissions();
    permissions.get_permission(category)
}

/// Check if a tool can auto-execute
pub fn can_auto_execute(category: &ToolCategory) -> bool {
    let permissions = get_tool_permissions();
    permissions.can_auto_execute(category)
}

/// Check if a tool is denied
pub fn is_tool_denied(category: &ToolCategory) -> bool {
    let permissions = get_tool_permissions();
    permissions.is_denied(category)
}

/// Check if a tool requires confirmation
pub fn requires_confirmation(category: &ToolCategory) -> bool {
    let permissions = get_tool_permissions();
    permissions.requires_confirmation(category)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_level_equality() {
        assert_eq!(PermissionLevel::Allow, PermissionLevel::Allow);
        assert_ne!(PermissionLevel::Allow, PermissionLevel::Deny);
    }

    #[test]
    fn test_tool_permissions_default() {
        let perms = ToolPermissions::default();
        assert_eq!(perms.analysis, PermissionLevel::Allow);
        assert_eq!(perms.portfolio, PermissionLevel::Allow);
        assert_eq!(perms.trading, PermissionLevel::AskUser);
        assert_eq!(perms.config, PermissionLevel::AskUser);
        assert_eq!(perms.system, PermissionLevel::Allow);
    }

    #[test]
    fn test_can_auto_execute() {
        let perms = ToolPermissions::default();
        assert!(perms.can_auto_execute(&ToolCategory::Analysis));
        assert!(perms.can_auto_execute(&ToolCategory::Portfolio));
        assert!(!perms.can_auto_execute(&ToolCategory::Trading));
        assert!(!perms.can_auto_execute(&ToolCategory::Config));
        assert!(perms.can_auto_execute(&ToolCategory::System));
    }

    #[test]
    fn test_requires_confirmation() {
        let perms = ToolPermissions::default();
        assert!(!perms.requires_confirmation(&ToolCategory::Analysis));
        assert!(perms.requires_confirmation(&ToolCategory::Trading));
        assert!(perms.requires_confirmation(&ToolCategory::Config));
    }

    #[test]
    fn test_pending_confirmation_creation() {
        let confirmation = PendingConfirmation::new(
            1,
            123,
            "test_tool".to_string(),
            serde_json::json!({"param": "value"}),
            60,
        );

        assert_eq!(confirmation.session_id, 1);
        assert_eq!(confirmation.message_id, 123);
        assert_eq!(confirmation.tool_name, "test_tool");
        assert!(!confirmation.id.is_empty());
        assert!(!confirmation.is_expired());
        assert!(confirmation.remaining_seconds() > 0);
    }

    #[test]
    fn test_confirmation_manager_add_and_get() {
        let manager = ConfirmationManager::new();
        let confirmation =
            PendingConfirmation::new(1, 123, "test_tool".to_string(), serde_json::json!({}), 60);

        let id = manager.add_pending(confirmation.clone());
        let retrieved = manager.get_pending(&id);

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[test]
    fn test_confirmation_manager_confirm() {
        let manager = ConfirmationManager::new();
        let confirmation =
            PendingConfirmation::new(1, 123, "test_tool".to_string(), serde_json::json!({}), 60);

        let id = manager.add_pending(confirmation);
        let confirmed = manager.confirm(&id);

        assert!(confirmed.is_some());
        assert!(manager.get_pending(&id).is_none());
    }

    #[test]
    fn test_confirmation_manager_deny() {
        let manager = ConfirmationManager::new();
        let confirmation =
            PendingConfirmation::new(1, 123, "test_tool".to_string(), serde_json::json!({}), 60);

        let id = manager.add_pending(confirmation);
        let denied = manager.deny(&id);

        assert!(denied.is_some());
        assert!(manager.get_pending(&id).is_none());
    }

    #[test]
    fn test_confirmation_manager_list_pending() {
        let manager = ConfirmationManager::new();

        manager.add_pending(PendingConfirmation::new(
            1,
            123,
            "tool1".to_string(),
            serde_json::json!({}),
            60,
        ));

        manager.add_pending(PendingConfirmation::new(
            2,
            456,
            "tool2".to_string(),
            serde_json::json!({}),
            60,
        ));

        let pending = manager.list_pending();
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_confirmation_manager_list_by_session() {
        let manager = ConfirmationManager::new();

        manager.add_pending(PendingConfirmation::new(
            1,
            123,
            "tool1".to_string(),
            serde_json::json!({}),
            60,
        ));

        manager.add_pending(PendingConfirmation::new(
            1,
            456,
            "tool2".to_string(),
            serde_json::json!({}),
            60,
        ));

        manager.add_pending(PendingConfirmation::new(
            2,
            789,
            "tool3".to_string(),
            serde_json::json!({}),
            60,
        ));

        let pending = manager.list_pending_for_session(1);
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_confirmation_manager_clear() {
        let manager = ConfirmationManager::new();

        manager.add_pending(PendingConfirmation::new(
            1,
            123,
            "tool1".to_string(),
            serde_json::json!({}),
            60,
        ));

        assert_eq!(manager.count(), 1);
        manager.clear();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_expired_confirmation() {
        let confirmation = PendingConfirmation::new(
            1,
            123,
            "test_tool".to_string(),
            serde_json::json!({}),
            0, // Immediate expiry
        );

        // Give it a moment to expire
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(confirmation.is_expired());
        assert_eq!(confirmation.remaining_seconds(), 0);
    }
}
