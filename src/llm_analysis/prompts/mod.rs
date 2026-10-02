// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Prompt construction, templates and builder for LLM analysis requests.
mod builder;
mod templates;

pub use builder::PromptBuilder;
pub use templates::{
    get_entry_analysis_prompt, get_exit_analysis_prompt, get_filter_prompt,
    get_trailing_stop_prompt,
};
