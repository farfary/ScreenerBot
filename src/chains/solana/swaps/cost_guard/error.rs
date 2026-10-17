// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Typed reasons the cost guard refuses to sign a third-party build.
//!
//! The variant is the refusal kind; its [`Display`](std::fmt::Display) is the
//! detail text carried into `NotSubmittedReason::BuildUnusable`.

use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// Why the static or conservation layers of the cost guard refuse a build.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BuildRefusal {
    /// The router has no registered configuration to bound its builds by.
    #[error("router {router} has no registered build expectations")]
    RouterUnregistered { router: String },
    /// The quote's own amounts cannot be expressed in the chain's integer range.
    #[error("{detail}")]
    QuoteBounds { detail: String },
    /// The quote's wallet is not an address.
    #[error("the quote's wallet {wallet} is not an address")]
    WalletNotAnAddress { wallet: String },
    /// A program or mint constant of the guard does not parse as an address.
    #[error("{detail}")]
    ConstantInvalid { detail: String },
    /// A quoted amount does not fit the range the check needs.
    #[error("the quoted {leg} exceeds the {limit}")]
    QuoteAmountOutOfRange {
        leg: &'static str,
        limit: &'static str,
    },
    /// An instruction lacks an account its kind requires.
    #[error("instruction {position} has no account {slot}, which its kind requires")]
    AccountMissing { position: usize, slot: usize },
    /// An instruction names an account index beyond the message.
    #[error("account {slot} of instruction {position} is index {index}, outside the message")]
    AccountOutsideMessage {
        position: usize,
        slot: usize,
        index: u8,
    },
    /// An instruction names an account from a lookup table no simulation resolved.
    #[error(
        "account {slot} of instruction {position} (index {index}) comes from an address \
         lookup table no simulation resolved"
    )]
    AccountUnresolved {
        position: usize,
        slot: usize,
        index: u8,
    },
    /// An account that must be the wallet is another address.
    #[error("instruction {position} names {found} as the {role}, not the wallet")]
    NotTheWallet {
        position: usize,
        found: Pubkey,
        role: &'static str,
    },
    /// An instruction names a program outside the message.
    #[error("instruction {position} names a program outside the message")]
    ProgramOutsideMessage { position: usize },
    /// An instruction's data does not decode as the kind it claims.
    #[error("instruction {position} is a malformed {kind}")]
    Malformed { position: usize, kind: &'static str },
    /// The compute-unit limit is above what the runtime grants.
    #[error(
        "instruction {position} requests {limit} compute units, above the {maximum} the runtime \
         grants"
    )]
    ComputeLimitTooHigh {
        position: usize,
        limit: u32,
        maximum: u32,
    },
    /// The compute-unit price is above what the router is configured to pay.
    #[error(
        "instruction {position} pays {price} micro-lamports per compute unit, above the \
         {maximum} the router is configured to pay"
    )]
    ComputePriceTooHigh {
        position: usize,
        price: u64,
        maximum: u64,
    },
    /// A compute-budget instruction of a kind a swap does not carry.
    #[error("instruction {position} is a compute-budget instruction a swap does not carry")]
    ComputeBudgetUnexpected { position: usize },
    /// An associated-token-account instruction that is not a create.
    #[error(
        "instruction {position} is an associated-token-account instruction other than a create"
    )]
    AssociatedTokenNotCreate { position: usize },
    /// A system instruction that is not a transfer.
    #[error("instruction {position} is a system instruction other than a transfer")]
    SystemNotTransfer { position: usize },
    /// A transfer to an address a swap has no reason to pay.
    #[error(
        "instruction {position} transfers {lamports} lamports to {destination}, which is neither \
         the wallet's wrapped-SOL account, the fee wallet nor a tip account"
    )]
    TransferToStranger {
        position: usize,
        lamports: u64,
        destination: Pubkey,
    },
    /// The transfers of one kind sum beyond u64.
    #[error("instruction {position} overflows the transfer sum")]
    TransferSumOverflow { position: usize },
    /// A token instruction that is neither a sync nor a close back to the wallet.
    #[error(
        "instruction {position} is a token instruction other than a sync or a close back to the \
         wallet"
    )]
    TokenInstructionUnexpected { position: usize },
    /// An instruction calls a program the quoting router does not own.
    #[error("instruction {position} calls {program}, which is not a program of router {router}")]
    ProgramNotOfRouter {
        position: usize,
        program: String,
        router: String,
    },
    /// The build wraps more lamports than the trade may.
    #[error("the build wraps {wrapped} lamports, above the {bound} the trade may wrap")]
    WrapAboveBound { wrapped: u64, bound: u64 },
    /// The build pays the fee wallet more than the platform fee.
    #[error("the build pays the fee wallet {paid} lamports, above the {bound} platform fee")]
    FeeAboveBound { paid: u64, bound: u64 },
    /// The build tips more than the allowance outside the trade.
    #[error("the build tips {tipped} lamports, above the {allowance} allowed outside the trade")]
    TipAboveAllowance { tipped: u64, allowance: u64 },
    /// The simulated run grants control over the wallet's accounts.
    #[error("the run grants control over the wallet: {}", grants.join("; "))]
    GrantsControl { grants: Vec<String> },
    /// The output does not arrive at the quoted minimum.
    #[error(
        "the run delivers {received} of {mint} to the wallet, below the quoted minimum of \
         {minimum}"
    )]
    OutputBelowMinimum {
        received: i128,
        mint: String,
        minimum: i128,
    },
    /// The run takes more of the input mint than the quote spends.
    #[error("the run takes {spent} of {mint} from the wallet, above the quoted input of {input}")]
    InputAboveQuote {
        spent: u128,
        mint: String,
        input: u128,
    },
    /// The run takes a mint the trade does not spend.
    #[error("the run takes {spent} of {mint} from the wallet, a mint the trade does not spend")]
    UnspentMintTaken { spent: u128, mint: String },
}
