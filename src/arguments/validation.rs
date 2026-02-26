//! Webserver argument validation and port/host utilities.

use super::get_arg_value;

/// Get the port override from CLI (overrides config file).
pub fn get_port_override() -> Option<u16> {
    get_arg_value("--port").and_then(|v| v.parse::<u16>().ok().filter(|&port| port > 0))
}

/// Get the host override from CLI (overrides config file).
pub fn get_host_override() -> Option<String> {
    get_arg_value("--host").filter(|h| !h.trim().is_empty())
}

/// Validates port argument provided via CLI.
pub fn validate_port_argument() -> Result<(), String> {
    if let Some(port_str) = get_arg_value("--port") {
        match port_str.parse::<u16>() {
            Ok(0) => {
                return Err("Invalid port value '0': Port must be between 1 and 65535".to_owned());
            }
            Ok(_) => Ok(()),
            Err(_) => {
                return Err(format!(
                    "Invalid port value '{}': Port must be a number between 1 and 65535",
                    port_str
                ));
            }
        }
    } else {
        Ok(())
    }
}

/// Validates host argument provided via CLI.
pub fn validate_host_argument() -> Result<(), String> {
    if let Some(host) = get_arg_value("--host") {
        if host.trim().is_empty() {
            return Err("Invalid host value: Host cannot be empty".to_owned());
        }
        Ok(())
    } else {
        Ok(())
    }
}

/// Validates port value is in acceptable range (1-65535).
pub fn is_valid_port(port: u16) -> bool {
    port > 0
}

/// Checks if port is privileged (<1024) which may require elevated permissions.
pub fn is_privileged_port(port: u16) -> bool {
    port > 0 && port < 1024
}
