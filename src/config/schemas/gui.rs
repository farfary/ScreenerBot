// GUI configuration schema

use crate::config_struct;

config_struct! {
    /// GUI/Desktop application configuration
    pub struct GuiConfig {
        /// Zoom level for the Tauri webview (0.5 = 50%, 1.0 = 100%, 3.0 = 300%)
        zoom_level: f64 = 1.0,
    }
}
