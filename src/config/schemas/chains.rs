// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Per-chain settings. The config macro has no map sections, so this is one
//! fixed field per `ChainId` variant the build supports; adding a variant makes
//! the compiler demand its settings here. Chain-indexed reads live in
//! `crate::chains::config`.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config::schemas::{RpcConfig, SolanaEndpointMonitorsConfig, SwapsConfig};
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Per-chain settings.
    pub struct ChainsConfig {
        /// Whether chains flagged preview-availability may be enabled and
        /// shown. Read at boot; the chain UI that reads it arrives later.
        #[metadata(field_metadata! {
            category: ConfigCategory::Debug,
            restart_required: true
        })]
        show_preview: bool = false,

        /// Solana: enablement, RPC transport, swap routers and the swap-API
        /// health monitors.
        #[metadata(field_metadata! {
            category: ConfigCategory::General,
        })]
        solana: SolanaChainConfig = SolanaChainConfig::default(),
    }
}

config_struct! {
    /// Settings that exist only for Solana.
    pub struct SolanaChainConfig {
        /// Whether Solana runs in this process. Read once at boot (the enabled
        /// set is frozen in the chain registry).
        #[metadata(field_metadata! {
            category: ConfigCategory::General,
            restart_required: true
        })]
        enabled: bool = true,

        /// RPC endpoints and transport tuning. Read when the RPC manager is
        /// built at boot.
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Endpoints,
            restart_required: true
        })]
        rpc: RpcConfig = RpcConfig::default(),

        /// Swap routers and the execution cost guard. Read per swap.
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Routers,
        })]
        swaps: SwapsConfig = SwapsConfig::default(),

        /// Health monitors for Solana's swap APIs. Internal tuning with no
        /// Settings control.
        #[metadata(field_metadata! {
            hidden: true
        })]
        connectivity: SolanaEndpointMonitorsConfig = SolanaEndpointMonitorsConfig::default(),
    }
}

impl ChainsConfig {
    /// True when at least one chain this build supports is enabled. A config
    /// that enables nothing cannot boot and is refused at load, on reload and
    /// on save.
    pub fn has_enabled_chain(&self) -> bool {
        self.solana.enabled
    }
}
