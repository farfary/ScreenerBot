/// Webserver Dashboard Module
///
/// Production-ready web dashboard for ScreenerBot providing:
/// - REST API for querying system state and trading data
/// - WebSocket connections for real-time updates
/// - Structured monitoring and management interface
///
/// Architecture:
/// - Phase 1: System status monitoring (current)
/// - Phase 2: Data access (positions, tokens, transactions)
/// - Phase 3: Trading operations and analytics
///
/// See docs/webserver-dashboard-architecture.md for full design

// Core modules

pub mod config;
pub mod server;
pub mod state;
pub mod templates;
pub mod utils;

// Route handlers (organized by feature area)
pub mod routes; // Routes module with create_router function

// Middleware stack
pub mod middleware {
    // pub mod auth;      // Authentication & authorization
    // pub mod cors;      // CORS configuration
    // pub mod logging;   // Request/response logging
    // pub mod ratelimit; // Rate limiting
}

// WebSocket support
pub mod websocket {
    // pub mod broadcaster; // Event broadcasting
    // pub mod channels;    // Subscription channels
    // pub mod handler;     // Connection handling
}

// Data models
pub mod models {
    pub mod events; // WebSocket event types
    pub mod requests; // API request types
    pub mod responses; // API response types
}

// Public API for starting/stopping the webserver
pub use server::{ shutdown, start_server };
