use crate::filtering::types::PassedToken;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::time::{Duration, Instant};
use uuid::Uuid;

const SESSION_TTL: Duration = Duration::from_secs(60 * 15); // 15 minutes
const DEFAULT_PAGE_SIZE: usize = 10;

pub struct PaginationSession {
    pub items: Vec<PassedToken>,
    pub items_per_page: usize,
    pub created_at: Instant,
}

pub struct PaginationManager {
    sessions: DashMap<String, PaginationSession>,
}

/// Global pagination manager singleton
pub static PAGINATION_MANAGER: Lazy<PaginationManager> = Lazy::new(PaginationManager::new);

impl PaginationManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    /// Create a new pagination session and return the session ID
    pub fn create_session(&self, items: Vec<PassedToken>, items_per_page: Option<usize>) -> String {
        // Opportunistic cleanup
        self.cleanup();

        let session_id = Uuid::new_v4().to_string();
        self.sessions.insert(
            session_id.clone(),
            PaginationSession {
                items,
                items_per_page: items_per_page.unwrap_or(DEFAULT_PAGE_SIZE),
                created_at: Instant::now(),
            },
        );
        session_id
    }

    /// Get items for a specific page (0-indexed)
    /// Returns: (items_for_page, total_pages, total_items)
    pub fn get_page(
        &self,
        session_id: &str,
        page: usize,
    ) -> Option<(Vec<PassedToken>, usize, usize)> {
        let session = self.sessions.get(session_id)?;

        let total_items = session.items.len();
        if total_items == 0 {
            return Some((Vec::new(), 0, 0));
        }

        let total_pages = (total_items + session.items_per_page - 1) / session.items_per_page;

        // Clamp page to valid range
        let page = if page >= total_pages {
            total_pages.saturating_sub(1)
        } else {
            page
        };

        let start_idx = page * session.items_per_page;
        let end_idx = std::cmp::min(start_idx + session.items_per_page, total_items);

        if start_idx >= total_items {
            return Some((Vec::new(), total_pages, total_items));
        }

        let page_items = session.items[start_idx..end_idx].to_vec();
        Some((page_items, total_pages, total_items))
    }

    pub fn cleanup(&self) {
        // Remove sessions older than TTL
        let now = Instant::now();
        self.sessions
            .retain(|_, session| now.duration_since(session.created_at) < SESSION_TTL);
    }
}
