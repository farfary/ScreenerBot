use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct StartupServiceStatus {
    pub name: &'static str,
    pub ready: bool,
    pub started_at: Option<DateTime<Utc>>,
    pub ready_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub message: Option<String>,
}

#[derive(Default)]
struct InternalStatus {
    started_at: Option<DateTime<Utc>>,
    ready_at: Option<DateTime<Utc>>,
    duration_ms: Option<u64>,
    ready: bool,
    message: Option<String>,
    start_instant: Option<Instant>,
}

static TRACKER: Lazy<Mutex<HashMap<&'static str, InternalStatus>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn tracker<'a>() -> MutexGuard<'a, HashMap<&'static str, InternalStatus>> {
    TRACKER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn mark_service_start(name: &'static str) {
    let mut tracker = tracker();
    let entry = tracker.entry(name).or_insert_with(InternalStatus::default);
    entry.started_at = Some(Utc::now());
    entry.start_instant = Some(Instant::now());
    entry.ready = false;
    entry.ready_at = None;
    entry.duration_ms = None;
}

pub fn mark_service_ready(name: &'static str) {
    let mut tracker = tracker();
    let entry = tracker.entry(name).or_insert_with(InternalStatus::default);
    if entry.ready {
        return;
    }

    entry.ready = true;
    entry.ready_at = Some(Utc::now());

    if entry.duration_ms.is_none() {
        if let Some(start) = entry.start_instant.take() {
            entry.duration_ms = Some(start.elapsed().as_millis() as u64);
        }
    }
}

pub fn set_service_message(name: &'static str, message: impl Into<String>) {
    let mut tracker = tracker();
    let entry = tracker.entry(name).or_insert_with(InternalStatus::default);
    entry.message = Some(message.into());
}

pub fn clear_service_message(name: &'static str) {
    let mut tracker = tracker();
    if let Some(entry) = tracker.get_mut(name) {
        entry.message = None;
    }
}

pub fn get_status(name: &'static str) -> Option<StartupServiceStatus> {
    let tracker = tracker();
    tracker.get(name).map(|status| StartupServiceStatus {
        name,
        ready: status.ready,
        started_at: status.started_at.clone(),
        ready_at: status.ready_at.clone(),
        duration_ms: status.duration_ms,
        message: status.message.clone(),
    })
}

pub fn snapshot() -> Vec<StartupServiceStatus> {
    let tracker = tracker();
    let mut statuses: Vec<_> = tracker
        .iter()
        .map(|(&name, status)| StartupServiceStatus {
            name,
            ready: status.ready,
            started_at: status.started_at.clone(),
            ready_at: status.ready_at.clone(),
            duration_ms: status.duration_ms,
            message: status.message.clone(),
        })
        .collect();

    statuses.sort_by(|a, b| a.name.cmp(b.name));
    statuses
}
