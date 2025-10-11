pub mod routes;
pub mod server;
pub mod state;
pub mod status_snapshot;
pub mod templates;
pub mod utils;

// Public API for starting/stopping the webserver
pub use server::{shutdown, start_server};
