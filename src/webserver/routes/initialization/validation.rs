//! Short-lived credential validation receipts for initialization.

use super::types::CompleteInitializationRequest;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

const SETUP_VALIDATION_TTL: Duration = Duration::from_secs(5 * 60);

struct PendingSetupValidation {
    id: String,
    expires_at: Instant,
    snapshot_digest: [u8; 32],
    wallet_address: String,
    working_rpc_indices: Vec<usize>,
}

pub(super) struct ValidatedSetup {
    pub(super) wallet_address: String,
    pub(super) working_rpc_indices: Vec<usize>,
}

static PENDING_SETUP_VALIDATION: LazyLock<Mutex<Option<PendingSetupValidation>>> =
    LazyLock::new(|| Mutex::new(None));

pub(super) fn validate_rpc_url_list(urls: &[String]) -> Vec<String> {
    let mut errors = Vec::new();
    if urls.is_empty() {
        errors.push("At least one RPC URL is required".to_owned());
        return errors;
    }
    if urls.len() > 10 {
        errors.push("Maximum 10 RPC URLs allowed".to_owned());
        return errors;
    }

    let mut seen = HashSet::new();
    for url in urls {
        let parsed = match url::Url::parse(url) {
            Ok(parsed) if parsed.scheme() == "https" && parsed.host_str().is_some() => parsed,
            _ => {
                errors.push("Every RPC endpoint must be a valid HTTPS URL".to_owned());
                continue;
            }
        };

        if !parsed.username().is_empty() || parsed.password().is_some() {
            errors.push("RPC URLs cannot contain usernames or passwords".to_owned());
        }
        if parsed.fragment().is_some() {
            errors.push("RPC URLs cannot contain fragments".to_owned());
        }

        let host = parsed.host_str().unwrap_or_default().to_lowercase();
        if host == "api.mainnet-beta.solana.com" {
            errors.push(
                "The public Solana RPC is not supported for continuous ScreenerBot polling"
                    .to_owned(),
            );
        }
        if is_private_rpc_host(&host) {
            errors.push("RPC endpoints cannot target local or private network hosts".to_owned());
        }

        let normalized = parsed.as_str().trim_end_matches('/').to_lowercase();
        if !seen.insert(normalized) {
            errors.push("Remove duplicate RPC endpoints".to_owned());
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

fn is_private_rpc_host(host: &str) -> bool {
    if host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
    {
        return true;
    }

    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(address)) => {
            address.is_private()
                || address.is_loopback()
                || address.is_link_local()
                || address.is_unspecified()
        }
        Ok(IpAddr::V6(address)) => {
            address.is_loopback()
                || address.is_unspecified()
                || address.is_unique_local()
                || (address.segments()[0] & 0xffc0) == 0xfe80
        }
        Err(_) => false,
    }
}

fn setup_snapshot_digest(wallet_private_key: &str, rpc_urls: &[String]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((wallet_private_key.len() as u64).to_le_bytes());
    hasher.update(wallet_private_key.as_bytes());
    hasher.update((rpc_urls.len() as u64).to_le_bytes());
    for url in rpc_urls {
        hasher.update((url.len() as u64).to_le_bytes());
        hasher.update(url.as_bytes());
    }
    hasher.finalize().into()
}

pub(super) fn clear_setup_validation() {
    let mut guard = match PENDING_SETUP_VALIDATION.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = None;
}

pub(super) fn store_setup_validation(
    wallet_private_key: &str,
    rpc_urls: &[String],
    wallet_address: String,
    working_rpc_indices: Vec<usize>,
) -> String {
    let id = Uuid::new_v4().to_string();
    let pending = PendingSetupValidation {
        id: id.clone(),
        expires_at: Instant::now() + SETUP_VALIDATION_TTL,
        snapshot_digest: setup_snapshot_digest(wallet_private_key, rpc_urls),
        wallet_address,
        working_rpc_indices,
    };
    let mut guard = match PENDING_SETUP_VALIDATION.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = Some(pending);
    id
}

pub(super) fn consume_setup_validation(
    request: &CompleteInitializationRequest,
) -> Result<ValidatedSetup, String> {
    let mut guard = match PENDING_SETUP_VALIDATION.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let Some(pending) = guard.take() else {
        return Err("Run credential verification again before saving setup".to_owned());
    };

    if pending.expires_at <= Instant::now() {
        return Err("Credential verification expired. Run it again before saving".to_owned());
    }
    if pending.id != request.validation_id
        || pending.snapshot_digest
            != setup_snapshot_digest(&request.wallet_private_key, &request.rpc_urls)
    {
        return Err("Credentials changed after verification. Run verification again".to_owned());
    }
    if pending.working_rpc_indices.is_empty() {
        return Err("No validated RPC endpoints are available to save".to_owned());
    }

    Ok(ValidatedSetup {
        wallet_address: pending.wallet_address,
        working_rpc_indices: pending.working_rpc_indices,
    })
}
