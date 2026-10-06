// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The chain-neutral settlement contract: what the chain says about a submitted signature and about a wallet's holding of an asset.

use crate::chains::{RawAmount, Result};

/// What the chain proves about one submitted signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureVerdict {
    /// The transaction executed and is confirmed.
    Landed,
    /// The transaction is confirmed and its execution failed; it moved no assets.
    FailedOnChain,
    /// Not decided yet: unconfirmed, unseen within its validity window, or unreadable.
    Pending,
    /// The validity window has passed and the chain holds no transaction for the signature.
    NotLanded,
}

/// One signature to settle, with the bound after which it can no longer land.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureCheck {
    pub signature: String,
    /// The bound reported by [`SettlementReader::expiry_bound`] at or after submission.
    /// Without one, an unseen signature stays [`SignatureVerdict::Pending`].
    pub expiry_bound: Option<u64>,
}

/// A wallet's holding of one asset, summed over every account that holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Holding {
    pub amount: RawAmount,
    /// True when any account holding the asset is frozen, so the holding cannot be moved.
    pub frozen: bool,
}

/// Reads settlement facts from one chain.
#[async_trait::async_trait]
pub trait SettlementReader: Send + Sync {
    /// One verdict per check, in order; one batched status read.
    async fn signature_verdicts(&self, checks: &[SignatureCheck]) -> Result<Vec<SignatureVerdict>>;
    /// The wallet's holding of `asset`, summed over its token accounts.
    async fn holding(&self, owner: &str, asset: &str) -> Result<Holding>;
    /// A bound no transaction submitted before now can still land after.
    async fn expiry_bound(&self) -> Result<u64>;
}
