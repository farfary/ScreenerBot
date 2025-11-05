mod server;

pub mod middleware;
pub mod routes;
pub mod snapshot;
pub mod state;
pub mod templates;
pub mod utils;

// Public API for starting/stopping the webserver
pub use server::{shutdown, start_server};

// Crate-visible defaults for service logging and tests
pub(crate) use server::{DEFAULT_HOST, DEFAULT_PORT};
