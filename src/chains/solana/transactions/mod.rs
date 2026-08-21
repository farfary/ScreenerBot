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
