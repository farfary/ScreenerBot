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

/// Action history response with pagination
#[derive(Debug, Serialize)]
pub struct ActionHistoryResponse {
    pub actions: Vec<crate::actions::Action>,
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
