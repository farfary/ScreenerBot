// Smart priority system with activity-based throttling

use crate::ohlcvs::types::{ Priority, TokenOhlcvConfig };
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
        hours_since_activity: f64
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
            return RecommendedAction::Throttle(
                Duration::from_secs(((config.fetch_frequency.as_secs() as f64) * 2.0) as u64)
            );
        }

        // Very inactive - pause
        if hours_inactive > 168.0 {
            // 1 week
            return RecommendedAction::Pause;
        }

        // Default - throttle moderately
        RecommendedAction::Throttle(
            Duration::from_secs(((config.fetch_frequency.as_secs() as f64) * 1.5) as u64)
        )
    }

    /// Update priority based on new activity
    pub fn update_priority_on_activity(
        current_priority: Priority,
        activity_type: ActivityType
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
