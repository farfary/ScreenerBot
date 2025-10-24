/// Log level definitions for structured logging
///
/// Levels are ordered by severity (Error < Warning < Info < Debug < Verbose)
/// This allows filtering by minimum level threshold.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 0,   // Critical errors, always shown
    Warning = 1, // Important issues that need attention
    Info = 2,    // Standard operational messages (default)
    Debug = 3,   // Detailed diagnostic info (gated by --debug-<module>)
    Verbose = 4, // Very detailed trace info (gated by --verbose)
}

impl LogLevel {
    /// Get string representation for logging
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warning => "WARNING",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Verbose => "VERBOSE",
        }
    }

    /// Get emoji representation (optional, for enhanced display)
    pub fn emoji(&self) -> &'static str {
        match self {
            LogLevel::Error => "❌",
            LogLevel::Warning => "⚠️",
            LogLevel::Info => "ℹ️",
            LogLevel::Debug => "🐛",
            LogLevel::Verbose => "🔍",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "ERROR" => Some(LogLevel::Error),
            "WARNING" | "WARN" => Some(LogLevel::Warning),
            "INFO" => Some(LogLevel::Info),
            "DEBUG" => Some(LogLevel::Debug),
            "VERBOSE" | "TRACE" => Some(LogLevel::Verbose),
            _ => None,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
