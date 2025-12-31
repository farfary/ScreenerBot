mod server;

pub mod demo;
pub mod middleware;
pub mod routes;
pub mod session;
pub mod snapshot;
pub mod state;
pub mod templates;
pub mod totp;
pub mod utils;

// Public API for starting/stopping the webserver
pub use server::{shutdown, start_server, test_port_binding};

// Crate-visible defaults for service logging and tests
pub(crate) use server::{DEFAULT_HOST, DEFAULT_PORT};
