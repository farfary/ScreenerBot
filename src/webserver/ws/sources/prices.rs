//! Removed legacy price bridge.
//!
//! The dedicated prices WebSocket topic was retired in realtime cleanup
//! phase 2; price deltas now flow exclusively through `tokens.update`
//! snapshots. This file remains as an empty stub so downstream tooling that
//! referenced the old module path fails fast at compile time.
