/// Centralized WebSocket Hub Module
///
/// This module implements a unified WebSocket architecture that multiplexes
/// multiple topics over a single persistent connection per client.
///
/// ## Architecture
/// - Single /ws endpoint for all real-time data
/// - Topic-based pub/sub with server-side filtering
/// - Per-connection queues with backpressure handling
/// - Snapshot + delta model with sequence tracking
/// - Graceful degradation (channel failures don't break connection)
///
/// ## Key Components
/// - `hub`: Central broker with routing and backpressure
/// - `connection`: WebSocket upgrade and lifecycle management
/// - `message`: Standard envelope and control message schemas
/// - `filters`: Per-topic filter definitions
/// - `topics`: Typed message constructors per domain
/// - `producers`: Internal broadcast → topic message adapters
/// - `health`: Heartbeat and connection health tracking
/// - `metrics`: Per-connection stats for monitoring
pub mod connection;
pub mod filters;
pub mod health;
pub mod hub;
pub mod message;
pub mod metrics;
pub mod producers;
pub mod snapshots;
pub mod topics;

pub use hub::WsHub;
pub use message::{ServerMessage, Topic, WsEnvelope};
pub use snapshots::{RpcStatsSnapshot, StatusSnapshot};
