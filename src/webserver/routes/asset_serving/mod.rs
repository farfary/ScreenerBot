//! Asset serving route — serves static files (CSS, JS, images) from embedded resources.

mod handlers;

// Re-export handlers so they're available when `use asset_serving::*;` is used
pub use handlers::{get_core_script, get_page_script, get_ui_script, get_asset, get_provider_logo, get_font};
