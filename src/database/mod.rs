//! Database initialization and connection pooling for SQLite storage.
//
// All SQLite connections must use `configure::configure_connection()` via
// `with_init()` to ensure PRAGMAs survive connection pool recycling.

pub mod configure;
pub mod maintenance;

pub use configure::*;
pub use maintenance::start_maintenance_task as start_db_maintenance_task;
