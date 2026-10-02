// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana-specific transaction ingestion and interpretation.
//!
//! Everything here decodes Solana wire types (`chains::solana::rpc::TransactionDetails`,
//! `solana_sdk` account keys, instructions) into the chain-neutral models owned by
//! `crate::transactions` (`Transaction`, `SubjectAssetDelta`, ...). Persistence, service
//! lifecycle, and reporting stay in `crate::transactions` — this module is decode-only.

pub mod analyzer;
pub mod deltas;
pub mod fetcher;
pub mod processor;
pub mod program_ids;
pub mod subject;
