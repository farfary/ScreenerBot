//! Deterministic, wallet-relative classification.
//!
//! The graph classifier in `classify.rs` answers "was this a swap, and which way".
//! It is good at that and blind to everything else: with no DEX program and no
//! clean transfer edge it produces no patterns at all and returns `Unknown`. That
//! is why 106 of 629 recorded transactions — every single one a plain SPL
//! `closeAccount` reclaiming rent — listed as `UNKNOWN` with no direction.
//!
//! This pass runs after the graph classifier and answers the rest of the question
//! from two things the graph never looks at: the parsed top-level instruction set,
//! and the subject wallet's own deltas ([`WalletView`]). It never overrides a
//! confident swap verdict; it replaces `Unknown` and sharpens the coarse
//! `Transfer`. `Unknown` survives only when the transaction could not be decoded
//! at all.

use crate::chains::solana::constants::SOL_MINT;
use crate::chains::solana::rpc::TransactionDetails;
use crate::chains::solana::transactions::program_ids::detect_router_from_program_id;
use crate::transactions::types::{TransactionType, DUST_LAMPORTS};

use super::classify::ClassifiedType;
use super::dex::DexAnalysis;
use super::wallet_view::WalletView;

const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
const COMPUTE_BUDGET_PROGRAM: &str = "ComputeBudget111111111111111111111111111111";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const ATA_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
const MEMO_PROGRAM: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
const MEMO_V1_PROGRAM: &str = "Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo";

/// Programs whose presence makes a transaction an NFT operation rather than a
/// generic program call.
const NFT_PROGRAMS: &[(&str, &str)] = &[
    ("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s", "Metaplex"),
    ("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY", "Bubblegum"),
    ("M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K", "Magic Eden"),
    ("TCMPhJdwDryooaGtiocG1u3xcYbRpiJzb283XfCZsDp", "Tensor"),
    ("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d", "Core"),
];

/// One decoded top-level instruction, reduced to what classification needs.
struct Call {
    program_id: String,
    /// `spl-token`, `system`, … when the RPC named it.
    program: Option<String>,
    /// `closeAccount`, `transfer`, `createIdempotent`, … when jsonParsed decoded it.
    parsed_type: Option<String>,
}

impl Call {
    fn is(&self, program_id: &str, parsed_type: &str) -> bool {
        self.program_id == program_id && self.parsed_type.as_deref() == Some(parsed_type)
    }

    /// Instructions that only shape execution and never move value.
    fn is_overhead(&self) -> bool {
        self.program_id == COMPUTE_BUDGET_PROGRAM
            || self.program_id == MEMO_PROGRAM
            || self.program_id == MEMO_V1_PROGRAM
    }
}

/// Refines a graph verdict into the transaction type actually worth showing.
pub fn refine(
    graph_type: &ClassifiedType,
    view: &WalletView,
    tx_data: &TransactionDetails,
    dex: &DexAnalysis,
    success: bool,
) -> TransactionType {
    let calls = top_level_calls(tx_data);
    let router = router_name(dex, &calls);

    // A confident swap verdict is the graph classifier's job; carry it through with
    // the mints and amounts this pass can attach for free.
    match graph_type {
        ClassifiedType::Buy => return swap_type(view, &router, true),
        ClassifiedType::Sell => return swap_type(view, &router, false),
        ClassifiedType::Swap => {
            if let Some(rich) = token_to_token(view, &router) {
                return rich;
            }
        }
        ClassifiedType::AddLiquidity => {
            return TransactionType::LiquidityAdd {
                pool: dex.pool_address.clone().unwrap_or_default(),
                router,
            }
        }
        ClassifiedType::RemoveLiquidity => {
            return TransactionType::LiquidityRemove {
                pool: dex.pool_address.clone().unwrap_or_default(),
                router,
            }
        }
        _ => {}
    }

    if !success {
        // Nothing moved, so every flow-derived answer below would describe value
        // that never changed hands. The attempt's shape is still worth keeping when
        // the graph classifier recognised it above; otherwise this is just a fee.
        return TransactionType::Failed;
    }

    if let Some(rich) = classify_from_calls(&calls, view, success) {
        return rich;
    }

    // Nothing structural matched, so fall back to the wallet's own value movement.
    if let Some(rich) = classify_from_flow(view) {
        return rich;
    }

    if let Some((program, name)) = nft_program(&calls) {
        return TransactionType::NftOperation {
            program,
            detail: name.to_owned(),
        };
    }

    match primary_program(&calls) {
        Some(program) => TransactionType::ProgramInteraction {
            detail: program_label(&program, &calls),
            program,
        },
        None => TransactionType::Unknown,
    }
}

/// Structural classification: what the transaction was *built* to do.
fn classify_from_calls(
    calls: &[Call],
    view: &WalletView,
    success: bool,
) -> Option<TransactionType> {
    let effective: Vec<&Call> = calls.iter().filter(|call| !call.is_overhead()).collect();

    if effective.is_empty() {
        // Nothing but compute-budget/memo instructions ran. A failed transaction that
        // never got past its budget is still a fee the owner paid, not a mystery.
        return Some(if success {
            TransactionType::Compute
        } else {
            TransactionType::Failed
        });
    }

    let closes: Vec<&&Call> = effective
        .iter()
        .filter(|call| {
            call.is(TOKEN_PROGRAM, "closeAccount") || call.is(TOKEN_2022_PROGRAM, "closeAccount")
        })
        .collect();
    let creates: Vec<&&Call> = effective
        .iter()
        .filter(|call| {
            call.program_id == ATA_PROGRAM
                || call.is(TOKEN_PROGRAM, "initializeAccount")
                || call.is(TOKEN_PROGRAM, "initializeAccount3")
        })
        .collect();
    let token_moves = effective.iter().filter(|call| {
        matches!(
            call.parsed_type.as_deref(),
            Some("transfer" | "transferChecked")
        ) && (call.program_id == TOKEN_PROGRAM || call.program_id == TOKEN_2022_PROGRAM)
    });
    let token_move_count = token_moves.count();
    let sol_moves = effective
        .iter()
        .filter(|call| call.is(SYSTEM_PROGRAM, "transfer"))
        .count();

    let accounted = closes.len() + creates.len() + token_move_count + sol_moves;
    if accounted != effective.len() {
        // Some other program did real work here: not a bare account/transfer shape.
        return None;
    }

    if !closes.is_empty() && token_move_count == 0 && sol_moves == 0 {
        return Some(TransactionType::AtaClose {
            recovered_sol: view.rent_recovered_lamports as f64 / 1_000_000_000.0,
            token_mint: view.closed_mints.first().cloned().unwrap_or_default(),
        });
    }

    if !creates.is_empty() && token_move_count == 0 && sol_moves == 0 && closes.is_empty() {
        return Some(TransactionType::AtaCreate {
            rent_paid: view.rent_paid_lamports as f64 / 1_000_000_000.0,
            token_mint: view.created_mints.first().cloned().unwrap_or_default(),
        });
    }

    if token_move_count > 0 {
        let delta = view.dominant_token()?;
        if let Some(spam) = spam_airdrop(view, delta.raw, &delta.mint, delta.ui) {
            return Some(spam);
        }
        let (from, to) = counterparties(view, delta.raw > 0);
        return Some(TransactionType::TokenTransfer {
            mint: delta.mint.clone(),
            amount: delta.ui.abs(),
            from,
            to,
        });
    }

    if sol_moves > 0 {
        let lamports = view.lamport_delta_excluding_fee();
        if let Some(dust) = dust(view, lamports) {
            return Some(dust);
        }
        let (from, to) = counterparties(view, lamports > 0);
        return Some(TransactionType::SolTransfer {
            amount: lamports.unsigned_abs() as f64 / 1_000_000_000.0,
            from,
            to,
        });
    }

    None
}

/// Value-flow classification for transactions no instruction shape covered.
fn classify_from_flow(view: &WalletView) -> Option<TransactionType> {
    if let Some(delta) = view.dominant_token() {
        if let Some(spam) = spam_airdrop(view, delta.raw, &delta.mint, delta.ui) {
            return Some(spam);
        }
    }
    let lamports = view.lamport_delta_excluding_fee();
    if let Some(dust) = dust(view, lamports) {
        return Some(dust);
    }
    if view.rent_recovered_lamports > 0 && view.rent_paid_lamports == 0 {
        return Some(TransactionType::AtaClose {
            recovered_sol: view.rent_recovered_lamports as f64 / 1_000_000_000.0,
            token_mint: view.closed_mints.first().cloned().unwrap_or_default(),
        });
    }
    None
}

/// An inbound crumb on a transaction we neither signed nor paid for.
fn dust(view: &WalletView, lamports_excluding_fee: i64) -> Option<TransactionType> {
    if view.signed || lamports_excluding_fee <= 0 {
        return None;
    }
    if lamports_excluding_fee as u64 > DUST_LAMPORTS {
        return None;
    }
    Some(TransactionType::Dust {
        sol_amount: lamports_excluding_fee as f64 / 1_000_000_000.0,
        from: view
            .sol_credit_sources
            .first()
            .map(|(source, _)| source.clone())
            .unwrap_or_default(),
    })
}

/// An inbound token credit on a transaction we did not sign is an airdrop we did
/// not ask for. Value is irrelevant here: an unsolicited million-token credit is
/// the classic worthless-token spam.
fn spam_airdrop(view: &WalletView, raw: i128, mint: &str, ui: f64) -> Option<TransactionType> {
    (!view.signed && raw > 0).then(|| TransactionType::SpamAirdrop {
        mint: mint.to_owned(),
        amount: ui.abs(),
        from: view
            .sol_credit_sources
            .first()
            .map(|(source, _)| source.clone())
            .unwrap_or_default(),
    })
}

fn counterparties(view: &WalletView, incoming: bool) -> (String, String) {
    let other = view
        .sol_credit_sources
        .first()
        .map(|(source, _)| source.clone())
        .unwrap_or_default();
    if incoming {
        (other, view.wallet.clone())
    } else {
        (view.wallet.clone(), other)
    }
}

fn swap_type(view: &WalletView, router: &str, buy: bool) -> TransactionType {
    let token = view.dominant_token();
    let token_mint = token.map(|d| d.mint.clone()).unwrap_or_default();
    let token_amount = token.map(|d| d.ui.abs()).unwrap_or_default();
    let sol_amount = view.lamport_delta_excluding_fee().unsigned_abs() as f64 / 1_000_000_000.0;
    if buy {
        TransactionType::SwapSolToToken {
            token_mint,
            sol_amount,
            token_amount,
            router: router.to_owned(),
        }
    } else {
        TransactionType::SwapTokenToSol {
            token_mint,
            token_amount,
            sol_amount,
            router: router.to_owned(),
        }
    }
}

fn token_to_token(view: &WalletView, router: &str) -> Option<TransactionType> {
    let from = view.token_deltas.iter().find(|d| d.raw < 0)?;
    let to = view.token_deltas.iter().find(|d| d.raw > 0)?;
    Some(TransactionType::SwapTokenToToken {
        from_mint: from.mint.clone(),
        to_mint: to.mint.clone(),
        from_amount: from.ui.abs(),
        to_amount: to.ui.abs(),
        router: router.to_owned(),
    })
}

fn router_name(dex: &DexAnalysis, calls: &[Call]) -> String {
    if let Some(detected) = dex.detected_dex.as_ref() {
        return format!("{detected:?}").to_ascii_lowercase();
    }
    calls
        .iter()
        .find_map(|call| detect_router_from_program_id(&call.program_id))
        .unwrap_or("unknown")
        .to_owned()
}

fn nft_program(calls: &[Call]) -> Option<(String, &'static str)> {
    calls.iter().find_map(|call| {
        NFT_PROGRAMS
            .iter()
            .find(|(id, _)| *id == call.program_id)
            .map(|(id, name)| ((*id).to_owned(), *name))
    })
}

/// The program that did the work, ignoring execution overhead.
fn primary_program(calls: &[Call]) -> Option<String> {
    calls
        .iter()
        .find(|call| !call.is_overhead())
        .map(|call| call.program_id.clone())
        .or_else(|| calls.first().map(|call| call.program_id.clone()))
}

fn program_label(program_id: &str, calls: &[Call]) -> String {
    if let Some(router) = detect_router_from_program_id(program_id) {
        return router.to_owned();
    }
    let named = calls.iter().find(|call| call.program_id == program_id);
    match named.and_then(|call| call.program.as_deref()) {
        Some(program) => program.to_owned(),
        None if program_id == SOL_MINT => "wrapped SOL".to_owned(),
        None => program_id.chars().take(8).collect(),
    }
}

fn top_level_calls(tx_data: &TransactionDetails) -> Vec<Call> {
    let Some(instructions) = tx_data
        .transaction
        .message
        .get("instructions")
        .and_then(|v| v.as_array())
    else {
        return Vec::new();
    };
    let keys = super::wallet_view::account_keys(&tx_data.transaction.message);
    instructions
        .iter()
        .map(|instruction| {
            // jsonParsed sends `programId`; the raw encoding sends `programIdIndex`.
            // Reading only the index — as the ATA analyzer did — resolves every
            // instruction to account 0, which is the fee payer, never a program.
            let program_id = instruction
                .get("programId")
                .and_then(|v| v.as_str())
                .map(str::to_owned)
                .or_else(|| {
                    instruction
                        .get("programIdIndex")
                        .and_then(serde_json::Value::as_u64)
                        .and_then(|index| keys.get(index as usize).cloned())
                })
                .unwrap_or_default();
            Call {
                program_id,
                program: instruction
                    .get("program")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned),
                parsed_type: instruction
                    .get("parsed")
                    .and_then(|parsed| parsed.get("type"))
                    .and_then(|v| v.as_str())
                    .map(str::to_owned),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::rpc::TransactionDetails;

    const WALLET: &str = "Wa11etUnderTest1111111111111111111111111111";

    fn details(json: serde_json::Value) -> TransactionDetails {
        serde_json::from_value(json).expect("fixture parses as TransactionDetails")
    }

    fn no_dex() -> DexAnalysis {
        DexAnalysis {
            detected_dex: None,
            program_ids: Vec::new(),
            pool_address: None,
            confidence: 0.0,
            detection_method:
                crate::chains::solana::transactions::analyzer::dex::DetectionMethod::Heuristic,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// A bare SPL `closeAccount`, the exact shape of the 106 transactions that
    /// listed as `UNKNOWN` with a `-0.000005 SOL` delta.
    fn ata_close() -> TransactionDetails {
        details(serde_json::json!({
            "slot": 384229878,
            "block_time": 1764776769,
            "meta": {
                "err": null,
                "fee": 5000,
                "preBalances": [72773882u64, 2039280u64, 5313147188u64],
                "postBalances": [74808162u64, 0u64, 5313147188u64],
                "preTokenBalances": [{
                    "accountIndex": 1,
                    "mint": "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
                    "owner": WALLET,
                    "uiTokenAmount": { "amount": "0", "decimals": 5, "uiAmount": 0.0 }
                }],
                "postTokenBalances": [],
                "innerInstructions": []
            },
            "transaction": {
                "signatures": ["sig"],
                "message": {
                    "accountKeys": [
                        { "pubkey": WALLET, "signer": true, "writable": true },
                        { "pubkey": "TokenAccountUnderTest111111111111111111111", "signer": false, "writable": true },
                        { "pubkey": TOKEN_PROGRAM, "signer": false, "writable": false }
                    ],
                    "instructions": [{
                        "program": "spl-token",
                        "programId": TOKEN_PROGRAM,
                        "parsed": { "type": "closeAccount", "info": {
                            "account": "TokenAccountUnderTest111111111111111111111",
                            "destination": WALLET,
                            "owner": WALLET
                        }}
                    }]
                }
            }
        }))
    }

    /// One lamport from an address-poisoning blaster: 20 transfers in one
    /// transaction, none of them ours to pay for.
    fn dust_blast() -> TransactionDetails {
        details(serde_json::json!({
            "slot": 1,
            "block_time": 1764776769,
            "meta": {
                "err": null,
                "fee": 5825,
                "preBalances": [1_000_000u64, 500u64],
                "postBalances": [994_175u64, 501u64],
                "preTokenBalances": [],
                "postTokenBalances": [],
                "innerInstructions": []
            },
            "transaction": {
                "signatures": ["sig"],
                "message": {
                    "accountKeys": [
                        { "pubkey": "DustBlaster11111111111111111111111111111111", "signer": true, "writable": true },
                        { "pubkey": WALLET, "signer": false, "writable": true }
                    ],
                    "instructions": [{
                        "program": "system",
                        "programId": SYSTEM_PROGRAM,
                        "parsed": { "type": "transfer", "info": {
                            "source": "DustBlaster11111111111111111111111111111111",
                            "destination": WALLET,
                            "lamports": 1
                        }}
                    }]
                }
            }
        }))
    }

    #[test]
    fn wallet_view_reports_the_wallet_delta_not_the_transaction_sum() {
        let view = WalletView::build(&ata_close(), WALLET);

        // Summing every account's change yields -fee, which is what the dashboard
        // showed on nearly every row. The wallet's own change is the rent it got back.
        assert_eq!(view.lamport_delta, 2_034_280);
        assert_eq!(view.lamport_delta_excluding_fee(), 2_039_280);
        assert!(view.signed);
        assert_eq!(view.rent_recovered_lamports, 2_039_280);
        assert_eq!(
            view.closed_mints,
            vec!["DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_owned()]
        );
    }

    #[test]
    fn bare_close_account_is_a_rent_reclaim_not_unknown() {
        let tx = ata_close();
        let view = WalletView::build(&tx, WALLET);
        let classified = refine(&ClassifiedType::Unknown, &view, &tx, &no_dex(), true);

        match classified {
            TransactionType::AtaClose {
                recovered_sol,
                ref token_mint,
            } => {
                assert!((recovered_sol - 0.00203928).abs() < 1e-9);
                assert_eq!(token_mint, "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263");
            }
            other => panic!("expected AtaClose, got {other:?}"),
        }
        assert_eq!(classified.kind(), "ata_close");
    }

    #[test]
    fn unsolicited_lamport_is_dust() {
        let tx = dust_blast();
        let view = WalletView::build(&tx, WALLET);
        assert!(!view.signed);
        // Someone else paid the fee, so none of it is ours to subtract.
        assert_eq!(view.fee_lamports, 0);

        let classified = refine(&ClassifiedType::Transfer, &view, &tx, &no_dex(), true);
        match classified {
            TransactionType::Dust {
                sol_amount,
                ref from,
            } => {
                assert!((sol_amount - 1e-9).abs() < 1e-12);
                assert_eq!(from, "DustBlaster11111111111111111111111111111111");
            }
            other => panic!("expected Dust, got {other:?}"),
        }
        assert_eq!(classified.kind(), "dust");
    }

    #[test]
    fn a_failed_transaction_never_claims_a_flow() {
        let tx = dust_blast();
        let view = WalletView::build(&tx, WALLET);
        let classified = refine(&ClassifiedType::Transfer, &view, &tx, &no_dex(), false);
        assert_eq!(classified.kind(), "failed");
    }

    #[test]
    fn every_kind_round_trips_through_serde() {
        // The kind is a column value and the payload is stored as serde JSON; a
        // variant that cannot deserialize reads back as Unknown forever.
        let samples = [
            TransactionType::AtaClose {
                recovered_sol: 0.002,
                token_mint: "mint".to_owned(),
            },
            TransactionType::Dust {
                sol_amount: 1e-9,
                from: "who".to_owned(),
            },
            TransactionType::SpamAirdrop {
                mint: "mint".to_owned(),
                amount: 1.0,
                from: "who".to_owned(),
            },
            TransactionType::ProgramInteraction {
                program: "prog".to_owned(),
                detail: "detail".to_owned(),
            },
        ];
        for sample in samples {
            let encoded = serde_json::to_string(&sample).expect("encodes");
            let decoded: TransactionType = serde_json::from_str(&encoded).expect("decodes");
            assert_eq!(decoded.kind(), sample.kind());
            assert_ne!(decoded.kind(), "unknown");
        }
    }
}
