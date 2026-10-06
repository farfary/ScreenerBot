// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana pool suite: recorded pool cases decoded through the pricing path.
//!
//! The cases live in `tests/fixtures/pools/solana/<slug>/`, one directory per
//! `ProgramKind::protocol_slug()`, and are read through `tests/common/pool_cases.rs`.
//! A case has a `purpose`: `direct-swap` cases are the accounts the swap engine reads, and
//! `price-snapshot` cases are the account bundle the price fetcher hands a decoder. Accounts the
//! swap cases share with a price case carry the price case's lamports; the others carry `0`.
//!
//! * `accounts` — every `direct-swap` case holds exactly the accounts the swap engine's
//!   production load path reads.
//! * `snapshot` — bit-exact decoded prices against `prices-snapshot.json`, and the account-set
//!   and refusal checks that run on the same cases.
//!
//! ```text
//! cargo nextest run -E 'binary(solana_pools)'
//! ```

mod accounts;
#[path = "../common/mod.rs"]
mod common;
mod snapshot;
