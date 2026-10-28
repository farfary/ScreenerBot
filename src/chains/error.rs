// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Errors returned while constructing or parsing chain identities.

use std::time::Duration;

use crate::chains::access::Capability;
use crate::chains::{ChainId, ExecutionFailure};
use crate::errors::{ErrorClass, Severity};
use crate::i18n::{ids, UiArg, UiText};

/// A validation error for a chain-neutral identity, or an execution failure
/// bubbled up from a chain adapter.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// The supplied chain name is not supported by this build.
    #[error("unsupported chain: {value}")]
    UnsupportedChain { value: String },
    /// An identity value was empty after whitespace was removed.
    #[error("{kind} cannot be empty")]
    EmptyIdentifier { kind: &'static str },
    /// The identity belongs to a different chain than the conversion requested.
    #[error("expected {expected} account, got {actual}")]
    WrongChain { expected: ChainId, actual: ChainId },
    /// The account address is not valid for the identity's chain.
    #[error("invalid {chain} account '{value}'")]
    InvalidAccount { chain: ChainId, value: String },
    /// The account does not exist on the chain.
    #[error("{chain} account '{address}' was not found")]
    AccountNotFound { chain: ChainId, address: String },
    /// Reading or decoding an account from the chain failed.
    #[error("could not read {chain} account '{address}': {detail}")]
    AccountRead {
        chain: ChainId,
        address: String,
        detail: String,
    },
    /// An encrypted key could not be decrypted into a key of the chain.
    #[error("the encrypted {chain} key could not be read: {detail}")]
    InvalidKey { chain: ChainId, detail: String },
    /// No enabled chain accepts the value as an address.
    #[error("'{value}' is not an address on any enabled chain")]
    UnrecognizedAddress { value: String },
    /// More than one enabled chain could be meant, so none is chosen.
    #[error("the chain is ambiguous between {candidates:?}")]
    AmbiguousChain { candidates: Vec<ChainId> },
    /// The chain is supported by this build but not enabled in config.
    #[error("chain {chain} is not enabled")]
    ChainNotEnabled { chain: ChainId },
    /// A settlement fact (signature status, holding, expiry bound) could not be read.
    #[error("could not read settlement state from {chain}: {detail}")]
    SettlementRead { chain: ChainId, detail: String },
    /// The install's chain access mode cannot do this.
    #[error("{chain} access in this mode does not provide {capability}")]
    CapabilityUnavailable {
        chain: ChainId,
        capability: Capability,
    },
    /// Service access with no signed-in account.
    #[error("{chain} service access needs a signed-in account")]
    ServiceSignInRequired { chain: ChainId },
    /// Service access on a device whose grant lacks a required scope.
    #[error("{chain} service access needs the '{scope}' scope on this device")]
    ServiceScopeMissing { chain: ChainId, scope: &'static str },
    /// The service asked to retry later.
    #[error("the {chain} service is busy; retry after {}s", retry_after.as_secs())]
    ServiceBusy {
        chain: ChainId,
        retry_after: Duration,
    },
    /// The account reached its daily allowance on the service.
    #[error("the {chain} service daily limit is reached; retry after {}s", retry_after.as_secs())]
    ServiceDailyLimit {
        chain: ChainId,
        retry_after: Duration,
    },
    /// The service no longer serves this app version.
    #[error("the {chain} service requires app version {minimum} or newer")]
    ServiceVersionUnsupported { chain: ChainId, minimum: String },
    /// The service refused the request itself (malformed or out of bounds).
    #[error("the {chain} service refused the request: {code}")]
    ServiceRequestRefused { chain: ChainId, code: String },
    /// The service did not answer.
    #[error("the {chain} service did not answer: {detail}")]
    ServiceUnreachable { chain: ChainId, detail: String },
    /// The service was not contacted: the network is offline or service
    /// access is off, so nothing was dispatched.
    #[error("the {chain} service was not contacted: the network is offline")]
    ServiceNotContacted { chain: ChainId },
    /// A wallet-scoped read for an address other than the trading wallet.
    #[error("{chain} service reads cover the trading wallet only, not '{address}'")]
    NotTradingWallet { chain: ChainId, address: String },
    /// An on-chain execution attempt failed.
    #[error(transparent)]
    Execution(#[from] ExecutionFailure),
}

impl Error {
    /// True for a refusal by the chain access layer itself: the request was
    /// never handed to a node, so nothing it carried can have reached the
    /// chain.
    pub fn is_access_refusal(&self) -> bool {
        matches!(
            self,
            Error::CapabilityUnavailable { .. }
                | Error::ServiceSignInRequired { .. }
                | Error::ServiceScopeMissing { .. }
                | Error::ServiceBusy { .. }
                | Error::ServiceDailyLimit { .. }
                | Error::ServiceVersionUnsupported { .. }
                | Error::ServiceRequestRefused { .. }
                | Error::ServiceNotContacted { .. }
                | Error::NotTradingWallet { .. }
        )
    }

    /// The catalog text a surface shows for a chain access refusal. Any other
    /// chain error reads as a failed chain access request.
    pub fn access_ui_text(&self) -> UiText {
        match self {
            Error::CapabilityUnavailable { .. } => {
                UiText::new(ids::ERRORS_CHAIN_CAPABILITY_UNAVAILABLE)
            }
            Error::ServiceSignInRequired { .. } => {
                UiText::new(ids::ERRORS_CHAIN_SERVICE_SIGNIN_REQUIRED)
            }
            Error::ServiceScopeMissing { .. } => {
                UiText::new(ids::ERRORS_CHAIN_SERVICE_SCOPE_MISSING)
            }
            Error::ServiceBusy { .. } => UiText::new(ids::ERRORS_CHAIN_SERVICE_BUSY),
            Error::ServiceDailyLimit { .. } => UiText::new(ids::ERRORS_CHAIN_SERVICE_DAILY_LIMIT),
            Error::ServiceVersionUnsupported { minimum, .. } => {
                UiText::new(ids::ERRORS_CHAIN_SERVICE_VERSION)
                    .arg("minimum", UiArg::Text(minimum.clone()))
            }
            Error::ServiceRequestRefused { .. } => UiText::new(ids::ERRORS_CHAIN_SERVICE_REFUSED),
            Error::ServiceUnreachable { .. } | Error::ServiceNotContacted { .. } => {
                UiText::new(ids::ERRORS_CHAIN_SERVICE_UNREACHABLE)
            }
            Error::NotTradingWallet { .. } => UiText::new(ids::ERRORS_CHAIN_NOT_TRADING_WALLET),
            _ => UiText::new(ids::ERRORS_CHAIN_ACCESS_FAILED),
        }
    }
}

/// Result alias for the chains module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Execution(e) => e.is_retryable(),
            Error::AccountRead { .. }
            | Error::SettlementRead { .. }
            | Error::ServiceBusy { .. }
            | Error::ServiceDailyLimit { .. }
            | Error::ServiceUnreachable { .. }
            | Error::ServiceNotContacted { .. } => true,
            Error::CapabilityUnavailable { .. }
            | Error::ServiceSignInRequired { .. }
            | Error::ServiceScopeMissing { .. }
            | Error::ServiceVersionUnsupported { .. }
            | Error::ServiceRequestRefused { .. }
            | Error::NotTradingWallet { .. }
            | Error::UnsupportedChain { .. }
            | Error::EmptyIdentifier { .. }
            | Error::WrongChain { .. }
            | Error::InvalidAccount { .. }
            | Error::InvalidKey { .. }
            | Error::AccountNotFound { .. }
            | Error::UnrecognizedAddress { .. }
            | Error::AmbiguousChain { .. }
            | Error::ChainNotEnabled { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Execution(e) => e.retry_after(),
            Error::ServiceBusy { retry_after, .. }
            | Error::ServiceDailyLimit { retry_after, .. } => Some(*retry_after),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Execution(e) => e.severity(),
            Error::CapabilityUnavailable { .. }
            | Error::ServiceSignInRequired { .. }
            | Error::ServiceScopeMissing { .. }
            | Error::ServiceBusy { .. }
            | Error::ServiceDailyLimit { .. }
            | Error::ServiceVersionUnsupported { .. }
            | Error::ServiceRequestRefused { .. }
            | Error::ServiceUnreachable { .. }
            | Error::ServiceNotContacted { .. }
            | Error::NotTradingWallet { .. }
            | Error::UnsupportedChain { .. }
            | Error::EmptyIdentifier { .. }
            | Error::WrongChain { .. }
            | Error::InvalidAccount { .. }
            | Error::InvalidKey { .. }
            | Error::AccountNotFound { .. }
            | Error::AccountRead { .. }
            | Error::SettlementRead { .. }
            | Error::UnrecognizedAddress { .. }
            | Error::AmbiguousChain { .. }
            | Error::ChainNotEnabled { .. } => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Execution(e) => e.http_status(),
            Error::AccountNotFound { .. } => 404,
            Error::AccountRead { .. } => 502,
            Error::SettlementRead { .. } => 503,
            Error::ServiceSignInRequired { .. } => 401,
            Error::ServiceScopeMissing { .. } => 403,
            Error::CapabilityUnavailable { .. } | Error::NotTradingWallet { .. } => 409,
            Error::ServiceVersionUnsupported { .. } => 426,
            Error::ServiceDailyLimit { .. } => 429,
            Error::ServiceRequestRefused { .. } | Error::ServiceUnreachable { .. } => 502,
            Error::ServiceBusy { .. } | Error::ServiceNotContacted { .. } => 503,
            Error::UnsupportedChain { .. }
            | Error::EmptyIdentifier { .. }
            | Error::WrongChain { .. }
            | Error::InvalidAccount { .. }
            | Error::InvalidKey { .. }
            | Error::UnrecognizedAddress { .. }
            | Error::AmbiguousChain { .. }
            | Error::ChainNotEnabled { .. } => 400,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every access refusal has its own catalog text; the fallback covers only
    /// errors that are not access refusals.
    #[test]
    fn every_access_refusal_has_its_own_text() {
        let chain = ChainId::Solana;
        let refusals = [
            Error::CapabilityUnavailable {
                chain,
                capability: Capability::WatchOtherWallets,
            },
            Error::ServiceSignInRequired { chain },
            Error::ServiceScopeMissing {
                chain,
                scope: "chain:read",
            },
            Error::ServiceBusy {
                chain,
                retry_after: Duration::from_secs(5),
            },
            Error::ServiceDailyLimit {
                chain,
                retry_after: Duration::from_secs(60),
            },
            Error::ServiceVersionUnsupported {
                chain,
                minimum: "0.3.0".to_owned(),
            },
            Error::ServiceRequestRefused {
                chain,
                code: "invalid".to_owned(),
            },
            Error::ServiceNotContacted { chain },
            Error::NotTradingWallet {
                chain,
                address: "A".to_owned(),
            },
        ];
        for refusal in &refusals {
            assert!(refusal.is_access_refusal(), "{refusal}");
            assert_ne!(
                refusal.access_ui_text().id,
                ids::ERRORS_CHAIN_ACCESS_FAILED.as_str(),
                "{refusal}"
            );
        }
        let unreachable = Error::ServiceUnreachable {
            chain,
            detail: "connection reset".to_owned(),
        };
        assert!(!unreachable.is_access_refusal());
        assert_eq!(
            unreachable.access_ui_text().id,
            ids::ERRORS_CHAIN_SERVICE_UNREACHABLE.as_str()
        );
    }
}
