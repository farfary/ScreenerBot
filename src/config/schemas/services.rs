use crate::config_struct;

// ============================================================================
// SERVICES CONFIGURATION
// ============================================================================

config_struct! {
    /// Individual service configuration
    pub struct ServiceConfig {
        enabled: bool = true,
        priority: i32 = 50,
    }
}

config_struct! {
    /// Services configuration
    pub struct ServicesConfig {
        events: ServiceConfig = ServiceConfig { enabled: true, priority: 10 },
        blacklist: ServiceConfig = ServiceConfig { enabled: true, priority: 15 },
        tokens: ServiceConfig = ServiceConfig { enabled: true, priority: 20 },
        positions: ServiceConfig = ServiceConfig { enabled: true, priority: 50 },
        pools: ServiceConfig = ServiceConfig { enabled: true, priority: 30 },
        trader: ServiceConfig = ServiceConfig { enabled: true, priority: 100 },
    }
}
