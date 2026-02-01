mod builder;
mod templates;

pub use builder::PromptBuilder;
pub use templates::{
    get_entry_analysis_prompt, get_exit_analysis_prompt, get_filter_prompt,
    get_trailing_stop_prompt,
};
