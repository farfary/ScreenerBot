//! Solana-specific wallet-watch mechanics: turning a decoded Solana transaction
//! into a chain-neutral, subject-relative `ActivityKind`. Chain-neutral watch
//! targets, dedupe policy, persistence and orchestration stay in
//! `crate::wallets::watch` — this module supplies the one Solana-specific step
//! that pipeline calls into.

pub mod classify;
