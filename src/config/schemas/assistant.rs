//! Dashboard assistant configuration: interactive chat plus scheduled
//! conversations. Provider credentials live in `llm`; tool permissions live
//! in `agent_control`.

use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Dashboard chat assistant and scheduled automation.
    pub struct AssistantConfig {
        /// Enable the assistant chat interface.
        #[metadata(field_metadata! {
            label: "Enable Assistant",
            hint: "Enable the assistant chat interface for interactive conversations",
            category: "Chat",
        })]
        enabled: bool = false,

        /// Maximum messages per chat session before auto-summarization.
        #[metadata(field_metadata! {
            label: "Max Session Messages",
            hint: "Maximum messages per chat session before auto-summarization",
            min: 10,
            max: 500,
            step: 10,
            unit: "messages",
            category: "Chat",
        })]
        max_session_messages: u32 = 100,

        /// Summarize and compress long conversations automatically.
        #[metadata(field_metadata! {
            label: "Auto Summarize",
            hint: "Automatically summarize and compress long conversations to save context",
            category: "Chat",
        })]
        auto_summarize: bool = true,

        /// Allow the assistant to react to events.
        #[metadata(field_metadata! {
            label: "Event Triggers",
            hint: "Allow the assistant to trigger actions based on events (disabled by default for safety)",
            category: "Automation",
        })]
        event_triggers_enabled: bool = false,

        /// Run assistant tasks on a schedule.
        #[metadata(field_metadata! {
            label: "Scheduled Tasks",
            hint: "Enable automated assistant tasks that run on a schedule",
            category: "Automation",
            impact: "medium",
        })]
        scheduled_tasks_enabled: bool = false,

        /// How often the scheduler checks for due tasks.
        #[metadata(field_metadata! {
            label: "Check Interval",
            hint: "How often the scheduler checks for tasks that need to run",
            min: 10,
            max: 300,
            step: 5,
            unit: "seconds",
            category: "Automation",
        })]
        check_interval_seconds: u64 = 30,

        /// Maximum concurrent scheduled task executions.
        #[metadata(field_metadata! {
            label: "Max Concurrent Tasks",
            hint: "Maximum number of scheduled tasks that can run simultaneously",
            min: 1,
            max: 5,
            step: 1,
            category: "Automation",
        })]
        max_concurrent: u32 = 1,

        /// Default timeout for a scheduled task execution.
        #[metadata(field_metadata! {
            label: "Default Timeout",
            hint: "Maximum time a scheduled task can run before being stopped",
            min: 30,
            max: 600,
            step: 30,
            unit: "seconds",
            category: "Automation",
        })]
        default_timeout_seconds: u64 = 120,
    }
}
