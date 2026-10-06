// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Chain-neutral recorded pool cases: the case schema and the loader for `tests/fixtures/pools/`.
//!
//! A case is one recorded pool: the accounts a venue reads, the mints it prices, and the slot
//! they were read at. Cases live at `tests/fixtures/pools/<chain>/<venue>/<case>.json` and are
//! immutable once committed. Every integer is a decimal string, which is safe for `u64` and
//! `u128`. The container is generic over the account payload so another chain adds its own
//! account type and reuses the rest unchanged.
//!
//! This file depends only on `serde`, `serde_json` and `base64`, so a crate-internal test can
//! include it with `#[path]` without importing the library under test.

use base64::Engine;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The case schema version this loader reads.
pub const SCHEMA_VERSION: u32 = 1;

/// One recorded pool.
#[derive(Debug, Clone, Deserialize)]
pub struct RecordedCase<A> {
    pub schema: u32,
    pub chain: String,
    pub venue: String,
    pub program: String,
    pub pool: String,
    pub target_mint: String,
    pub quote_mint: String,
    #[serde(deserialize_with = "decimal_u64")]
    pub slot: u64,
    pub purpose: String,
    /// Mint decimals the pricing path reads from the token cache rather than from an account.
    #[serde(default)]
    pub decimals: BTreeMap<String, u8>,
    pub accounts: BTreeMap<String, A>,
}

/// The account payload of a Solana case.
#[derive(Debug, Clone, Deserialize)]
pub struct SolanaAccount {
    pub owner: String,
    #[serde(deserialize_with = "decimal_u64")]
    pub lamports: u64,
    #[serde(deserialize_with = "base64_bytes")]
    pub data: Vec<u8>,
}

fn decimal_u64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let text = String::deserialize(deserializer)?;
    text.parse().map_err(serde::de::Error::custom)
}

fn base64_bytes<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    let text = String::deserialize(deserializer)?;
    base64::engine::general_purpose::STANDARD
        .decode(text)
        .map_err(serde::de::Error::custom)
}

/// File stems in a venue directory that are not recorded cases: the vendored layout spec and its
/// provenance record.
const VENUE_METADATA_STEMS: [&str; 2] = ["spec", "spec-source"];

/// The directory holding every recorded case and fixture of the pool suites.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("pools")
}

/// Load one case by chain, venue slug and case name (the file stem).
pub fn load_case<A: DeserializeOwned>(chain: &str, venue: &str, name: &str) -> RecordedCase<A> {
    let path = fixtures_dir()
        .join(chain)
        .join(venue)
        .join(format!("{name}.json"));
    read_case(&path)
}

/// Every case of one venue as `(case name, case)`, sorted by case name.
pub fn load_venue_cases<A: DeserializeOwned>(
    chain: &str,
    venue: &str,
) -> Vec<(String, RecordedCase<A>)> {
    let dir = fixtures_dir().join(chain).join(venue);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("read case directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter(|path| {
            !VENUE_METADATA_STEMS
                .iter()
                .any(|stem| path.file_stem().is_some_and(|name| name == *stem))
        })
        .collect();
    paths.sort_by(|a, b| a.file_stem().cmp(&b.file_stem()));
    paths
        .into_iter()
        .map(|path| {
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("case file name is UTF-8")
                .to_owned();
            let case = read_case(&path);
            (name, case)
        })
        .collect()
}

/// Every case of one chain as `(venue, case name, case)`, sorted by venue then case name.
pub fn load_chain_cases<A: DeserializeOwned>(
    chain: &str,
) -> Vec<(String, String, RecordedCase<A>)> {
    let dir = fixtures_dir().join(chain);
    let mut venues: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("read chain directory entry").path())
        .filter(|path| path.is_dir())
        .map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .expect("venue directory name is UTF-8")
                .to_owned()
        })
        .collect();
    venues.sort();
    venues
        .into_iter()
        .flat_map(|venue| {
            load_venue_cases(chain, &venue)
                .into_iter()
                .map(move |(name, case)| (venue.clone(), name, case))
        })
        .collect()
}

fn read_case<A: DeserializeOwned>(path: &std::path::Path) -> RecordedCase<A> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read case {}: {e}", path.display()));
    let case: RecordedCase<A> = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("parse case {}: {e}", path.display()));
    assert_eq!(
        case.schema,
        SCHEMA_VERSION,
        "case {} has an unsupported schema version",
        path.display()
    );
    case
}
