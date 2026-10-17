// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! DEX analysis — identifies swap instructions and extracts trade details from DEX programs.
//
// DEX detection module - Router and DEX identification system
//
// This module implements the industry-standard DEX detection approach used by
// DexScreener, GMGN, and Birdeye for identifying which DEX/router processed a swap.
//
// Detection strategy:
// 1. Program ID detection (primary method) - Jupiter V6, Raydium CLMM, Orca, PumpFun
// 2. Log parsing fallback - Extract swap events from program logs
// 3. Pool address recognition - Identify specific pool contracts
// 4. Confidence scoring - Weight multiple detection signals

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::balance::BalanceAnalysis;
use crate::chains::solana::transactions::program_ids::*;
use crate::logger::{self, LogTag};
use crate::transactions::types::*;

// =============================================================================
// DEX ANALYSIS TYPES
// =============================================================================

/// Comprehensive DEX detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexAnalysis {
    /// Primary DEX/router detected
    pub detected_dex: Option<DetectedDex>,
    /// Pool address if identified
    pub pool_address: Option<String>,
    /// All program IDs found in transaction
    pub program_ids: Vec<String>,
    /// Confidence score for detection (0.0 - 1.0)
    pub confidence: f64,
    /// Detection method used
    pub detection_method: DetectionMethod,
    /// Additional metadata from logs
    pub metadata: HashMap<String, String>,
}

/// Supported DEX platforms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectedDex {
    Jupiter,
    Raydium,
    RaydiumCLMM,
    Orca,
    OrcaWhirlpool,
    PumpFun,
    Meteora,
    Lifinity,
    Aldrin,
    Serum,
    OpenBook,
    Phoenix,
    Gmgn,
    Raptor,
    Moonshot,
    Fluxbeam,
}

impl DetectedDex {
    /// Every platform, in declaration order.
    pub const ALL: [DetectedDex; 16] = [
        DetectedDex::Jupiter,
        DetectedDex::Raydium,
        DetectedDex::RaydiumCLMM,
        DetectedDex::Orca,
        DetectedDex::OrcaWhirlpool,
        DetectedDex::PumpFun,
        DetectedDex::Meteora,
        DetectedDex::Lifinity,
        DetectedDex::Aldrin,
        DetectedDex::Serum,
        DetectedDex::OpenBook,
        DetectedDex::Phoenix,
        DetectedDex::Gmgn,
        DetectedDex::Raptor,
        DetectedDex::Moonshot,
        DetectedDex::Fluxbeam,
    ];

    /// Stable router id stored in a transaction's `router` field. The dashboard
    /// labels it through `ROUTER_LABELS` (ui/venue.js) and Rust through
    /// `transactions::router_label`; a new variant fails to compile here until
    /// it has an id, and fails `every_router_id_has_a_venue_label` until labelled.
    pub const fn router_id(&self) -> &'static str {
        match self {
            DetectedDex::Jupiter => "jupiter",
            DetectedDex::Raydium => "raydium",
            DetectedDex::RaydiumCLMM => "raydiumclmm",
            DetectedDex::Orca => "orca",
            DetectedDex::OrcaWhirlpool => "orcawhirlpool",
            DetectedDex::PumpFun => "pumpfun",
            DetectedDex::Meteora => "meteora",
            DetectedDex::Lifinity => "lifinity",
            DetectedDex::Aldrin => "aldrin",
            DetectedDex::Serum => "serum",
            DetectedDex::OpenBook => "openbook",
            DetectedDex::Phoenix => "phoenix",
            DetectedDex::Gmgn => "gmgn",
            DetectedDex::Raptor => "raptor",
            DetectedDex::Moonshot => "moonshot",
            DetectedDex::Fluxbeam => "fluxbeam",
        }
    }

    /// The platform behind a stored router id.
    pub fn from_router_id(router: &str) -> Option<DetectedDex> {
        DetectedDex::ALL
            .into_iter()
            .find(|dex| dex.router_id() == router)
    }
}

/// Method used for DEX detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionMethod {
    ProgramId,   // Direct program ID match
    LogParsing,  // Event log analysis
    PoolAddress, // Pool contract recognition
    Heuristic,   // Pattern-based detection
    Combined,    // Multiple methods
}

// =============================================================================
// LOG PATTERNS
// =============================================================================

/// How a log line must match a pattern.
enum LogMatch {
    /// The whole line.
    Line(&'static str),
    /// The start of the line; the rest is a payload.
    Prefix(&'static str),
}

/// Log lines that name a platform when no known program is present, in priority order.
const LOG_PATTERNS: &[(LogMatch, DetectedDex)] = &[
    (
        LogMatch::Prefix("Program log: ray_log:"),
        DetectedDex::Raydium,
    ),
    (
        LogMatch::Line("Program log: Instruction: Swap"),
        DetectedDex::Jupiter,
    ),
    (
        LogMatch::Line("Program log: Instruction: swap"),
        DetectedDex::Orca,
    ),
    (
        LogMatch::Line("Program log: Instruction: buy"),
        DetectedDex::PumpFun,
    ),
    (
        LogMatch::Line("Program log: Instruction: sell"),
        DetectedDex::PumpFun,
    ),
];

// =============================================================================
// MAIN DETECTION FUNCTIONS
// =============================================================================
// MAIN PUBLIC FUNCTIONS
// =============================================================================

/// Main public function for detecting DEX interactions
pub async fn detect_dex_interactions(
    transaction: &Transaction,
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    balance_analysis: &BalanceAnalysis,
) -> crate::chains::solana::Result<DexAnalysis> {
    detect_dex_and_router(transaction, tx_data, balance_analysis).await
}

/// Comprehensive DEX detection with confidence scoring
pub async fn detect_dex_and_router(
    transaction: &Transaction,
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    _balance_analysis: &BalanceAnalysis,
) -> crate::chains::solana::Result<DexAnalysis> {
    logger::debug(
        LogTag::Transactions,
        &format!("Detecting DEX/router for tx: {}", transaction.signature),
    );

    // Step 1: Extract all program IDs from instructions
    let program_ids = extract_program_ids(tx_data)?;

    // Step 2: Try program ID detection (primary method)
    let (program_detection, program_confidence) = detect_by_program_id(&program_ids);

    // Step 3: Try log parsing, only when no known program names the swap
    let (log_detection, log_confidence) = if program_detection.is_some() {
        (None, 0.0)
    } else {
        detect_by_log_parsing(tx_data)
    };

    // Step 4: Try pool address detection
    let (pool_detection, pool_address, pool_confidence) = detect_by_pool_address(tx_data)?;

    // Step 5: Combine results with confidence weighting
    let (final_dex, detection_method, confidence) = combine_detection_results(
        program_detection,
        program_confidence,
        log_detection,
        log_confidence,
        pool_detection,
        pool_confidence,
    );

    // Step 6: Extract metadata from logs
    let metadata = extract_dex_metadata(tx_data, &final_dex)?;

    Ok(DexAnalysis {
        detected_dex: final_dex,
        pool_address,
        program_ids,
        confidence,
        detection_method,
        metadata,
    })
}

/// Quick DEX detection for performance-critical paths
pub async fn quick_dex_detection(
    _transaction: &Transaction,
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
) -> crate::chains::solana::Result<DexAnalysis> {
    // Lightweight detection - just program ID matching
    let program_ids = extract_program_ids(tx_data)?;
    let (detected_dex, confidence) = detect_by_program_id(&program_ids);

    Ok(DexAnalysis {
        detected_dex,
        pool_address: None,
        program_ids,
        confidence,
        detection_method: DetectionMethod::ProgramId,
        metadata: HashMap::new(),
    })
}

// =============================================================================
// DETECTION METHODS
// =============================================================================

/// Detect DEX by program ID (most reliable method)
fn detect_by_program_id(program_ids: &[String]) -> (Option<DetectedDex>, f64) {
    match detect_known_program(program_ids) {
        Some(known) => (
            DetectedDex::from_router_id(known.router),
            if known.aggregator { 0.95 } else { 0.9 },
        ),
        None => (None, 0.0),
    }
}

/// Detect DEX by parsing transaction logs
fn detect_by_log_parsing(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
) -> (Option<DetectedDex>, f64) {
    let empty_logs = Vec::new();
    let logs = tx_data
        .meta
        .as_ref()
        .and_then(|m| m.log_messages.as_ref())
        .unwrap_or(&empty_logs);

    for log in logs {
        for (pattern, dex) in LOG_PATTERNS {
            let matched = match pattern {
                LogMatch::Line(line) => log == line,
                LogMatch::Prefix(prefix) => log.starts_with(prefix),
            };
            if matched {
                return (Some(dex.clone()), 0.7); // Medium confidence for log match
            }
        }
    }

    (None, 0.0)
}

/// Detect DEX by recognizing known pool addresses
fn detect_by_pool_address(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
) -> crate::chains::solana::Result<(Option<DetectedDex>, Option<String>, f64)> {
    // This would be expanded with a database of known pool addresses
    // For now, we'll use heuristics based on account patterns

    let message = &tx_data.transaction.message;
    let account_keys = extract_account_keys(message);

    // Look for Raydium pool patterns (specific account structure)
    for account in &account_keys {
        if is_raydium_pool_pattern(account) {
            return Ok((Some(DetectedDex::Raydium), Some(account.clone()), 0.8));
        }
        if is_orca_pool_pattern(account) {
            return Ok((Some(DetectedDex::Orca), Some(account.clone()), 0.8));
        }
    }

    Ok((None, None, 0.0))
}

// =============================================================================
// RESULT COMBINATION
// =============================================================================

/// Combine multiple detection results with confidence weighting
fn combine_detection_results(
    program_detection: Option<DetectedDex>,
    program_confidence: f64,
    log_detection: Option<DetectedDex>,
    log_confidence: f64,
    pool_detection: Option<DetectedDex>,
    pool_confidence: f64,
) -> (Option<DetectedDex>, DetectionMethod, f64) {
    // Weighted scoring: program ID > pool address > log parsing
    let program_weight = 1.0;
    let pool_weight = 0.8;
    let log_weight = 0.6;

    let program_score = program_confidence * program_weight;
    let pool_score = pool_confidence * pool_weight;
    let log_score = log_confidence * log_weight;

    // Find highest scoring detection
    if program_score >= pool_score && program_score >= log_score && program_detection.is_some() {
        (
            program_detection,
            DetectionMethod::ProgramId,
            program_confidence,
        )
    } else if pool_score >= log_score && pool_detection.is_some() {
        (
            pool_detection,
            DetectionMethod::PoolAddress,
            pool_confidence,
        )
    } else if log_detection.is_some() {
        (log_detection, DetectionMethod::LogParsing, log_confidence)
    } else {
        (None, DetectionMethod::Heuristic, 0.0)
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Extract all program IDs from transaction instructions
pub(crate) fn extract_program_ids(
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
) -> crate::chains::solana::Result<Vec<String>> {
    let mut program_ids = Vec::new();

    let message = &tx_data.transaction.message;
    let account_keys = extract_account_keys(message);

    // Extract from outer instructions
    if let Some(instructions) = message.get("instructions").and_then(|v| v.as_array()) {
        for instruction in instructions {
            // Prefer direct programId string when available
            if let Some(pid) = instruction.get("programId").and_then(|v| v.as_str()) {
                program_ids.push(pid.to_string());
            } else if let Some(program_id_index) =
                instruction.get("programIdIndex").and_then(|v| v.as_u64())
            {
                if let Some(program_id) = account_keys.get(program_id_index as usize) {
                    program_ids.push(program_id.clone());
                }
            }
        }
    }

    // Extract from inner instructions
    if let Some(meta) = &tx_data.meta {
        if let Some(inner_instructions) = &meta.inner_instructions {
            for inner_ix_group in inner_instructions {
                if let Some(instructions) = inner_ix_group
                    .get("instructions")
                    .and_then(|v| v.as_array())
                {
                    for inner_ix in instructions {
                        // Prefer direct programId string when available
                        if let Some(pid) = inner_ix.get("programId").and_then(|v| v.as_str()) {
                            program_ids.push(pid.to_string());
                        } else if let Some(program_id_index) =
                            inner_ix.get("programIdIndex").and_then(|v| v.as_u64())
                        {
                            if let Some(program_id) = account_keys.get(program_id_index as usize) {
                                program_ids.push(program_id.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Remove duplicates
    program_ids.sort();
    program_ids.dedup();

    Ok(program_ids)
}

/// Extract account keys from transaction message
fn extract_account_keys(message: &Value) -> Vec<String> {
    // Legacy or v0 array format
    if let Some(array) = message.get("accountKeys").and_then(|v| v.as_array()) {
        // Try plain strings first
        let mut keys: Vec<String> = array
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if !keys.is_empty() {
            return keys;
        }
        // Fallback: array of objects with { pubkey, ... }
        keys = array
            .iter()
            .filter_map(|v| {
                v.get("pubkey")
                    .and_then(|p| p.as_str())
                    .map(|s| s.to_string())
            })
            .collect();
        return keys;
    }

    // v0 format
    if let Some(obj) = message.get("accountKeys").and_then(|v| v.as_object()) {
        let mut keys = Vec::new();

        if let Some(static_keys) = obj.get("staticAccountKeys").and_then(|v| v.as_array()) {
            keys.extend(
                static_keys
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string()),
            );
        }

        if let Some(loaded) = obj.get("loadedAddresses").and_then(|v| v.as_object()) {
            if let Some(writable) = loaded.get("writable").and_then(|v| v.as_array()) {
                keys.extend(
                    writable
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string()),
                );
            }
            if let Some(readonly) = loaded.get("readonly").and_then(|v| v.as_array()) {
                keys.extend(
                    readonly
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string()),
                );
            }
        }

        return keys;
    }

    Vec::new()
}

/// Check if account follows Raydium pool pattern
fn is_raydium_pool_pattern(account: &str) -> bool {
    // Simplified heuristic - would be replaced with actual pool recognition
    account.len() == 44 && account.starts_with("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWbkwMihLYQo2")
}

/// Check if account follows Orca pool pattern
fn is_orca_pool_pattern(account: &str) -> bool {
    // Simplified heuristic - would be replaced with actual pool recognition
    account.len() == 44 && account.starts_with("2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ")
}

/// Extract DEX-specific metadata from transaction logs
fn extract_dex_metadata(
    _tx_data: &crate::chains::solana::rpc::TransactionDetails,
    detected_dex: &Option<DetectedDex>,
) -> crate::chains::solana::Result<HashMap<String, String>> {
    let mut metadata = HashMap::new();

    if let Some(dex) = detected_dex {
        match dex {
            DetectedDex::Jupiter => {
                metadata.insert("aggregator".to_owned(), "jupiter".to_owned());
            }
            DetectedDex::PumpFun => {
                metadata.insert("meme_platform".to_owned(), "pumpfun".to_owned());
            }
            DetectedDex::RaydiumCLMM => {
                metadata.insert("amm_type".to_owned(), "clmm".to_owned());
            }
            DetectedDex::OrcaWhirlpool => {
                metadata.insert("amm_type".to_owned(), "whirlpool".to_owned());
            }
            _ => {}
        }
    }

    Ok(metadata)
}
