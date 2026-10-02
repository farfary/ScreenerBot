// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Per-chain enablement. The config macro has no map sections, so this is one
//! fixed field per `ChainId` variant the build supports — adding a chain
//! variant makes the compiler demand its toggle here.

use crate::config::metadata::ConfigCategory;
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Per-chain enablement, read once at boot.
    pub struct ChainsConfig {
        /// Whether chains flagged preview-availability may be enabled and
        /// shown. Read at boot; the chain UI that reads it arrives later.
        #[metadata(field_metadata! {
            category: ConfigCategory::Debug,
            restart_required: true
        })]
        show_preview: bool = false,

        /// Solana enablement. Read once at boot; a change applies after a
        /// process restart (the enabled set is frozen in the chain registry).
        #[metadata(field_metadata! {
            category: ConfigCategory::General,
            restart_required: true
        })]
        solana: ChainToggleConfig = ChainToggleConfig::default(),
    }
}

config_struct! {
    /// Boot-time enablement for one chain.
    pub struct ChainToggleConfig {
        /// Whether this chain runs in this process.
        #[metadata(field_metadata! {
            category: ConfigCategory::General,
            restart_required: true
        })]
        enabled: bool = true,
    }
}

impl ChainsConfig {
    /// True when at least one chain this build supports is enabled. A config
    /// that enables nothing cannot boot — the process chain would not exist —
    /// and is refused at load, on reload and on save.
    pub fn has_enabled_chain(&self) -> bool {
        self.solana.enabled
    }
}
