use anyhow::Result;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RpcEndpoint {
    pub url: String,
    pub weight: u32,
    pub healthy: bool,
    pub last_health_check: std::time::Instant,
    pub response_time_ms: u64,
    pub error_count: u32,
    pub success_count: u32,
}

impl RpcEndpoint {
    pub fn new(url: String, weight: u32) -> Self {
        Self {
            url,
            weight,
            healthy: true,
            last_health_check: std::time::Instant::now(),
            response_time_ms: 0,
            error_count: 0,
            success_count: 0,
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.error_count;
        if total == 0 {
            1.0
        } else {
            (self.success_count as f64) / (total as f64)
        }
    }

    pub fn record_success(&mut self, response_time_ms: u64) {
        self.success_count += 1;
        self.response_time_ms = response_time_ms;
        self.healthy = true;
    }

    pub fn record_error(&mut self) {
        self.error_count += 1;
        if self.error_count > 5 {
            self.healthy = false;
        }
    }

    pub fn update_health_check(&mut self) {
        self.last_health_check = std::time::Instant::now();
    }
}

#[derive(Debug, Clone)]
pub struct RpcStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: u64,
    pub current_endpoint_index: usize,
}

impl RpcStats {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time_ms: 0,
            current_endpoint_index: 0,
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            1.0
        } else {
            (self.successful_requests as f64) / (self.total_requests as f64)
        }
    }
}

#[derive(Debug)]
pub enum RpcError {
    ConnectionFailed(String),
    Timeout,
    InvalidResponse(String),
    AllEndpointsFailed,
    ConfigurationError(String),
    RequestFailed(String),
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            RpcError::Timeout => write!(f, "Request timeout"),
            RpcError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            RpcError::AllEndpointsFailed => write!(f, "All RPC endpoints failed"),
            RpcError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            RpcError::RequestFailed(msg) => write!(f, "Request failed: {}", msg),
        }
    }
}

impl std::error::Error for RpcError {}

pub type RpcResult<T> = Result<T, RpcError>;

#[derive(Debug, Clone)]
pub struct TransactionConfig {
    pub compute_unit_limit: Option<u32>,
    pub compute_unit_price: Option<u64>,
    pub skip_preflight: bool,
    pub max_retries: Option<usize>,
    pub commitment: CommitmentConfig,
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            compute_unit_limit: None,
            compute_unit_price: None,
            skip_preflight: false,
            max_retries: Some(3),
            commitment: CommitmentConfig::confirmed(),
        }
    }
}
