//! Event recording functions for each category.
//!
//! Specialized recording entry points that build typed `Event` structs and
//! submit them through `crate::events::record_safe`. Each function checks its
//! category via `is_category_enabled` before doing any work.

mod entities;
mod flexible;
mod lifecycle;

pub use entities::{
    record_connectivity_event, record_security_event, record_system_event, record_token_event,
    record_wallet_event,
};
pub use flexible::{
    record_api_event, record_filtering_event, record_ohlcv_event, record_rpc_event,
    record_scheduled_task_event, record_trader_event,
};
pub use lifecycle::{
    record_pool_event, record_position_event, record_position_event_flexible, record_swap_event,
    record_transaction_event,
};
