//! Service lifecycle errors (ServiceManager, startup/shutdown, dependencies).
//!
//! Keep errors `Clone` by storing messages as strings.

#[derive(Debug, Clone, thiserror::Error)]
pub enum ServiceError {
    #[error("service init failed (service={service}): {message}")]
    Initialize { service: String, message: String },
    #[error("service start failed (service={service}): {message}")]
    Start { service: String, message: String },
    #[error("service stop failed (service={service}): {message}")]
    Stop { service: String, message: String },
    #[error("service dependency error (service={service}, dep={dependency}): {message}")]
    Dependency {
        service: String,
        dependency: String,
        message: String,
    },
    #[error("{message}")]
    Generic { message: String },
}
