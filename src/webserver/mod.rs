mod server;

pub mod routes;
pub mod snapshot;
pub mod state;
pub mod templates;
pub mod utils;

// Public API for starting/stopping the webserver
pub use server::{shutdown, start_server};
