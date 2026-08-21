//! Solana pool discovery, on-chain account decoding, and protocol recognition.
//!
//! This module owns everything Solana-specific about finding and reading pools:
//! RPC account fetching (`fetcher`), program account discovery (`discovery`),
//! protocol classification and reserve-account extraction (`analyzer`,
//! `analyzer_extractors`, `types::ProgramKind`), and per-DEX byte decoding
//! (`decoders`). Chain-neutral pool persistence, caching, pricing policy and
//! service lifecycle stay in `crate::pools` and consume this module's output
//! (`PoolDescriptor`, `PriceResult`) — see `crate::pools` for that boundary.

pub mod analyzer;
mod analyzer_extractors;
pub mod calculator;
pub mod decode_utils;
pub mod decoders;
pub mod discovery;
pub mod fetcher;
mod fetcher_ops;
mod fetcher_types;
pub mod service;
pub mod types;
