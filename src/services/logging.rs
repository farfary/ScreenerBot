//! Service lifecycle logging helpers.

use super::ServiceLogEvent;
use crate::logger::{self, LogTag};

fn should_log_service_details(always: bool) -> bool {
    always
}

fn append_details(message: &mut String, details: Option<&str>) {
    if let Some(extra) = details {
        let trimmed = extra.trim();
        if !trimmed.is_empty() {
            message.push(' ');
            message.push_str(trimmed);
        }
    }
}

pub fn log_service_event(
    service_name: &str,
    event: ServiceLogEvent,
    details: Option<&str>,
    always: bool,
) {
    if !should_log_service_details(always) {
        return;
    }

    let mut message = format!("service={} event={}", service_name, event.label());
    append_details(&mut message, details);

    // Map dynamic level string to logger methods
    match event.level() {
        "DEBUG" => logger::debug(LogTag::System, &message),
        "SUCCESS" | "INFO" => logger::info(LogTag::System, &message),
        "WARN" | "WARNING" => logger::warning(LogTag::System, &message),
        "ERROR" => logger::error(LogTag::System, &message),
        _ => logger::info(LogTag::System, &message),
    }
}

pub fn log_service_notice(service_name: &str, kind: &str, details: Option<&str>, always: bool) {
    if !should_log_service_details(always) {
        return;
    }

    let mut message = format!("service_notice service={service_name} kind={kind}");
    append_details(&mut message, details);

    logger::info(LogTag::System, &message);
}

pub fn log_service_startup_phase(phase: &str, details: Option<&str>) {
    let mut message = format!("service_startup phase={phase}");
    append_details(&mut message, details);
    logger::info(LogTag::System, &message);
}
