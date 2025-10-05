/// API request type definitions
///
/// Standard request structures for REST API endpoints

use serde::{ Deserialize, Serialize };

/// WebSocket subscription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeRequest {
    pub r#type: String,
    pub channels: Vec<String>,
}

/// WebSocket unsubscribe request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeRequest {
    pub r#type: String,
    pub channels: Vec<String>,
}

/// Standard pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: usize,

    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: default_page(),
            page_size: default_page_size(),
        }
    }
}

impl PaginationParams {
    /// Calculate offset for SQL queries
    pub fn offset(&self) -> usize {
        self.page.saturating_sub(1) * self.page_size
    }

    /// Validate parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.page == 0 {
            return Err("Page must be >= 1".to_string());
        }

        if self.page_size == 0 || self.page_size > 100 {
            return Err("Page size must be between 1 and 100".to_string());
        }

        Ok(())
    }
}
