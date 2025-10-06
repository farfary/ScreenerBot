// Smart priority system with activity-based throttling

use crate::ohlcvs::types::{Priority, TokenOhlcvConfig};
use chrono::Utc;
use std::time::Duration;

pub struct PriorityManager;

impl PriorityManager {
    /// Calculate priority score for a token
    /// Higher score = higher priority
    pub fn calculate_priority_score(
        is_open_position: bool,
        recent_views: u32,
        recent_trades: u32,
        hours_since_activity: f64,
    ) -> u32 {
        let mut score = 0u32;

        // Open positions get maximum priority
        if is_open_position {
            score += 100;
        }

        // Recent views contribute to priority
        score += recent_views * 10;

        // Recent trades contribute more
        score += recent_trades * 50;

        // Decay based on inactivity
        let decay_factor = 1.0 / (1.0 + hours_since_activity / 24.0);
        score = ((score as f64) * decay_factor) as u32;

        score
    }

    /// Determine priority level from score
    pub fn priority_from_score(score: u32) -> Priority {
        match score {
            100.. => Priority::Critical,
            50..=99 => Priority::High,
            10..=49 => Priority::Medium,
            _ => Priority::Low,
        }
    }

    /// Calculate adjusted fetch interval based on activity
    pub fn calculate_fetch_interval(config: &TokenOhlcvConfig) -> Duration {
        config.calculate_adjusted_interval()
    }

    /// Should throttle based on empty fetches
    pub fn should_throttle(config: &TokenOhlcvConfig) -> bool {
        // Throttle if we've had 5+ consecutive empty fetches
        config.consecutive_empty_fetches >= 5
    }

    /// Calculate throttle multiplier
    pub fn throttle_multiplier(consecutive_empty_fetches: u32) -> f64 {
        // Exponential backoff: 1.0, 1.5, 2.0, 2.5, 3.0 (capped at 3x)
        (1.0 + (consecutive_empty_fetches as f64) * 0.5).min(3.0)
    }

    /// Get recommended action for a token
    pub fn get_recommended_action(config: &TokenOhlcvConfig) -> RecommendedAction {
        let hours_inactive = (Utc::now() - config.last_activity).num_hours() as f64;

        // Critical priority - always fetch
        if config.priority == Priority::Critical {
            return RecommendedAction::FetchNow;
        }

        // Too many empty fetches - pause
        if config.consecutive_empty_fetches >= 10 {
            return RecommendedAction::Pause;
        }

        // Recently active - fetch normally
        if hours_inactive < 1.0 {
            return RecommendedAction::FetchNow;
        }

        // Moderately inactive - throttle
        if hours_inactive < 24.0 {
            return RecommendedAction::Throttle(Duration::from_secs(
                ((config.fetch_frequency.as_secs() as f64) * 2.0) as u64,
            ));
        }

        // Very inactive - pause
        if hours_inactive > 168.0 {
            // 1 week
            return RecommendedAction::Pause;
        }

        // Default - throttle moderately
        RecommendedAction::Throttle(Duration::from_secs(
            ((config.fetch_frequency.as_secs() as f64) * 1.5) as u64,
        ))
    }

    /// Update priority based on new activity
    pub fn update_priority_on_activity(
        current_priority: Priority,
        activity_type: ActivityType,
    ) -> Priority {
        match activity_type {
            ActivityType::PositionOpened => Priority::Critical,
            ActivityType::PositionClosed => {
                // Downgrade from critical but keep high
                if current_priority == Priority::Critical {
                    Priority::High
                } else {
                    current_priority
                }
            }
            ActivityType::TokenViewed => {
                // Upgrade if low, maintain if higher
                match current_priority {
                    Priority::Low => Priority::Medium,
                    _ => current_priority,
                }
            }
            ActivityType::ChartViewed => {
                // Significant interest, upgrade
                match current_priority {
                    Priority::Low => Priority::Medium,
                    Priority::Medium => Priority::High,
                    _ => current_priority,
                }
            }
            ActivityType::DataRequested => Priority::High, // Immediate data need
        }
    }

    /// Calculate optimal batch size for fetching
    pub fn calculate_batch_size(priority: Priority) -> usize {
        match priority {
            Priority::Critical => 1000, // Max candles
            Priority::High => 500,
            Priority::Medium => 200,
            Priority::Low => 100,
        }
    }

    /// Get fetch timeout based on priority
    pub fn get_fetch_timeout(priority: Priority) -> Duration {
        match priority {
            Priority::Critical => Duration::from_secs(60),
            Priority::High => Duration::from_secs(45),
            Priority::Medium => Duration::from_secs(30),
            Priority::Low => Duration::from_secs(15),
        }
    }

    /// Should retry on failure
    pub fn should_retry(priority: Priority, attempt: u32) -> bool {
        let max_retries = match priority {
            Priority::Critical => 5,
            Priority::High => 3,
            Priority::Medium => 2,
            Priority::Low => 1,
        };

        attempt < max_retries
    }

    /// Calculate retry delay with exponential backoff
    pub fn retry_delay(attempt: u32) -> Duration {
        let base_delay = Duration::from_secs(2);
        let multiplier = (2u32).pow(attempt.min(5)); // Cap at 2^5 = 32
        base_delay * multiplier
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityType {
    PositionOpened,
    PositionClosed,
    TokenViewed,
    ChartViewed,
    DataRequested,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecommendedAction {
    FetchNow,
    Throttle(Duration),
    Pause,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_score_calculation() {
        // Open position should have highest score
        let score1 = PriorityManager::calculate_priority_score(true, 0, 0, 0.0);
        assert!(score1 >= 100);

        // Recent activity should have higher score than inactive
        let score2 = PriorityManager::calculate_priority_score(false, 10, 5, 1.0);
        let score3 = PriorityManager::calculate_priority_score(false, 10, 5, 100.0);
        assert!(score2 > score3);

        // Trades should contribute more than views
        let score4 = PriorityManager::calculate_priority_score(false, 10, 0, 0.0);
        let score5 = PriorityManager::calculate_priority_score(false, 0, 2, 0.0);
        assert!(score5 > score4);
    }

    #[test]
    fn test_priority_from_score() {
        assert_eq!(
            PriorityManager::priority_from_score(150),
            Priority::Critical
        );
        assert_eq!(PriorityManager::priority_from_score(75), Priority::High);
        assert_eq!(PriorityManager::priority_from_score(25), Priority::Medium);
        assert_eq!(PriorityManager::priority_from_score(5), Priority::Low);
    }

    #[test]
    fn test_throttle_multiplier() {
        assert_eq!(PriorityManager::throttle_multiplier(0), 1.0);
        assert_eq!(PriorityManager::throttle_multiplier(2), 2.0);
        assert_eq!(PriorityManager::throttle_multiplier(10), 3.0); // Capped
    }

    #[test]
    fn test_update_priority_on_activity() {
        assert_eq!(
            PriorityManager::update_priority_on_activity(
                Priority::Low,
                ActivityType::PositionOpened
            ),
            Priority::Critical
        );

        assert_eq!(
            PriorityManager::update_priority_on_activity(
                Priority::Critical,
                ActivityType::PositionClosed
            ),
            Priority::High
        );

        assert_eq!(
            PriorityManager::update_priority_on_activity(Priority::Low, ActivityType::TokenViewed),
            Priority::Medium
        );
    }

    #[test]
    fn test_batch_size_calculation() {
        assert_eq!(
            PriorityManager::calculate_batch_size(Priority::Critical),
            1000
        );
        assert_eq!(PriorityManager::calculate_batch_size(Priority::High), 500);
        assert_eq!(PriorityManager::calculate_batch_size(Priority::Medium), 200);
        assert_eq!(PriorityManager::calculate_batch_size(Priority::Low), 100);
    }

    #[test]
    fn test_retry_logic() {
        assert!(PriorityManager::should_retry(Priority::Critical, 3));
        assert!(!PriorityManager::should_retry(Priority::Critical, 6));
        assert!(!PriorityManager::should_retry(Priority::Low, 2));
    }

    #[test]
    fn test_retry_delay() {
        assert_eq!(PriorityManager::retry_delay(0), Duration::from_secs(2));
        assert_eq!(PriorityManager::retry_delay(1), Duration::from_secs(4));
        assert_eq!(PriorityManager::retry_delay(2), Duration::from_secs(8));
        assert_eq!(PriorityManager::retry_delay(10), Duration::from_secs(64)); // Capped at 2^5
    }
}
