/// Webserver Dashboard Module
///
/// Production-ready web dashboard for ScreenerBot providing:
/// - REST API for querying system state and trading data
/// - WebSocket connections for real-time updates
/// - Service monitoring and management interface
///
/// See docs/websocket-hub-architecture.md for WebSocket architecture
pub mod routes;
pub mod server;
pub mod state;
pub mod templates;
pub mod utils;
pub mod ws;

// Public API for starting/stopping the webserver
pub use server::{shutdown, start_server};
