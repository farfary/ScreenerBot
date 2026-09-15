//! What a built swap would spend BEYOND the trade, and the refusal when that
//! cost is out of proportion to it.
//!
//! Slippage protects the OUTPUT of a swap: it guarantees a minimum number of
//! tokens and nothing else. It is therefore blind to lamports that leave the
//! wallet ALONGSIDE the swap, which is exactly what a route can do — a venue
//! that keeps per-trader state on chain has the trader's own transaction create
//! and fund that account, and only that venue's program can ever close it. The
//! quote does not mention it, the price impact does not move, and every
//! slippage check ever written passes. On a small trade the deposit can be
//! several times the trade itself.
//!
//! This module is the systematic answer, and it deliberately knows nothing
//! about any particular venue: it simulates the transaction a router built,
//! reads the lamport movements the run would actually perform, and classifies
//! every lamport that leaves the wallet as either
//!
//! * **recoverable** — rent for one of the wallet's OWN token accounts, which
//!   the wallet can close later (the WSOL sweep and wallet cleanup do exactly
//!   that), or
//! * **unrecoverable** — lamports parked in an account owned by somebody else's
//!   program, which this wallet can never close.
//!
//! Anything unrecoverable beyond the configured allowance refuses the
//! transaction BEFORE it is signed, so the refusal costs nothing and the caller
//! is free to route the trade another way. Because the rule is expressed in
//! lamports and account ownership rather than in venue names, a venue that
//! starts charging a deposit tomorrow, or an aggregator added next year, is
//! covered the day it appears.
//!
//! **Stability rule: only a node that ANSWERED may block a trade.** A transport
//! failure, a node that cannot simulate, or a response we cannot parse leaves
//! the swap to proceed exactly as it did before this module existed. An
//! unreadable node is not evidence of a cost.

use std::collections::HashMap;
use std::str::FromStr;

use serde_json::Value;

use crate::chains::solana::constants::{
    SPL_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID,
};
use crate::chains::solana::rpc::RpcClientMethods;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::{Error, Result};
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::swaps::Quote;

/// What a simulated swap would move out of the wallet that the trade itself
/// does not explain.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SwapCostAssessment {
    /// Lamports parked in accounts owned by a third-party program. The wallet
    /// has no authority to close these, so this is money spent, not deposited.
    pub unrecoverable_lamports: u64,
    /// Rent for token accounts the wallet itself owns. Reclaimed on close, so
    /// it is reported for transparency but never blocks a trade.
    pub recoverable_rent_lamports: u64,
    /// The program that ends up owning the funded account, when one does. With
    /// several, the one that took the most.
    pub venue_program: Option<String>,
    /// Accounts the wallet funded whose owner the instructions alone do not
    /// establish, as `(address, lamports)`. Resolved against chain state by
    /// [`attribute_unresolved`] before any verdict is reached.
    pub unresolved: Vec<(String, u64)>,
}

impl SwapCostAssessment {
    /// Whether anything at all left the wallet outside the trade.
    fn is_empty(&self) -> bool {
        self.unrecoverable_lamports == 0
            && self.recoverable_rent_lamports == 0
            && self.unresolved.is_empty()
    }
}

/// The caller's tolerance, read from `swaps.cost_guard`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CostGuardSettings {
    pub enabled: bool,
    pub max_extra_cost_pct: f64,
    pub always_allow_below_lamports: u64,
    pub retry_excluding_venue: bool,
}

impl CostGuardSettings {
    /// The live configuration.
    pub fn current() -> Self {
        with_config(|cfg| Self {
            enabled: cfg.swaps.cost_guard.enabled,
            max_extra_cost_pct: cfg.swaps.cost_guard.max_extra_cost_pct,
            always_allow_below_lamports: cfg.swaps.cost_guard.always_allow_below_lamports,
            retry_excluding_venue: cfg.swaps.cost_guard.retry_excluding_venue,
        })
    }
}

/// An owner program whose accounts the WALLET can close again, so rent paid
/// into one is a deposit rather than a cost. Only the two token programs
/// qualify: a token account's close authority is its owner, which is this
/// wallet for every account a swap creates for it.
fn owner_returns_rent(owner: &str) -> bool {
    owner == SPL_TOKEN_PROGRAM_ID || owner == TOKEN_2022_PROGRAM_ID
}

/// Read one parsed system-program instruction's `info` field.
fn info_str<'a>(info: &'a Value, key: &str) -> Option<&'a str> {
    info.get(key).and_then(Value::as_str)
}

/// Lamports out of a parsed `info`, tolerating the string form some nodes emit.
fn info_lamports(info: &Value) -> u64 {
    match info.get("lamports") {
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
        Some(Value::String(s)) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

/// Classify every lamport `wallet` sends out inside `groups`, the
/// `innerInstructions` of a simulated run.
///
/// Only the SYSTEM program moves bare lamports, so only its parsed
/// instructions are read. Three shapes matter:
///
/// * `createAccount` — the account's owner is stated outright.
/// * `transfer` followed by `assign` — how a program funds an account it is
///   about to take over, and the shape a venue deposit actually takes.
/// * `transfer` with no `assign` — the destination already existed; its owner
///   is a fact about chain state, recorded in `unresolved` for the caller to
///   look up rather than guessed at here.
///
/// Anything this function cannot attribute is reported, never assumed: a cost
/// it cannot see is a gap in coverage, but a cost it invents would block real
/// trades.
pub fn assess_inner_instructions(wallet: &str, groups: &[Value]) -> SwapCostAssessment {
    let mut assessment = SwapCostAssessment::default();
    let mut by_program: HashMap<String, u64> = HashMap::new();
    // Accounts this transaction funded out of the wallet, still unclaimed by an
    // `assign`. Insertion order is kept so the report is deterministic.
    let mut funded: Vec<(String, u64)> = Vec::new();

    for instruction in groups
        .iter()
        .filter_map(|group| group.get("instructions").and_then(Value::as_array))
        .flatten()
    {
        let is_system = instruction.get("program").and_then(Value::as_str) == Some("system")
            || instruction.get("programId").and_then(Value::as_str) == Some(SYSTEM_PROGRAM_ID);
        if !is_system {
            continue;
        }
        let Some(parsed) = instruction.get("parsed") else {
            continue;
        };
        let (Some(kind), Some(info)) = (
            parsed.get("type").and_then(Value::as_str),
            parsed.get("info"),
        ) else {
            continue;
        };

        match kind {
            "createAccount" | "createAccountWithSeed" => {
                if info_str(info, "source") != Some(wallet) {
                    continue;
                }
                let lamports = info_lamports(info);
                let account = info_str(info, "newAccount").unwrap_or_default().to_owned();
                match info_str(info, "owner") {
                    Some(owner) if owner_returns_rent(owner) => {
                        assessment.recoverable_rent_lamports = assessment
                            .recoverable_rent_lamports
                            .saturating_add(lamports);
                    }
                    // Created as a plain system account: whoever assigns it next
                    // owns it, exactly like the transfer case below.
                    Some(SYSTEM_PROGRAM_ID) | None => {
                        if lamports > 0 && !account.is_empty() {
                            funded.push((account, lamports));
                        }
                    }
                    Some(owner) => {
                        *by_program.entry(owner.to_owned()).or_default() += lamports;
                    }
                }
            }
            "transfer" => {
                if info_str(info, "source") != Some(wallet) {
                    continue;
                }
                let lamports = info_lamports(info);
                let Some(destination) = info_str(info, "destination") else {
                    continue;
                };
                if lamports == 0 {
                    continue;
                }
                match funded.iter_mut().find(|(a, _)| a == destination) {
                    Some((_, total)) => *total = total.saturating_add(lamports),
                    None => funded.push((destination.to_owned(), lamports)),
                }
            }
            "assign" => {
                let (Some(account), Some(owner)) =
                    (info_str(info, "account"), info_str(info, "owner"))
                else {
                    continue;
                };
                let Some(position) = funded.iter().position(|(a, _)| a == account) else {
                    continue;
                };
                let (_, lamports) = funded.remove(position);
                if owner_returns_rent(owner) {
                    assessment.recoverable_rent_lamports = assessment
                        .recoverable_rent_lamports
                        .saturating_add(lamports);
                } else if owner != SYSTEM_PROGRAM_ID {
                    *by_program.entry(owner.to_owned()).or_default() += lamports;
                }
            }
            _ => {}
        }
    }

    assessment.unresolved = funded;
    finalise(&mut assessment, by_program);
    assessment
}

/// Fold per-program charges into the assessment, naming the biggest as the
/// venue responsible.
fn finalise(assessment: &mut SwapCostAssessment, by_program: HashMap<String, u64>) {
    let mut charged: Vec<(String, u64)> = by_program.into_iter().collect();
    // Largest first, then by address, so the named venue never depends on hash
    // iteration order.
    charged.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    for (program, lamports) in &charged {
        assessment.unrecoverable_lamports =
            assessment.unrecoverable_lamports.saturating_add(*lamports);
        if assessment.venue_program.is_none() {
            assessment.venue_program = Some(program.clone());
        }
    }
}

/// Settle the accounts the instructions alone could not classify, given the
/// owner each one has on chain (`None` for an account that does not exist yet,
/// which cannot be a pre-existing third-party account).
pub fn attribute_unresolved(assessment: &mut SwapCostAssessment, owners: &HashMap<String, String>) {
    let unresolved = std::mem::take(&mut assessment.unresolved);
    let mut by_program: HashMap<String, u64> = HashMap::new();
    for (account, lamports) in unresolved {
        match owners.get(&account) {
            Some(owner) if owner_returns_rent(owner) => {
                assessment.recoverable_rent_lamports = assessment
                    .recoverable_rent_lamports
                    .saturating_add(lamports);
            }
            Some(owner) if owner != SYSTEM_PROGRAM_ID => {
                *by_program.entry(owner.clone()).or_default() += lamports;
            }
            // A system-owned or brand-new destination holds no program's state:
            // a tip or a plain payment, not rent locked behind someone's code.
            _ => {}
        }
    }
    // Re-fold so an owner charged here merges with one charged from the
    // instructions, and the named venue stays the largest overall.
    let mut merged = by_program;
    if let Some(existing) = assessment.venue_program.take() {
        *merged.entry(existing).or_default() += assessment.unrecoverable_lamports;
        assessment.unrecoverable_lamports = 0;
    }
    finalise(assessment, merged);
}

/// The most a trade of `trade_value_lamports` may spend on accounts it will
/// never get back.
///
/// Two dials, deliberately: a PROPORTION, so the same venue deposit that is
/// absurd on a 0.005 SOL trade is acceptable on a large one, and a FLOOR, so
/// dust never blocks a swap. A trade whose value cannot be expressed in
/// lamports (neither leg is the native asset) has only the floor.
pub fn allowance_lamports(trade_value_lamports: u64, settings: &CostGuardSettings) -> u64 {
    let proportional = if settings.max_extra_cost_pct > 0.0 {
        (trade_value_lamports as f64 * settings.max_extra_cost_pct / 100.0).round() as u64
    } else {
        0
    };
    proportional.max(settings.always_allow_below_lamports)
}

/// The trade's own size in lamports, when one of its legs is the native asset.
pub fn trade_value_lamports(quote: &Quote) -> u64 {
    let adapter = crate::chains::adapter();
    if adapter.is_native_asset(&quote.input_mint) {
        quote.input_amount
    } else if adapter.is_native_asset(&quote.output_mint) {
        quote.output_amount
    } else {
        0
    }
}

/// Simulate `transaction_base64` and refuse it if it would fail, or if it would
/// spend more of the wallet's lamports outside the trade than
/// `swaps.cost_guard` allows.
///
/// Both refusals happen before anything is signed, so neither costs a fee and
/// both are safe for the caller to answer by trying a different route.
pub async fn preflight(
    router: &'static str,
    transaction_base64: &str,
    quote: &Quote,
) -> Result<()> {
    use base64::Engine;

    use crate::chains::solana::solana_sdk::transaction::VersionedTransaction;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(transaction_base64)
        .map_err(|e| Error::Decode {
            payload: "swap transaction base64",
            detail: format!("{router}: {e}"),
        })?;
    let transaction: VersionedTransaction =
        bincode::deserialize(&bytes).map_err(|e| Error::Decode {
            payload: "swap transaction",
            detail: format!("{router}: {e}"),
        })?;

    // A node that cannot answer says nothing about this transaction. Keep the
    // trade moving rather than inventing a verdict from a transport failure.
    let outcome = match crate::chains::solana::rpc::get_rpc_client()
        .simulate_transaction(&transaction)
        .await
    {
        Ok(outcome) => outcome,
        Err(e) => {
            logger::warning(
                LogTag::Swap,
                &format!("{router} preflight simulation unavailable, proceeding: {e}"),
            );
            return Ok(());
        }
    };

    if let Some(err) = &outcome.err {
        let last_logs = outcome
            .logs
            .iter()
            .rev()
            .take(3)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ");
        return Err(Error::SimulationRejected {
            router,
            detail: format!("{err} {last_logs}").trim().to_owned(),
        });
    }

    let settings = CostGuardSettings::current();
    if !settings.enabled {
        return Ok(());
    }

    let mut assessment =
        assess_inner_instructions(&quote.wallet_address, &outcome.inner_instructions);
    if assessment.is_empty() {
        return Ok(());
    }
    if !assessment.unresolved.is_empty() {
        let owners = resolve_owners(&assessment.unresolved).await;
        attribute_unresolved(&mut assessment, &owners);
    }

    let allowance = allowance_lamports(trade_value_lamports(quote), &settings);
    if assessment.unrecoverable_lamports > allowance {
        let venue_program = assessment.venue_program.clone().unwrap_or_default();
        // Name the venue the way people and the aggregator both know it; fall
        // back to the address when the aggregator has no name for it.
        let venue = venue_display_name(&venue_program).await;
        crate::swaps::progress::report_swap_stage(crate::swaps::SwapStage::CostRejected {
            router: router.to_owned(),
            venue: venue.clone(),
            extra_lamports: assessment.unrecoverable_lamports,
        })
        .await;
        logger::warning(
            LogTag::Swap,
            &format!(
                "{router} transaction refused: it would lock {:.6} SOL in an account owned by {venue}, above the {:.6} SOL allowed for this trade",
                crate::chains::solana::constants::lamports_to_sol(
                    assessment.unrecoverable_lamports
                ),
                crate::chains::solana::constants::lamports_to_sol(allowance),
            ),
        );
        return Err(Error::SwapCostRejected {
            router,
            extra_lamports: assessment.unrecoverable_lamports,
            venue,
            venue_program,
        });
    }

    if assessment.unrecoverable_lamports > 0 {
        logger::info(
            LogTag::Swap,
            &format!(
                "{router} transaction spends {:.6} SOL outside the trade, within the {:.6} SOL allowed",
                crate::chains::solana::constants::lamports_to_sol(
                    assessment.unrecoverable_lamports
                ),
                crate::chains::solana::constants::lamports_to_sol(allowance),
            ),
        );
    }
    Ok(())
}

/// How a venue is named to the user: the aggregator's own label when it has
/// one, otherwise the program address, shortened so it fits a toast.
async fn venue_display_name(program: &str) -> String {
    if program.is_empty() {
        return "a third-party program".to_owned();
    }
    if let Some(label) =
        crate::chains::solana::swaps::routers::venue_label_for_program(program).await
    {
        return label;
    }
    match program.len() > 12 {
        true => format!("{}…{}", &program[..4], &program[program.len() - 4..]),
        false => program.to_owned(),
    }
}

/// Owners of the funded accounts the instructions left unattributed. A failed
/// read returns what it could: an account with no answer stays unattributed,
/// which can only ever make the guard more permissive.
async fn resolve_owners(unresolved: &[(String, u64)]) -> HashMap<String, String> {
    let keys: Vec<Pubkey> = unresolved
        .iter()
        .filter_map(|(address, _)| Pubkey::from_str(address).ok())
        .collect();
    if keys.is_empty() {
        return HashMap::new();
    }
    match crate::chains::solana::rpc::get_rpc_client()
        .get_multiple_accounts(&keys)
        .await
    {
        Ok(accounts) => keys
            .iter()
            .zip(accounts)
            .filter_map(|(key, account)| {
                account.map(|account| (key.to_string(), account.owner.to_string()))
            })
            .collect(),
        Err(e) => {
            logger::warning(
                LogTag::Swap,
                &format!("Could not read the owners of accounts a swap funds: {e}"),
            );
            HashMap::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const WALLET: &str = "6uodGCMLfDLfeXkyW71WpUDK1BEG19bU2EV51x2dcGMv";
    const VENUE: &str = "9H6tua7jkLhdm3w8BvgpTn5LZNU7g4ZynDmCiNN3q6Rp";
    const VENUE_ACCOUNT: &str = "GVHuUhKQYx3LfoquXK9gmCmEBUM3nMaeFHuonhYASiZt";
    const OUR_ATA: &str = "CUzfpKAieLAgVzNjXM4mH2fuh9vTrExnGKGooEUKY8TK";

    fn group(instructions: Vec<Value>) -> Value {
        json!({"index": 6, "instructions": instructions})
    }

    fn system(kind: &str, info: Value) -> Value {
        json!({
            "program": "system",
            "programId": SYSTEM_PROGRAM_ID,
            "parsed": {"type": kind, "info": info},
            "stackHeight": 2,
        })
    }

    fn transfer(source: &str, destination: &str, lamports: u64) -> Value {
        system(
            "transfer",
            json!({"source": source, "destination": destination, "lamports": lamports}),
        )
    }

    fn create_account(source: &str, new_account: &str, owner: &str, lamports: u64) -> Value {
        system(
            "createAccount",
            json!({
                "source": source,
                "newAccount": new_account,
                "owner": owner,
                "lamports": lamports,
                "space": 165,
            }),
        )
    }

    fn settings() -> CostGuardSettings {
        CostGuardSettings {
            enabled: true,
            max_extra_cost_pct: 1.0,
            always_allow_below_lamports: 100_000,
            retry_excluding_venue: true,
        }
    }

    /// The shape that cost real money: an aggregator's route transfers rent out
    /// of the trader's wallet and the venue's program then takes the account
    /// over, all inside one swap the quote described as 0.005 SOL.
    #[test]
    fn rent_a_venue_takes_over_mid_swap_is_charged_to_that_venue() {
        let groups = vec![group(vec![
            transfer(WALLET, VENUE_ACCOUNT, 13_045_440),
            system("allocate", json!({"account": VENUE_ACCOUNT, "space": 2440})),
            system("assign", json!({"account": VENUE_ACCOUNT, "owner": VENUE})),
        ])];

        let assessment = assess_inner_instructions(WALLET, &groups);

        assert_eq!(assessment.unrecoverable_lamports, 13_045_440);
        assert_eq!(assessment.venue_program.as_deref(), Some(VENUE));
        assert_eq!(assessment.recoverable_rent_lamports, 0);
        assert!(assessment.unresolved.is_empty());
    }

    #[test]
    fn rent_for_the_wallets_own_token_accounts_is_recoverable_not_a_cost() {
        let groups = vec![group(vec![
            create_account(WALLET, OUR_ATA, SPL_TOKEN_PROGRAM_ID, 1_488_440),
            create_account(WALLET, VENUE_ACCOUNT, TOKEN_2022_PROGRAM_ID, 1_539_240),
        ])];

        let assessment = assess_inner_instructions(WALLET, &groups);

        assert_eq!(assessment.unrecoverable_lamports, 0);
        assert_eq!(assessment.recoverable_rent_lamports, 3_027_680);
        assert_eq!(assessment.venue_program, None);
    }

    #[test]
    fn an_account_created_straight_into_a_third_party_program_is_charged() {
        let groups = vec![group(vec![create_account(
            WALLET,
            VENUE_ACCOUNT,
            VENUE,
            2_500_000,
        )])];

        let assessment = assess_inner_instructions(WALLET, &groups);

        assert_eq!(assessment.unrecoverable_lamports, 2_500_000);
        assert_eq!(assessment.venue_program.as_deref(), Some(VENUE));
    }

    #[test]
    fn lamports_somebody_else_pays_are_never_charged_to_this_wallet() {
        let other = "AgmLJBMDN6vCXqVqHUcgDqXKRZKPQvmdKfKhVpVLRZ2r";
        let groups = vec![group(vec![
            transfer(other, VENUE_ACCOUNT, 13_045_440),
            system("assign", json!({"account": VENUE_ACCOUNT, "owner": VENUE})),
            create_account(other, OUR_ATA, SPL_TOKEN_PROGRAM_ID, 1_488_440),
        ])];

        assert_eq!(
            assess_inner_instructions(WALLET, &groups),
            SwapCostAssessment::default()
        );
    }

    #[test]
    fn a_transfer_with_no_assign_is_reported_unresolved_rather_than_guessed_at() {
        let groups = vec![group(vec![transfer(WALLET, VENUE_ACCOUNT, 900_000)])];

        let assessment = assess_inner_instructions(WALLET, &groups);

        assert_eq!(assessment.unrecoverable_lamports, 0);
        assert_eq!(
            assessment.unresolved,
            vec![(VENUE_ACCOUNT.to_owned(), 900_000)]
        );
    }

    #[test]
    fn an_unresolved_account_is_settled_by_the_owner_it_has_on_chain() {
        let mut third_party = SwapCostAssessment {
            unresolved: vec![(VENUE_ACCOUNT.to_owned(), 900_000)],
            ..Default::default()
        };
        attribute_unresolved(
            &mut third_party,
            &HashMap::from([(VENUE_ACCOUNT.to_owned(), VENUE.to_owned())]),
        );
        assert_eq!(third_party.unrecoverable_lamports, 900_000);
        assert_eq!(third_party.venue_program.as_deref(), Some(VENUE));

        let mut token_account = SwapCostAssessment {
            unresolved: vec![(OUR_ATA.to_owned(), 1_488_440)],
            ..Default::default()
        };
        attribute_unresolved(
            &mut token_account,
            &HashMap::from([(OUR_ATA.to_owned(), SPL_TOKEN_PROGRAM_ID.to_owned())]),
        );
        assert_eq!(token_account.unrecoverable_lamports, 0);
        assert_eq!(token_account.recoverable_rent_lamports, 1_488_440);

        // A tip to a plain wallet, and an account no node knows, are both left
        // alone: neither is rent locked behind someone else's program.
        let mut plain = SwapCostAssessment {
            unresolved: vec![
                (VENUE_ACCOUNT.to_owned(), 10_000),
                (OUR_ATA.to_owned(), 20_000),
            ],
            ..Default::default()
        };
        attribute_unresolved(
            &mut plain,
            &HashMap::from([(VENUE_ACCOUNT.to_owned(), SYSTEM_PROGRAM_ID.to_owned())]),
        );
        assert_eq!(plain.unrecoverable_lamports, 0);
        assert_eq!(plain.venue_program, None);
    }

    #[test]
    fn charges_from_instructions_and_from_chain_state_merge_under_one_venue() {
        let mut assessment = SwapCostAssessment {
            unrecoverable_lamports: 13_045_440,
            venue_program: Some(VENUE.to_owned()),
            unresolved: vec![(OUR_ATA.to_owned(), 1_000_000)],
            ..Default::default()
        };

        attribute_unresolved(
            &mut assessment,
            &HashMap::from([(OUR_ATA.to_owned(), VENUE.to_owned())]),
        );

        assert_eq!(assessment.unrecoverable_lamports, 14_045_440);
        assert_eq!(assessment.venue_program.as_deref(), Some(VENUE));
    }

    #[test]
    fn the_venue_named_is_the_one_that_took_the_most() {
        let smaller = "TessVdML9pBGgG9yGks7o4HewRaXVAMuoVj4x83GLQH";
        let groups = vec![group(vec![
            create_account(WALLET, OUR_ATA, smaller, 2_000_000),
            create_account(WALLET, VENUE_ACCOUNT, VENUE, 13_045_440),
        ])];

        let assessment = assess_inner_instructions(WALLET, &groups);

        assert_eq!(assessment.unrecoverable_lamports, 15_045_440);
        assert_eq!(assessment.venue_program.as_deref(), Some(VENUE));
    }

    #[test]
    fn instructions_are_read_across_every_inner_group_and_odd_shapes_are_survived() {
        let groups = vec![
            json!({"index": 2, "instructions": []}),
            json!({"index": 3}),
            json!("not a group"),
            group(vec![
                // A token-program instruction: not a lamport movement.
                json!({
                    "program": "spl-token",
                    "programId": SPL_TOKEN_PROGRAM_ID,
                    "parsed": {"type": "transfer", "info": {"amount": "4975000"}},
                }),
                // Unparsed inner instruction (no `parsed` field at all).
                json!({"programId": SYSTEM_PROGRAM_ID, "accounts": [], "data": "3Bxs"}),
                // Lamports as a string, which some nodes emit.
                system(
                    "transfer",
                    json!({"source": WALLET, "destination": VENUE_ACCOUNT, "lamports": "13045440"}),
                ),
                system("assign", json!({"account": VENUE_ACCOUNT, "owner": VENUE})),
            ]),
        ];

        let assessment = assess_inner_instructions(WALLET, &groups);

        assert_eq!(assessment.unrecoverable_lamports, 13_045_440);
        assert_eq!(assessment.venue_program.as_deref(), Some(VENUE));
    }

    #[test]
    fn a_run_that_moves_nothing_out_of_the_wallet_costs_nothing() {
        let groups: Vec<Value> = vec![group(vec![json!({
            "program": "spl-token",
            "programId": SPL_TOKEN_PROGRAM_ID,
            "parsed": {"type": "transfer", "info": {"amount": "4975000"}},
        })])];

        let assessment = assess_inner_instructions(WALLET, &groups);

        assert!(assessment.is_empty());
        assert_eq!(assess_inner_instructions(WALLET, &[]), assessment);
    }

    #[test]
    fn the_allowance_is_a_share_of_the_trade_with_a_floor_under_it() {
        let settings = settings();
        // 1% of a 1 SOL trade, well above the floor.
        assert_eq!(allowance_lamports(1_000_000_000, &settings), 10_000_000);
        // 1% of a 0.005 SOL trade is 50,000 -- the floor wins.
        assert_eq!(allowance_lamports(5_000_000, &settings), 100_000);
        // A trade with no native leg has only the floor to go on.
        assert_eq!(allowance_lamports(0, &settings), 100_000);

        let strict = CostGuardSettings {
            max_extra_cost_pct: 0.0,
            always_allow_below_lamports: 0,
            ..settings
        };
        assert_eq!(allowance_lamports(1_000_000_000, &strict), 0);

        let generous = CostGuardSettings {
            max_extra_cost_pct: 100.0,
            ..settings
        };
        assert_eq!(allowance_lamports(5_000_000, &generous), 5_000_000);
    }

    /// The real numbers from the trade that exposed this: a 0.005 SOL buy whose
    /// route locked 0.01304544 SOL of rent. It must be refused at the default
    /// settings, and accepted once the same deposit is small against the trade.
    #[test]
    fn the_real_deposit_is_refused_on_a_small_trade_and_allowed_on_a_large_one() {
        let settings = settings();
        let deposit = 13_045_440;

        assert!(deposit > allowance_lamports(5_000_000, &settings));
        // Against 2 SOL the same one-off deposit is inside the 1% allowance.
        assert!(deposit <= allowance_lamports(2_000_000_000, &settings));
    }
}
