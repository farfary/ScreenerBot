// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Dashboard assistant configuration: interactive chat plus scheduled
//! conversations. Provider credentials live in `llm`; tool permissions live
//! in `agent_control`.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Dashboard chat assistant and scheduled automation.
    pub struct AssistantConfig {
        /// Enable the assistant chat interface.
        #[metadata(field_metadata! {
            category: ConfigCategory::Chat,
        })]
        enabled: bool = false,

        /// Maximum messages per chat session before auto-summarization.
        #[metadata(field_metadata! {
            min: 10,
            max: 500,
            step: 10,
            category: ConfigCategory::Chat,
        })]
        max_session_messages: u32 = 100,

        /// Summarize and compress long conversations automatically.
        #[metadata(field_metadata! {
            category: ConfigCategory::Chat,
        })]
        auto_summarize: bool = true,

        /// Allow the assistant to react to events.
        #[metadata(field_metadata! {
            category: ConfigCategory::Automation,
        })]
        event_triggers_enabled: bool = false,

        /// Run assistant tasks on a schedule.
        #[metadata(field_metadata! {
            category: ConfigCategory::Automation,
            impact: ConfigImpact::Medium,
        })]
        scheduled_tasks_enabled: bool = false,

        /// How often the scheduler checks for due tasks.
        #[metadata(field_metadata! {
            min: 10,
            max: 300,
            step: 5,
            category: ConfigCategory::Automation,
        })]
        check_interval_seconds: u64 = 30,

        /// Maximum concurrent scheduled task executions.
        #[metadata(field_metadata! {
            min: 1,
            max: 5,
            step: 1,
            category: ConfigCategory::Automation,
        })]
        max_concurrent: u32 = 1,

        /// Default timeout for a scheduled task execution.
        #[metadata(field_metadata! {
            min: 30,
            max: 600,
            step: 30,
            category: ConfigCategory::Automation,
        })]
        default_timeout_seconds: u64 = 120,
    }
}
