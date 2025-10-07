/// Webserver Dashboard Module
///
/// Production-ready web dashboard for ScreenerBot providing:
/// - REST API for querying system state and trading data
/// - WebSocket connections for real-time updates
/// - Service monitoring and management interface
///
/// See docs/webserver-dashboard-architecture.md for full design
pub mod routes;
pub mod server;
pub mod state;
pub mod status_broadcast;
pub mod templates;
pub mod utils;

// Public API for starting/stopping the webserver
pub use server::{ shutdown, start_server };
pub use status_broadcast::{
    get_subscriber_count as get_status_subscriber_count,
    initialize_status_broadcaster,
    start_status_broadcaster,
    subscribe as subscribe_status,
    StatusSnapshot,
};
