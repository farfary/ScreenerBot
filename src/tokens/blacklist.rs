// tokens/blacklist.rs
// Fast in-memory blacklist with DB persistence hooks (to be wired via storage layer later)

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use crate::tokens::storage::database::Database;
use crate::tokens::storage::operations::{list_blacklist, remove_blacklist, upsert_blacklist};

static BLACKLIST: std::sync::LazyLock<Arc<RwLock<HashSet<String>>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(HashSet::new())));

pub fn is(mint: &str) -> bool {
    BLACKLIST.read().ok().map_or(false, |s| s.contains(mint))
}

pub fn add(mint: &str) -> bool {
    if let Ok(mut s) = BLACKLIST.write() {
        s.insert(mint.to_string())
    } else {
        false
    }
}

pub fn remove(mint: &str) -> bool {
    if let Ok(mut s) = BLACKLIST.write() {
        s.remove(mint)
    } else {
        false
    }
}

pub fn persist_add(db: &Database, mint: &str, reason: Option<&str>) -> Result<(), String> {
    upsert_blacklist(db, mint, reason)
}

pub fn persist_remove(db: &Database, mint: &str) -> Result<(), String> {
    remove_blacklist(db, mint)
}

pub fn hydrate_from_db(db: &Database) -> Result<usize, String> {
    let items = list_blacklist(db)?;
    if let Ok(mut s) = BLACKLIST.write() {
        for (mint, _reason) in items.into_iter() {
            s.insert(mint);
        }
        Ok(s.len())
    } else {
        Err("blacklist cache poisoned".to_string())
    }
}
