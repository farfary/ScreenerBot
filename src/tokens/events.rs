// tokens/events.rs
// Lightweight internal event bus for token updates

use chrono::{DateTime, Utc};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub enum TokenEvent {
    TokenDiscovered {
        mint: String,
        source: String,
        at: DateTime<Utc>,
    },
    TokenUpdated {
        mint: String,
        at: DateTime<Utc>,
    },
    DecimalsUpdated {
        mint: String,
        decimals: u8,
        at: DateTime<Utc>,
    },
    PoolsUpdated {
        mint: String,
        best_pool: Option<String>,
        at: DateTime<Utc>,
    },
    TokenBlacklisted {
        mint: String,
        reason: String,
        at: DateTime<Utc>,
    },
    TokenUnblacklisted {
        mint: String,
        at: DateTime<Utc>,
    },
}

static SUBSCRIBERS: std::sync::LazyLock<RwLock<Vec<Arc<dyn Fn(&TokenEvent) + Send + Sync>>>> =
    std::sync::LazyLock::new(|| RwLock::new(Vec::new()));

pub fn subscribe(cb: Arc<dyn Fn(&TokenEvent) + Send + Sync>) {
    if let Ok(mut subs) = SUBSCRIBERS.write() {
        subs.push(cb);
    }
}

pub fn emit(event: TokenEvent) {
    if let Ok(subs) = SUBSCRIBERS.read() {
        for cb in subs.iter() {
            cb(&event);
        }
    }
}
