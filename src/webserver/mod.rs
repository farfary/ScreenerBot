/// Webserver Dashboard Module
///
/// Production-ready web dashboard for ScreenerBot providing:
/// - REST API for querying system state and trading data
/// - WebSocket connections for real-time updates
/// - Service monitoring and management interface
///
/// See docs/webserver-dashboard-architecture.md for full design

pub mod config;
pub mod server;
pub mod state;
pub mod templates;
pub mod utils;
pub mod routes;

pub mod models {
    pub mod events;
    pub mod requests;
    pub mod responses;
}

// Public API for starting/stopping the webserver
pub use server::{ shutdown, start_server };
