use once_cell::sync::OnceCell;
use tokio::sync::broadcast;

use crate::tokens::{summary::TokenSummary, summary_cache};

const TOKEN_UPDATES_CAPACITY: usize = 2048;

static TOKEN_UPDATES_TX: OnceCell<broadcast::Sender<TokenRealtimeEvent>> = OnceCell::new();

#[derive(Debug, Clone)]
pub enum TokenRealtimeEvent {
    Summary(TokenSummary),
    Removed(String),
}

fn get_broadcaster() -> &'static broadcast::Sender<TokenRealtimeEvent> {
    TOKEN_UPDATES_TX.get_or_init(|| {
        let (tx, _rx) = broadcast::channel(TOKEN_UPDATES_CAPACITY);
        tx
    })
}

pub fn emit_token_summary(summary: TokenSummary) {
    summary_cache::store(summary.clone());
    let _ = get_broadcaster().send(TokenRealtimeEvent::Summary(summary));
}

pub fn emit_token_removed(mint: String) {
    summary_cache::remove(&mint);
    let _ = get_broadcaster().send(TokenRealtimeEvent::Removed(mint));
}

pub fn subscribe_token_updates() -> broadcast::Receiver<TokenRealtimeEvent> {
    get_broadcaster().subscribe()
}
