//! Event logging and notification delivery configuration.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// EVENTS SYSTEM
// ============================================================================

config_struct! {
    /// Events system configuration
    ///
    /// WARNING: The events system stores detailed operational data and can generate
    /// 5+ GB of database storage per day. It is intended for debugging and development
    /// purposes only. Disable in production to reduce disk usage.
    pub struct EventsConfig {
        // ==================== GLOBAL CONTROL ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::GlobalControl,
        })]
        enabled: bool = false,

        // ==================== CATEGORY RECORDING ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_swap: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_transaction: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_pool: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_token: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_system: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_position: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_wallet: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_trader: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_ohlcv: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_rpc: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_api: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_security: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_connectivity: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CategoryRecording,
        })]
        record_filtering: bool = true,
    }
}
