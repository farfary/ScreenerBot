// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Short-lived credential validation receipts for initialization and wallet replacement.

use super::types::CompleteInitializationRequest;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::chains::access::ChainAccessMode;
use crate::chains::ChainId;
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::wallets::replacement::StagedKey;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::{Error, Result};

const SETUP_VALIDATION_TTL: Duration = Duration::from_secs(5 * 60);

/// Which RPC endpoints a validation tested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RpcSource {
    /// The endpoints the request carried; the working ones are saved.
    Request,
    /// The configured endpoints of a fully set-up app; they are kept unchanged.
    Configured,
}

struct PendingSetupValidation {
    id: String,
    expires_at: Instant,
    snapshot_digest: [u8; 32],
    chain: ChainId,
    access: ChainAccessMode,
    wallet_address: String,
    rpc_source: RpcSource,
    working_rpc_indices: Vec<usize>,
}

struct ValidatedSetup {
    chain: ChainId,
    access: ChainAccessMode,
    wallet_address: String,
    rpc_source: RpcSource,
    working_rpc_indices: Vec<usize>,
}

/// The chain access a validation or its completion is for: the requested one,
/// else own RPC during setup and the install's current mode after it, so a
/// wallet replacement keeps the mode unless it names one.
pub(in crate::webserver::routes) fn requested_access(
    requested: Option<ChainAccessMode>,
    chain: ChainId,
) -> ChainAccessMode {
    requested.unwrap_or_else(|| {
        if crate::global::is_initialization_complete() {
            crate::chains::access::mode(chain)
        } else {
            ChainAccessMode::OwnRpc
        }
    })
}

static PENDING_SETUP_VALIDATION: LazyLock<Mutex<Option<PendingSetupValidation>>> =
    LazyLock::new(|| Mutex::new(None));

pub(in crate::webserver::routes) fn validate_rpc_url_list(urls: &[String]) -> Vec<String> {
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

fn setup_snapshot_digest(
    wallet_private_key: &str,
    rpc_urls: &[String],
    access: ChainAccessMode,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    let access = access.as_config_value();
    hasher.update((access.len() as u64).to_le_bytes());
    hasher.update(access.as_bytes());
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
    access: ChainAccessMode,
    chain: ChainId,
    wallet_address: String,
    rpc_source: RpcSource,
    working_rpc_indices: Vec<usize>,
) -> String {
    let id = Uuid::new_v4().to_string();
    let pending = PendingSetupValidation {
        id: id.clone(),
        expires_at: Instant::now() + SETUP_VALIDATION_TTL,
        snapshot_digest: setup_snapshot_digest(wallet_private_key, rpc_urls, access),
        chain,
        access,
        wallet_address,
        rpc_source,
        working_rpc_indices,
    };
    let mut guard = match PENDING_SETUP_VALIDATION.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = Some(pending);
    id
}

fn consume_setup_validation(request: &CompleteInitializationRequest) -> Result<ValidatedSetup> {
    let mut guard = match PENDING_SETUP_VALIDATION.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let Some(pending) = guard.take() else {
        return Err(Error::InvalidInitialization {
            detail: "Run credential verification again before saving setup".to_owned(),
        });
    };

    if pending.expires_at <= Instant::now() {
        return Err(Error::InvalidInitialization {
            detail: "Credential verification expired. Run it again before saving".to_owned(),
        });
    }
    let access = requested_access(request.access, pending.chain);
    if pending.id != request.validation_id
        || pending.access != access
        || pending.snapshot_digest
            != setup_snapshot_digest(&request.wallet_private_key, &request.rpc_urls, access)
    {
        return Err(Error::InvalidInitialization {
            detail: "Credentials changed after verification. Run verification again".to_owned(),
        });
    }
    // Service access saves no endpoint; own RPC needs at least one that worked.
    if pending.access == ChainAccessMode::OwnRpc && pending.working_rpc_indices.is_empty() {
        return Err(Error::InvalidInitialization {
            detail: "No validated RPC endpoints are available to save".to_owned(),
        });
    }

    Ok(ValidatedSetup {
        chain: pending.chain,
        access: pending.access,
        wallet_address: pending.wallet_address,
        rpc_source: pending.rpc_source,
        working_rpc_indices: pending.working_rpc_indices,
    })
}

/// Consume the validation receipt of `request` and encrypt its key: the trading key the
/// validation accepted, with the working endpoints of the request, or none when the
/// validation tested the configured endpoints and keeps them.
pub(in crate::webserver::routes) fn consume_validated_key(
    request: &CompleteInitializationRequest,
) -> std::result::Result<StagedKey, ApiError> {
    let validated = consume_setup_validation(request).map_err(|message| {
        ApiError::new(
            ApiErrorCode::SetupValidationRequired,
            ids::ERRORS_INITIALIZATION_VALIDATION_REQUIRED,
        )
        .details(message.to_string())
    })?;
    let encrypted = crate::secure_storage::encrypt_private_key(&request.wallet_private_key)
        .map_err(|e| {
            ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_INITIALIZATION_ENCRYPT_FAILED,
            )
            .details(e.to_string())
        })?;
    let rpc_urls: Vec<String> = match validated.rpc_source {
        RpcSource::Request => validated
            .working_rpc_indices
            .iter()
            .filter_map(|index| request.rpc_urls.get(*index).cloned())
            .collect(),
        RpcSource::Configured => Vec::new(),
    };
    logger::info(
        LogTag::Webserver,
        &format!(
            "Using validated setup snapshot: {} working endpoint(s), source {:?}",
            validated.working_rpc_indices.len(),
            validated.rpc_source
        ),
    );
    Ok(StagedKey {
        chain: validated.chain,
        access: validated.access,
        address: validated.wallet_address,
        ciphertext: encrypted.ciphertext,
        nonce: encrypted.nonce,
        rpc_urls,
    })
}
