/// Service lifecycle errors (ServiceManager, startup/shutdown, dependencies).
///
/// Keep errors `Clone` by storing messages as strings.

#[derive(Debug, Clone)]
pub enum ServiceError {
    Initialize { service: String, message: String },
    Start { service: String, message: String },
    Stop { service: String, message: String },
    Dependency { service: String, dependency: String, message: String },
    Generic { message: String },
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::Initialize { service, message } => {
                write!(f, "Service init failed (service={service}): {message}")
            }
            ServiceError::Start { service, message } => {
                write!(f, "Service start failed (service={service}): {message}")
            }
            ServiceError::Stop { service, message } => {
                write!(f, "Service stop failed (service={service}): {message}")
            }
            ServiceError::Dependency {
                service,
                dependency,
                message,
            } => {
                write!(
                    f,
                    "Service dependency error (service={service}, dep={dependency}): {message}"
                )
            }
            ServiceError::Generic { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ServiceError {}

