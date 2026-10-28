// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! What a simulated run's inner instructions do with the wallet: the lamports
//! it sends out, where they end up, and any control over its accounts it
//! hands to someone else.

use std::collections::HashMap;

use serde_json::Value;

use crate::chains::solana::access::AccountControl;
use crate::chains::solana::constants::{
    SPL_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID,
};
use crate::chains::solana::swaps::revenue::FEE_WALLET;

/// What a simulated swap would move out of the wallet that the trade itself
/// does not explain.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SwapCostAssessment {
    /// Lamports parked in accounts the wallet can never close: owned by a
    /// third-party program, or token accounts whose authority is not the
    /// wallet. This is money spent, not deposited.
    pub unrecoverable_lamports: u64,
    /// Rent for token accounts the wallet itself controls. Reclaimed on close,
    /// so it is reported for transparency but never blocks a trade.
    pub recoverable_rent_lamports: u64,
    /// The program or token-account authority that ends up holding the
    /// unrecoverable lamports, when one does. With several, the one that took
    /// the most.
    pub venue_program: Option<String>,
    /// Accounts the wallet funded whose owner the run alone does not
    /// establish, as `(address, lamports)`. Resolved against chain state by
    /// [`attribute_unresolved`] before any verdict is reached.
    pub unresolved: Vec<(String, u64)>,
    /// Lamports the wallet sends to plain addresses — system-owned accounts or
    /// accounts that do not exist — as `(destination, lamports)`. Gone like
    /// unrecoverable rent, though no program holds them.
    pub paid_out: Vec<(String, u64)>,
    /// Control over the wallet or its token accounts the run hands to someone
    /// else, described for the refusal. Never acceptable in a swap, whatever
    /// the amount.
    pub grants: Vec<String>,
    /// Lamports the run pays the platform fee wallet.
    pub fee_wallet_lamports: u64,
}

impl SwapCostAssessment {
    /// Whether anything at all left the wallet outside the trade.
    pub(super) fn is_empty(&self) -> bool {
        self.unrecoverable_lamports == 0
            && self.recoverable_rent_lamports == 0
            && self.unresolved.is_empty()
            && self.paid_out.is_empty()
            && self.grants.is_empty()
            && self.fee_wallet_lamports == 0
    }

    /// Lamports the run spends outside the trade that the wallet never gets
    /// back: unrecoverable rent plus every payment to a plain address.
    pub fn extra_cost_lamports(&self) -> u64 {
        self.paid_out
            .iter()
            .fold(self.unrecoverable_lamports, |total, (_, lamports)| {
                total.saturating_add(*lamports)
            })
    }

    /// The address that took the largest single share of the extra cost: the
    /// venue charged unrecoverable rent, or the largest payment's destination.
    pub(super) fn largest_charge(&self) -> Option<&str> {
        let venue = self
            .venue_program
            .as_deref()
            .map(|venue| (venue, self.unrecoverable_lamports));
        let payment = self
            .paid_out
            .iter()
            .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
            .map(|(destination, lamports)| (destination.as_str(), *lamports));
        match (venue, payment) {
            (Some(venue), Some(payment)) if payment.1 > venue.1 => Some(payment.0),
            (Some(venue), _) => Some(venue.0),
            (None, payment) => payment.map(|(destination, _)| destination),
        }
    }
}

fn is_token_program(program: &str) -> bool {
    program == SPL_TOKEN_PROGRAM_ID || program == TOKEN_2022_PROGRAM_ID
}

/// Read one parsed instruction's `info` field.
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

/// Add `lamports` to the entry of `address`, keeping first-seen order.
fn add_to(entries: &mut Vec<(String, u64)>, address: &str, lamports: u64) {
    match entries.iter_mut().find(|(entry, _)| entry == address) {
        Some((_, total)) => *total = total.saturating_add(lamports),
        None => entries.push((address.to_owned(), lamports)),
    }
}

/// Which program a parsed inner instruction belongs to, as far as this
/// module reads any.
enum Program {
    System,
    Token,
}

fn program_of(instruction: &Value) -> Option<Program> {
    let name = instruction.get("program").and_then(Value::as_str);
    let id = instruction.get("programId").and_then(Value::as_str);
    if name == Some("system") || id == Some(SYSTEM_PROGRAM_ID) {
        return Some(Program::System);
    }
    if matches!(name, Some("spl-token" | "spl-token-2022")) || id.is_some_and(is_token_program) {
        return Some(Program::Token);
    }
    None
}

/// A token account the wallet funded inside the run.
struct Created {
    account: String,
    lamports: u64,
    program: String,
}

/// Classify every lamport `wallet` sends out inside `groups`, the
/// `innerInstructions` of a simulated run, and every grant of control over
/// the wallet's accounts.
///
/// System shapes:
///
/// * `createAccount` — the account's owner is stated outright. A token
///   account is settled by the authority its initialization names.
/// * `transfer` followed by `assign` — how a program funds an account it is
///   about to take over, and the shape a venue deposit actually takes.
/// * `transfer` with no `assign` — the destination already existed; its owner
///   is a fact about chain state, recorded in `unresolved` for the caller to
///   look up rather than guessed at here. A transfer to the fee wallet is the
///   platform fee.
///
/// Token shapes: `initializeAccount*` names a created account's authority, a
/// `closeAccount` returns its lamports, and `approve`, `setAuthority` by the
/// wallet, or a system `assign` of the wallet itself, are grants.
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
    let mut created: Vec<Created> = Vec::new();
    let mut initialized: HashMap<String, String> = HashMap::new();
    let mut closed: HashMap<String, String> = HashMap::new();
    // Approvals by the wallet not yet revoked, as `(source, description)`.
    let mut approvals: Vec<(String, String)> = Vec::new();

    for instruction in groups
        .iter()
        .filter_map(|group| group.get("instructions").and_then(Value::as_array))
        .flatten()
    {
        let Some(program) = program_of(instruction) else {
            continue;
        };
        let Some(parsed) = instruction.get("parsed") else {
            continue;
        };
        let (Some(kind), Some(info)) = (
            parsed.get("type").and_then(Value::as_str),
            parsed.get("info"),
        ) else {
            continue;
        };

        match (program, kind) {
            (Program::System, "createAccount" | "createAccountWithSeed") => {
                if info_str(info, "source") != Some(wallet) {
                    continue;
                }
                let lamports = info_lamports(info);
                let account = info_str(info, "newAccount").unwrap_or_default().to_owned();
                match info_str(info, "owner") {
                    Some(owner) if is_token_program(owner) => created.push(Created {
                        account,
                        lamports,
                        program: owner.to_owned(),
                    }),
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
            (Program::System, "transfer") => {
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
                if destination == FEE_WALLET {
                    assessment.fee_wallet_lamports =
                        assessment.fee_wallet_lamports.saturating_add(lamports);
                } else if let Some(account) = created.iter_mut().find(|c| c.account == destination)
                {
                    account.lamports = account.lamports.saturating_add(lamports);
                } else {
                    add_to(&mut funded, destination, lamports);
                }
            }
            (Program::System, "assign") => {
                let (Some(account), Some(owner)) =
                    (info_str(info, "account"), info_str(info, "owner"))
                else {
                    continue;
                };
                if account == wallet {
                    assessment
                        .grants
                        .push(format!("the wallet is assigned to program {owner}"));
                    continue;
                }
                if owner == SYSTEM_PROGRAM_ID {
                    continue;
                }
                let Some(position) = funded.iter().position(|(a, _)| a == account) else {
                    continue;
                };
                let (account, lamports) = funded.remove(position);
                if is_token_program(owner) {
                    created.push(Created {
                        account,
                        lamports,
                        program: owner.to_owned(),
                    });
                } else {
                    *by_program.entry(owner.to_owned()).or_default() += lamports;
                }
            }
            (Program::Token, "initializeAccount" | "initializeAccount2" | "initializeAccount3") => {
                if let (Some(account), Some(owner)) =
                    (info_str(info, "account"), info_str(info, "owner"))
                {
                    initialized.insert(account.to_owned(), owner.to_owned());
                }
            }
            (Program::Token, "closeAccount") => {
                if let (Some(account), Some(destination)) =
                    (info_str(info, "account"), info_str(info, "destination"))
                {
                    closed.insert(account.to_owned(), destination.to_owned());
                }
            }
            (Program::Token, "approve" | "approveChecked") => {
                if info_str(info, "owner") != Some(wallet) {
                    continue;
                }
                let source = info_str(info, "source").unwrap_or_default();
                let delegate = info_str(info, "delegate").unwrap_or_default();
                approvals.push((
                    source.to_owned(),
                    format!("token account {source} is approved to delegate {delegate}"),
                ));
            }
            (Program::Token, "revoke") => {
                if let Some(source) = info_str(info, "source") {
                    approvals.retain(|(approved, _)| approved != source);
                }
            }
            (Program::Token, "setAuthority") => {
                if info_str(info, "authority") != Some(wallet) {
                    continue;
                }
                let new_authority = info_str(info, "newAuthority");
                if new_authority == Some(wallet) {
                    continue;
                }
                let target = info_str(info, "account")
                    .or_else(|| info_str(info, "mint"))
                    .unwrap_or_default();
                assessment.grants.push(format!(
                    "authority over {target} moves from the wallet to {}",
                    new_authority.unwrap_or("nobody")
                ));
            }
            _ => {}
        }
    }

    assessment
        .grants
        .extend(approvals.into_iter().map(|(_, description)| description));

    for account in created {
        match closed.get(&account.account) {
            // Closed back to the wallet inside the run: nothing left it.
            Some(destination) if destination == wallet => {}
            Some(destination) if destination == FEE_WALLET => {
                assessment.fee_wallet_lamports = assessment
                    .fee_wallet_lamports
                    .saturating_add(account.lamports);
            }
            Some(destination) => add_to(&mut assessment.paid_out, destination, account.lamports),
            None => match initialized.get(&account.account) {
                Some(authority) if authority == wallet => {
                    assessment.recoverable_rent_lamports = assessment
                        .recoverable_rent_lamports
                        .saturating_add(account.lamports);
                }
                Some(authority) => {
                    *by_program.entry(authority.clone()).or_default() += account.lamports;
                }
                // Never initialized: no authority can close it, the wallet least
                // of all.
                None => *by_program.entry(account.program).or_default() += account.lamports,
            },
        }
    }

    for (account, lamports) in funded {
        match initialized.get(&account) {
            Some(authority) if authority == wallet => {
                assessment.recoverable_rent_lamports = assessment
                    .recoverable_rent_lamports
                    .saturating_add(lamports);
            }
            Some(authority) => *by_program.entry(authority.clone()).or_default() += lamports,
            None => assessment.unresolved.push((account, lamports)),
        }
    }

    finalise(&mut assessment, by_program);
    assessment
}

/// Fold per-venue charges into the assessment, naming the biggest as the
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

/// Settle the accounts the run alone could not classify, given who controls
/// each one on chain. An account missing from `owners` does not exist yet, so
/// it cannot hold anyone's state.
///
/// * a token account whose authority is `wallet` — recoverable rent;
/// * a token account of any other authority — unrecoverable, charged to it;
/// * a system-owned or missing account — a payment, in `paid_out`;
/// * an account of any other program — unrecoverable, charged to it.
pub fn attribute_unresolved(
    assessment: &mut SwapCostAssessment,
    wallet: &str,
    owners: &HashMap<String, AccountControl>,
) {
    let unresolved = std::mem::take(&mut assessment.unresolved);
    let mut by_program: HashMap<String, u64> = HashMap::new();
    for (account, lamports) in unresolved {
        match owners.get(&account) {
            Some(AccountControl::TokenAccount { authority, .. }) if authority == wallet => {
                assessment.recoverable_rent_lamports = assessment
                    .recoverable_rent_lamports
                    .saturating_add(lamports);
            }
            Some(AccountControl::TokenAccount { authority, .. }) => {
                *by_program.entry(authority.clone()).or_default() += lamports;
            }
            Some(AccountControl::Program(program)) if program != SYSTEM_PROGRAM_ID => {
                *by_program.entry(program.clone()).or_default() += lamports;
            }
            Some(AccountControl::Program(_)) | None => {
                add_to(&mut assessment.paid_out, &account, lamports);
            }
        }
    }
    // Re-fold so a venue charged here merges with one charged from the
    // instructions, and the named venue stays the largest overall.
    let mut merged = by_program;
    if let Some(existing) = assessment.venue_program.take() {
        *merged.entry(existing).or_default() += assessment.unrecoverable_lamports;
        assessment.unrecoverable_lamports = 0;
    }
    finalise(assessment, merged);
}
