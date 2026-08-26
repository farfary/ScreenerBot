//! The direct pool-swap engine: build a DEX swap instruction ourselves and
//! execute it against the pool, with no aggregator in the path.
//!
//! # Why it exists
//!
//! An aggregator adds a network hop, a quote that can go stale, a rate limit and
//! a dependency on someone else's uptime. Swapping straight against a pool we
//! already decode for pricing removes all four. It also means the swap executes
//! in the SAME pool the live price came from, so the price a decision was made on
//! is the price the decision trades at.
//!
//! # Shape
//!
//! ```text
//! intent.rs   what was asked: a mint pair, an amount, a pool
//! registry.rs pool account owner -> venue                (the only naming site)
//! venue.rs    the venue contract: async load + PURE market
//! venues/     one module per DEX program
//! quote.rs    venue curve + slippage floor + platform fee -> DirectQuote
//! fee.rs      the 0.5% platform fee and which leg pays it
//! accounts.rs the wallet's two token accounts, idempotently created
//! compute.rs  compute unit limit and price
//! plan.rs     the instruction ORDER, shared by every venue
//! execute.rs  sign -> simulate -> send -> confirm
//! verify.rs   what actually arrived
//! ```
//!
//! # The invariants
//!
//! * A swap is a MINT PAIR, never a buy/sell. SOL->TOKEN, TOKEN->SOL,
//!   USDC->TOKEN, TOKEN->USDC, SOL->USDC and USDC->SOL are one code path.
//! * `min_out` is the only on-chain protection. It is computed from the venue's
//!   own exact curve, never from a spot price times an amount.
//! * The platform fee is in the same transaction as the swap. There is no
//!   ordering in which the user swaps and we are not paid.
//! * The pure half of a venue never touches RPC, config or the clock, so the
//!   offline test tier can assert real numbers against real captured accounts.

pub mod accounts;
pub mod compute;
pub mod error;
pub mod execute;
pub mod fee;
pub mod intent;
pub mod plan;
pub mod quote;
pub mod registry;
pub mod venue;
pub mod venues;
pub mod verify;

pub use error::{DirectSwapError, DirectSwapResult};
pub use execute::{execute_plan, simulate_plan, DirectSwapOutcome};
pub use fee::{FeeSide, PlatformFee};
pub use intent::DirectSwapIntent;
pub use plan::{build_plan, SwapPlan};
pub use quote::DirectQuote;
pub use venue::{PoolMarket, PoolVenue, SwapAccounts, VenueQuote};
pub use verify::Receipt;

use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::{pubkey::Pubkey, signature::Keypair};

/// Load and decode a pool into a quotable market, choosing the venue from the
/// account's owner.
pub async fn load_market(pool: &Pubkey) -> DirectSwapResult<Box<dyn PoolMarket>> {
    let account = get_rpc_client()
        .get_account(pool)
        .await
        .map_err(|e| DirectSwapError::AccountUnavailable {
            address: *pool,
            detail: e.to_string(),
        })?
        .ok_or(DirectSwapError::AccountUnavailable {
            address: *pool,
            detail: "pool account does not exist".to_owned(),
        })?;

    let venue = registry::require_venue(&account.owner)?;
    venue.load(pool, &account).await
}

/// Price `intent` against an already-loaded market. Pure apart from config reads:
/// no RPC, so a caller holding a market can re-quote a new size for free.
pub fn quote_with_market(
    intent: &DirectSwapIntent,
    market: &dyn PoolMarket,
) -> DirectSwapResult<DirectQuote> {
    intent.validate()?;

    if !market.trades(&intent.input_mint, &intent.output_mint) {
        return Err(DirectSwapError::PairNotInPool {
            pool: market.pool(),
            input_mint: intent.input_mint,
            output_mint: intent.output_mint,
        });
    }

    // Side first, then the input-side fee, then the pool quote on what is left.
    // See the ordering note in `fee` -- doing this in any other order makes the
    // fee and the quote depend on each other.
    let side = FeeSide::for_pair(&intent.input_mint, &intent.output_mint);
    let base = if side == FeeSide::Input {
        intent.amount_in
    } else {
        0
    };
    let input_fee = PlatformFee::resolve(side, &intent.input_mint, &intent.output_mint, base)?;

    let swap_amount_in = if side == FeeSide::Input {
        intent.amount_in.saturating_sub(input_fee.amount)
    } else {
        intent.amount_in
    };
    if swap_amount_in == 0 {
        return Err(DirectSwapError::InvalidRequest {
            detail: "the platform fee consumes the entire input at this size".to_owned(),
        });
    }

    let venue_quote = market.quote(&intent.input_mint, swap_amount_in)?;
    DirectQuote::assemble(
        intent,
        market.program(),
        swap_amount_in,
        venue_quote,
        input_fee,
    )
}

/// Load the pool and price `intent` in one call. Returns the market alongside the
/// quote so the caller can build a plan without re-reading the chain.
pub async fn quote(
    intent: &DirectSwapIntent,
) -> DirectSwapResult<(DirectQuote, Box<dyn PoolMarket>)> {
    intent.validate()?;
    let market = load_market(&intent.pool).await?;
    let quote = quote_with_market(intent, market.as_ref())?;
    Ok((quote, market))
}

/// Quote, build and execute `intent` end to end.
pub async fn swap(
    intent: &DirectSwapIntent,
    keypair: &Keypair,
) -> DirectSwapResult<DirectSwapOutcome> {
    let (quote, market) = quote(intent).await?;
    let plan = build_plan(intent, market.as_ref(), &quote)?;
    execute_plan(&plan, keypair).await
}
