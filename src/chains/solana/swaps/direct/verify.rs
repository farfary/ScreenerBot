//! Proving what a confirmed swap actually delivered.
//!
//! Two different guarantees, and they must not be confused:
//!
//! * SAFETY comes from `min_out` inside the swap instruction. The pool program
//!   itself refuses to return less, so a transaction that confirmed WITHOUT an
//!   error already proves at least the minimum arrived. Nothing measured here
//!   can make a confirmed swap unsafe after the fact.
//! * ACCOUNTING comes from this module. P&L, fills and position sizing need the
//!   amount that actually landed, not the estimate.
//!
//! The two output shapes are measured differently, and only a TOKEN output was
//! ever exact:
//!
//! * A TOKEN output is exact. The transaction's own pre/post token balances for
//!   the owner and mint give the delta to the raw unit, and a delta below the
//!   guaranteed minimum is a hard [`DirectSwapError::OutputNotReceived`].
//! * A NATIVE SOL output is now ALSO exact, most of the time. The WSOL account is
//!   closed in the same transaction, so its own balance is gone by the time this
//!   reads back -- but the swap's own CPI token transfer INTO that account, before
//!   it closes, is still in `meta.innerInstructions`, and it carries the precise
//!   amount the pool paid out. Falling back to the owner's lamport delta (which
//!   also carries the transaction fee and any rent that moved) only happens when
//!   the inner instructions are missing or unparsable -- never as the first
//!   choice, and never used to FAIL a swap the chain already accepted.

use super::error::{DirectSwapError, DirectSwapResult};
use super::plan::SwapPlan;
use crate::chains::solana::rpc::types::TransactionDetails;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// What a confirmed swap delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Receipt {
    /// Raw units of the output mint that reached the wallet.
    pub received: u64,
    /// Whether [`Self::received`] is an exact on-chain measurement. False only
    /// when a native-SOL output could not be read from the inner instructions
    /// and fell back to the lamport delta.
    pub exact: bool,
    /// Network fee the transaction paid, in lamports.
    pub network_fee_lamports: u64,
    /// Slot the transaction landed in.
    pub slot: u64,
}

/// Read the receipt out of a confirmed transaction.
///
/// `signature` is only used to build errors; the caller has already confirmed it.
pub fn receipt_from_transaction(
    signature: &str,
    owner: &Pubkey,
    plan: &SwapPlan,
    details: &TransactionDetails,
) -> DirectSwapResult<Receipt> {
    let meta = details
        .meta
        .as_ref()
        .ok_or_else(|| DirectSwapError::TransactionFailed {
            signature: signature.to_owned(),
            detail: "the confirmed transaction carries no metadata to verify against".to_owned(),
        })?;

    if let Some(err) = &meta.err {
        return Err(DirectSwapError::TransactionFailed {
            signature: signature.to_owned(),
            detail: err.to_string(),
        });
    }

    if plan.output_is_native {
        return native_receipt(signature, owner, plan, details, meta);
    }

    let owner_str = owner.to_string();
    let mint_str = plan.quote.output_mint.to_string();
    let balance_of = |balances: &Option<Vec<crate::chains::solana::rpc::types::TokenBalance>>| {
        balances
            .as_ref()
            .map(|list| {
                list.iter()
                    .filter(|b| {
                        b.mint == mint_str && b.owner.as_deref() == Some(owner_str.as_str())
                    })
                    .filter_map(|b| b.ui_token_amount.amount.parse::<u128>().ok())
                    .sum::<u128>()
            })
            .unwrap_or(0)
    };

    let pre = balance_of(&meta.pre_token_balances);
    let post = balance_of(&meta.post_token_balances);
    let received = post.saturating_sub(pre) as u64;

    if received < plan.quote.min_net_out {
        return Err(DirectSwapError::OutputNotReceived {
            signature: signature.to_owned(),
            expected_minimum: plan.quote.min_net_out,
            received,
        });
    }

    Ok(Receipt {
        received,
        exact: true,
        network_fee_lamports: meta.fee,
        slot: details.slot,
    })
}

/// Measure a native-SOL output, preferring the exact inner-instruction transfer
/// over the lamport delta.
fn native_receipt(
    signature: &str,
    owner: &Pubkey,
    plan: &SwapPlan,
    details: &TransactionDetails,
    meta: &crate::chains::solana::rpc::types::TransactionMeta,
) -> DirectSwapResult<Receipt> {
    if let Some(gross) = swap_output_transfer_amount(meta, plan.output_account) {
        // The output-side platform fee, when there is one, is transferred OUT of
        // the WSOL account before it is closed -- so the swap's own inner
        // transfer measures the GROSS pool output, and the figure that matches
        // `min_net_out` is that amount minus the fee.
        let received = gross.saturating_sub(plan.quote.fee.amount);
        if received >= plan.quote.min_net_out {
            return Ok(Receipt {
                received,
                exact: true,
                network_fee_lamports: meta.fee,
                slot: details.slot,
            });
        }
        // Below the chain-guaranteed floor, which the pool programme itself
        // enforced -- so the SWAP is not what is wrong here, the measurement is:
        // this path identifies one transfer among many by its destination, and a
        // venue that pays out in a shape this does not recognise would land
        // exactly here. Distrust the figure rather than the swap. Failing a
        // confirmed sell on a heuristic reading is the false alarm this module
        // exists to avoid; the token path may fail hard because a balance delta
        // cannot be misidentified, and this reading can.
        crate::logger::warning(
            crate::logger::LogTag::Swap,
            &format!(
                "Swap {signature} measured {received} raw units from its inner transfer, below \
                 the chain-guaranteed {}; the reading is untrustworthy, falling back to the \
                 lamport delta",
                plan.quote.min_net_out
            ),
        );
    }

    // Fall back to the lamport delta, read at the OWNER's own index rather than
    // assumed to be index 0 -- that assumption only ever held because the owner
    // happens to be the fee payer today. Not finding the owner at all is a decode
    // failure, never a silent zero: a wrong index here would misattribute
    // someone else's balance change as the swap's proceeds.
    let owner_index =
        account_index_of(details, owner).ok_or_else(|| DirectSwapError::TransactionFailed {
            signature: signature.to_owned(),
            detail: "the owner account could not be located in the transaction's own account \
                     keys, so the native-SOL lamport delta cannot be attributed"
                .to_owned(),
        })?;
    let pre = meta.pre_balances.get(owner_index).copied().unwrap_or(0);
    let post = meta.post_balances.get(owner_index).copied().unwrap_or(0);
    // Never used to FAIL a swap the chain already accepted -- a false alarm here
    // would report a successful sell as a failure -- so once the owner's index is
    // known, this path only reports.
    let gained = post.saturating_sub(pre).saturating_add(meta.fee);
    Ok(Receipt {
        received: gained,
        exact: false,
        network_fee_lamports: meta.fee,
        slot: details.slot,
    })
}

/// The index of `account` in the transaction's own account list, or `None` if it
/// cannot be located -- which is a decode failure, not license to assume index 0.
fn account_index_of(details: &TransactionDetails, account: &Pubkey) -> Option<usize> {
    account_keys_from_message(&details.transaction.message)
        .into_iter()
        .position(|key| key == account.to_string())
}

/// Account keys from a `jsonParsed` transaction message, which encodes them
/// either as bare strings or as `{ pubkey, signer, writable, source }` objects
/// depending on the node and transaction version.
fn account_keys_from_message(message: &serde_json::Value) -> Vec<String> {
    let Some(array) = message.get("accountKeys").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    array
        .iter()
        .filter_map(|entry| {
            entry
                .as_str()
                .map(str::to_owned)
                .or_else(|| entry.get("pubkey")?.as_str().map(str::to_owned))
        })
        .collect()
}

/// Find the SPL token transfer inside `meta.innerInstructions` whose destination
/// is `output_account`, and return its raw amount. This is the swap program's own
/// CPI paying the pool's proceeds into the WSOL account, before that account is
/// closed -- the one place a native-SOL output is still measurable exactly.
fn swap_output_transfer_amount(
    meta: &crate::chains::solana::rpc::types::TransactionMeta,
    output_account: Pubkey,
) -> Option<u64> {
    let groups = meta.inner_instructions.as_ref()?;
    let output_str = output_account.to_string();

    // Every lookup below SKIPS a malformed entry rather than abandoning the
    // search. A `?` here would let one unparsable inner instruction — a group
    // shaped differently by one node, an instruction this venue does not care
    // about — silently discard the exact measurement for the whole transaction
    // and drop the receipt back to the lamport-delta estimate.
    for group in groups {
        let Some(instructions) = group.get("instructions").and_then(|v| v.as_array()) else {
            continue;
        };
        for ix in instructions {
            let Some(parsed) = ix.get("parsed") else {
                continue;
            };
            let kind = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if kind != "transfer" && kind != "transferChecked" {
                continue;
            }
            let Some(info) = parsed.get("info") else {
                continue;
            };
            if info.get("destination").and_then(|v| v.as_str()) != Some(output_str.as_str()) {
                continue;
            }
            let amount = if kind == "transferChecked" {
                info.get("tokenAmount")
                    .and_then(|v| v.get("amount"))
                    .and_then(|v| v.as_str())
            } else {
                info.get("amount").and_then(|v| v.as_str())
            };
            if let Some(amount) = amount.and_then(|a| a.parse::<u64>().ok()) {
                return Some(amount);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::pools::types::ProgramKind;
    use crate::chains::solana::rpc::types::{
        TokenBalance, TransactionData, TransactionMeta, UiTokenAmount,
    };
    use crate::chains::solana::swaps::direct::fee::PlatformFee;
    use crate::chains::solana::swaps::direct::intent::wsol_mint;
    use crate::chains::solana::swaps::direct::quote::DirectQuote;
    use serde_json::json;

    fn base_quote(output_mint: Pubkey, min_net_out: u64, fee_amount: u64) -> DirectQuote {
        DirectQuote {
            pool: Pubkey::new_unique(),
            program: ProgramKind::RaydiumCpmm,
            input_mint: Pubkey::new_unique(),
            output_mint,
            amount_in: 1_000_000,
            swap_amount_in: 1_000_000,
            expected_out: min_net_out + fee_amount,
            min_out: min_net_out + fee_amount,
            expected_net_out: min_net_out,
            min_net_out,
            fee: PlatformFee {
                side: crate::chains::solana::swaps::direct::fee::FeeSide::Output,
                amount: fee_amount,
                mint: Some(wsol_mint()),
                destination: Some(Pubkey::new_unique()),
                decimals: 9,
            },
            lp_fee: 0,
            price_impact_pct: 0.0,
            slippage_bps: 100,
        }
    }

    fn plan(output_account: Pubkey, output_is_native: bool, quote: DirectQuote) -> SwapPlan {
        SwapPlan {
            instructions: Vec::new(),
            venue_compute_units: 0,
            input_account: Pubkey::new_unique(),
            output_account,
            output_is_native,
            quote,
        }
    }

    fn details(message: serde_json::Value, meta: TransactionMeta) -> TransactionDetails {
        TransactionDetails {
            slot: 42,
            transaction: TransactionData {
                message,
                signatures: vec!["sig".to_owned()],
            },
            meta: Some(meta),
            block_time: None,
        }
    }

    fn empty_meta() -> TransactionMeta {
        TransactionMeta {
            err: None,
            pre_balances: vec![],
            post_balances: vec![],
            pre_token_balances: None,
            post_token_balances: None,
            fee: 5_000,
            compute_units_consumed: None,
            log_messages: None,
            inner_instructions: None,
        }
    }

    #[test]
    fn a_token_output_is_measured_exactly_from_the_balance_delta() {
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let quote = base_quote(mint, 900_000, 0);
        let plan = plan(Pubkey::new_unique(), false, quote);

        let mut meta = empty_meta();
        meta.pre_token_balances = Some(vec![token_balance(&owner, &mint, "0")]);
        meta.post_token_balances = Some(vec![token_balance(&owner, &mint, "1000000")]);
        let details = details(json!({ "accountKeys": [owner.to_string()] }), meta);

        let receipt = receipt_from_transaction("sig", &owner, &plan, &details).unwrap();
        assert_eq!(receipt.received, 1_000_000);
        assert!(receipt.exact);
    }

    fn token_balance(owner: &Pubkey, mint: &Pubkey, amount: &str) -> TokenBalance {
        TokenBalance {
            account_index: 0,
            mint: mint.to_string(),
            owner: Some(owner.to_string()),
            program_id: None,
            ui_token_amount: UiTokenAmount {
                amount: amount.to_owned(),
                decimals: 6,
                ui_amount: None,
                ui_amount_string: None,
            },
        }
    }

    #[test]
    fn a_token_output_below_the_guaranteed_minimum_is_output_not_received() {
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let quote = base_quote(mint, 900_000, 0);
        let plan = plan(Pubkey::new_unique(), false, quote);

        let mut meta = empty_meta();
        meta.pre_token_balances = Some(vec![token_balance(&owner, &mint, "0")]);
        meta.post_token_balances = Some(vec![token_balance(&owner, &mint, "500000")]);
        let details = details(json!({ "accountKeys": [owner.to_string()] }), meta);

        assert!(matches!(
            receipt_from_transaction("sig", &owner, &plan, &details),
            Err(DirectSwapError::OutputNotReceived { .. })
        ));
    }

    #[test]
    fn a_native_output_is_measured_exactly_from_the_inner_transfer_net_of_the_fee() {
        let owner = Pubkey::new_unique();
        let output_account = Pubkey::new_unique();
        let quote = base_quote(wsol_mint(), 990_000, 5_000);
        let plan = plan(output_account, true, quote);

        let mut meta = empty_meta();
        meta.inner_instructions = Some(vec![json!({
            "index": 0,
            "instructions": [{
                "parsed": {
                    "type": "transfer",
                    "info": {
                        "source": Pubkey::new_unique().to_string(),
                        "destination": output_account.to_string(),
                        "amount": "995000",
                    }
                }
            }]
        })]);
        let details = details(json!({ "accountKeys": [owner.to_string()] }), meta);

        let receipt = receipt_from_transaction("sig", &owner, &plan, &details).unwrap();
        assert!(receipt.exact);
        assert_eq!(
            receipt.received, 990_000,
            "the pool's inner transfer minus the output-side platform fee"
        );
    }

    #[test]
    fn an_inner_transfer_below_the_guaranteed_floor_distrusts_itself_rather_than_failing_the_swap()
    {
        // The pool programme enforced `min_out` on chain, so a reading UNDER the
        // guaranteed floor means this heuristic picked the wrong transfer, not
        // that the swap short-changed the wallet. Failing a confirmed sell on a
        // misread would be the false alarm this module exists to avoid.
        let owner = Pubkey::new_unique();
        let output_account = Pubkey::new_unique();
        let quote = base_quote(wsol_mint(), 990_000, 5_000);
        let plan = plan(output_account, true, quote);

        let mut meta = empty_meta();
        meta.pre_balances = vec![1_000_000_000];
        meta.post_balances = vec![1_000_990_000];
        meta.inner_instructions = Some(vec![json!({
            "index": 0,
            "instructions": [{
                "parsed": {
                    "type": "transfer",
                    "info": {
                        "source": Pubkey::new_unique().to_string(),
                        "destination": output_account.to_string(),
                        "amount": "1",
                    }
                }
            }]
        })]);
        let details = details(json!({ "accountKeys": [owner.to_string()] }), meta);

        let receipt = receipt_from_transaction("sig", &owner, &plan, &details)
            .expect("a confirmed swap is never failed by a distrusted measurement");
        assert!(
            !receipt.exact,
            "the fallback reports an estimate, not a measurement"
        );
    }

    #[test]
    fn a_malformed_inner_instruction_does_not_abandon_the_search_for_the_real_transfer() {
        let owner = Pubkey::new_unique();
        let output_account = Pubkey::new_unique();
        let quote = base_quote(wsol_mint(), 990_000, 5_000);
        let plan = plan(output_account, true, quote);

        let mut meta = empty_meta();
        meta.inner_instructions = Some(vec![
            json!({ "index": 0 }),
            json!({
                "index": 1,
                "instructions": [
                    { "parsed": { "type": "transfer", "info": { "amount": "7" } } },
                    {
                        "parsed": {
                            "type": "transferChecked",
                            "info": {
                                "destination": output_account.to_string(),
                                "tokenAmount": { "amount": "995000" },
                            }
                        }
                    },
                ]
            }),
        ]);
        let details = details(json!({ "accountKeys": [owner.to_string()] }), meta);

        let receipt = receipt_from_transaction("sig", &owner, &plan, &details).unwrap();
        assert!(receipt.exact);
        assert_eq!(receipt.received, 990_000);
    }

    #[test]
    fn a_native_output_falls_back_to_the_lamport_delta_when_inner_instructions_are_absent() {
        let owner = Pubkey::new_unique();
        let output_account = Pubkey::new_unique();
        let quote = base_quote(wsol_mint(), 900_000, 0);
        let plan = plan(output_account, true, quote);

        let mut meta = empty_meta();
        meta.pre_balances = vec![1_000_000_000];
        meta.post_balances = vec![1_950_000_000];
        let details = details(json!({ "accountKeys": [owner.to_string()] }), meta);

        let receipt = receipt_from_transaction("sig", &owner, &plan, &details).unwrap();
        assert!(!receipt.exact, "no inner transfer to measure exactly");
        assert_eq!(
            receipt.received, 950_005_000,
            "lamport delta plus the fee paid"
        );
    }

    #[test]
    fn a_transaction_whose_meta_carries_an_error_is_transaction_failed_before_anything_else() {
        let owner = Pubkey::new_unique();
        let quote = base_quote(wsol_mint(), 900_000, 0);
        let plan = plan(Pubkey::new_unique(), true, quote);

        let mut meta = empty_meta();
        meta.err = Some(json!({ "InstructionError": [0, "Custom"] }));
        let details = details(json!({ "accountKeys": [owner.to_string()] }), meta);

        assert!(matches!(
            receipt_from_transaction("sig", &owner, &plan, &details),
            Err(DirectSwapError::TransactionFailed { .. })
        ));
    }

    #[test]
    fn a_missing_owner_in_the_account_keys_fails_cleanly_rather_than_reading_index_zero() {
        let owner = Pubkey::new_unique();
        let stranger = Pubkey::new_unique();
        let quote = base_quote(wsol_mint(), 0, 0);
        let plan = plan(Pubkey::new_unique(), true, quote);

        let mut meta = empty_meta();
        // Index 0 belongs to someone else entirely; a positional read here would
        // silently report their balance change as the owner's.
        meta.pre_balances = vec![5_000_000_000];
        meta.post_balances = vec![9_000_000_000];
        let details = details(json!({ "accountKeys": [stranger.to_string()] }), meta);

        assert!(matches!(
            receipt_from_transaction("sig", &owner, &plan, &details),
            Err(DirectSwapError::TransactionFailed { .. })
        ));
    }
}
