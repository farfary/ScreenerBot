//! Assembling the instruction list for a direct swap.
//!
//! The ORDER below is the whole safety story of a direct swap, so it is written
//! once, here, and every venue inherits it:
//!
//! ```text
//! 1. compute unit limit + price      always first, or they do not apply
//! 2. create input ATA  (idempotent)  no read-then-create race
//! 3. create output ATA (idempotent)
//! 4. wrap native SOL + sync_native   only when the input leg is WSOL
//! 5. platform fee transfer (input)   only when the fee rides the input leg
//! 6. the venue swap instruction      carries min_out, the on-chain guarantee
//! 7. platform fee transfer (output)  after the swap, before the WSOL close
//! 8. close the WSOL account          unwraps the proceeds back to native SOL
//! ```
//!
//! Step 7 before step 8 is not cosmetic: closing the account first would delete
//! the balance the fee transfer reads from, and the whole transaction — swap
//! included — would revert.
//!
//! The fee is in the SAME transaction as the swap by construction. There is no
//! ordering in which a user swaps and the platform is not paid.

use super::accounts::WalletLegs;
use super::compute::compute_budget_instructions;
use super::error::{DirectSwapError, DirectSwapResult};
use super::fee::FeeSide;
use super::intent::{is_wsol, DirectSwapIntent};
use super::quote::DirectQuote;
use super::venue::{PoolMarket, SwapAccounts};
use crate::chains::solana::solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, system_instruction,
};

/// A direct swap, fully expanded into instructions and the facts needed to check
/// afterwards that it did what it said.
#[derive(Debug, Clone)]
pub struct SwapPlan {
    /// The instruction list, in execution order.
    pub instructions: Vec<Instruction>,
    /// Compute units the venue asked for, before headroom.
    pub venue_compute_units: u32,
    /// The token account the proceeds land in.
    pub output_account: Pubkey,
    /// Whether the proceeds are unwrapped to native SOL inside this transaction,
    /// which is what makes receipt measurable only in lamports afterwards.
    pub output_is_native: bool,
    /// The quote this plan was built from.
    pub quote: DirectQuote,
}

/// Build the instruction list for `quote` against `market`.
pub fn build_plan(
    intent: &DirectSwapIntent,
    market: &dyn PoolMarket,
    quote: &DirectQuote,
) -> DirectSwapResult<SwapPlan> {
    let legs = WalletLegs::resolve(
        &intent.owner,
        market,
        &intent.input_mint,
        &intent.output_mint,
    )?;

    let venue_units = market.compute_units();
    let mut instructions = compute_budget_instructions(venue_units);

    instructions.extend(legs.ensure_instructions(
        &intent.owner,
        &intent.owner,
        &intent.input_mint,
        &intent.output_mint,
    ));

    if intent.wraps_native() {
        instructions.push(system_instruction::transfer(
            &intent.owner,
            &legs.input_account,
            intent.amount_in,
        ));
        instructions.push(
            crate::chains::solana::spl_token::instruction::sync_native(
                &crate::chains::solana::spl_token::id(),
                &legs.input_account,
            )
            .map_err(|e| DirectSwapError::Build {
                detail: format!("sync_native could not be built: {e}"),
            })?,
        );
    }

    if quote.fee.side == FeeSide::Input {
        if let Some(ix) = quote
            .fee
            .transfer_instruction(&legs.input_account, &intent.owner)?
        {
            instructions.push(ix);
        }
    }

    instructions.push(market.swap_instruction(
        &SwapAccounts {
            owner: intent.owner,
            input_mint: intent.input_mint,
            output_mint: intent.output_mint,
            input_token_account: legs.input_account,
            output_token_account: legs.output_account,
        },
        quote.swap_amount_in,
        quote.min_out,
    )?);

    if quote.fee.side == FeeSide::Output {
        if let Some(ix) = quote
            .fee
            .transfer_instruction(&legs.output_account, &intent.owner)?
        {
            instructions.push(ix);
        }
    }

    // Close whichever leg is the WSOL account so the wallet ends holding native
    // SOL. On a buy this reclaims the wrap account and any dust left in it; on a
    // sell it is how the proceeds become spendable SOL at all.
    if let Some(wsol_account) = wsol_account(intent, &legs) {
        instructions.push(
            crate::chains::solana::spl_token::instruction::close_account(
                &crate::chains::solana::spl_token::id(),
                &wsol_account,
                &intent.owner,
                &intent.owner,
                &[],
            )
            .map_err(|e| DirectSwapError::Build {
                detail: format!("WSOL close could not be built: {e}"),
            })?,
        );
    }

    Ok(SwapPlan {
        instructions,
        venue_compute_units: venue_units,
        output_account: legs.output_account,
        output_is_native: intent.unwraps_native(),
        quote: quote.clone(),
    })
}

/// The WSOL token account involved in this swap, if any.
fn wsol_account(intent: &DirectSwapIntent, legs: &WalletLegs) -> Option<Pubkey> {
    if is_wsol(&intent.input_mint) {
        Some(legs.input_account)
    } else if is_wsol(&intent.output_mint) {
        Some(legs.output_account)
    } else {
        None
    }
}
