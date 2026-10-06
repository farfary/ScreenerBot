// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! RPC endpoint configuration.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// RPC endpoint configuration
    pub struct RpcConfig {
        /// List of RPC URLs to use (round-robin)
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Endpoints,
        })]
        urls: Vec<String> = vec!["https://api.mainnet-beta.solana.com".to_owned()],

        // Provider Selection
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::ProviderSelection,
        })]
        selection_strategy: String = "adaptive".to_owned(),

        // Rate Limiting
        #[metadata(field_metadata! {
            min: 1,
            max: 100,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::RateLimiting,
        })]
        default_rate_limit: u32 = 10,
        #[metadata(field_metadata! {
            min: 1,
            max: 200,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::RateLimiting,
        })]
        helius_rate_limit: u32 = 50,
        #[metadata(field_metadata! {
            min: 1,
            max: 100,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::RateLimiting,
        })]
        quicknode_rate_limit: u32 = 25,
        #[metadata(field_metadata! {
            min: 1,
            max: 200,
            step: 10,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::RateLimiting,
        })]
        triton_rate_limit: u32 = 100,
        #[metadata(field_metadata! {
            min: 1,
            max: 20,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::RateLimiting,
        })]
        public_rate_limit: u32 = 4,

        // Circuit Breaker
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::CircuitBreaker,
        })]
        circuit_breaker_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 1,
            max: 20,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CircuitBreaker,
        })]
        circuit_breaker_failure_threshold: u32 = 5,
        #[metadata(field_metadata! {
            min: 1,
            max: 10,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CircuitBreaker,
        })]
        circuit_breaker_success_threshold: u32 = 3,
        #[metadata(field_metadata! {
            min: 5,
            max: 120,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CircuitBreaker,
        })]
        circuit_breaker_open_duration_secs: u64 = 30,
        #[metadata(field_metadata! {
            min: 1,
            max: 10,
            step: 1,
            impact: ConfigImpact::Low,
            category: ConfigCategory::CircuitBreaker,
        })]
        circuit_breaker_half_open_requests: u32 = 3,

        // Timeouts
        #[metadata(field_metadata! {
            min: 5,
            max: 120,
            step: 5,
            impact: ConfigImpact::High,
            category: ConfigCategory::Timeouts,
        })]
        request_timeout_secs: u64 = 30,
        #[metadata(field_metadata! {
            min: 1,
            max: 30,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Timeouts,
        })]
        connection_timeout_secs: u64 = 10,

        // Retries
        #[metadata(field_metadata! {
            min: 0,
            max: 10,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Retries,
        })]
        max_retries: u32 = 3,
        #[metadata(field_metadata! {
            min: 50,
            max: 1000,
            step: 50,
            impact: ConfigImpact::Low,
            category: ConfigCategory::Retries,
        })]
        retry_base_delay_ms: u64 = 100,
        #[metadata(field_metadata! {
            min: 1000,
            max: 30000,
            step: 1000,
            impact: ConfigImpact::Low,
            category: ConfigCategory::Retries,
        })]
        retry_max_delay_ms: u64 = 5000,

        // Connection Pooling
        #[metadata(field_metadata! {
            min: 1,
            max: 50,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::ConnectionPooling,
        })]
        pool_connections_per_host: u32 = 10,
        #[metadata(field_metadata! {
            min: 30,
            max: 300,
            step: 30,
            impact: ConfigImpact::Low,
            category: ConfigCategory::ConnectionPooling,
        })]
        pool_idle_timeout_secs: u64 = 90,

        // Stats Collection
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Statistics,
        })]
        stats_enabled: bool = true,
    }
}
