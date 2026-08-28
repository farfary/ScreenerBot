//! Moonit (formerly Moonshot, `MoonCVVNZFSYkqNXP6bxHLPL6QQJiMagDL3qcqUQTrG`) —
//! a bonding-curve launchpad settling in NATIVE SOL, the same shape as
//! `pumpfun_legacy.rs`. Only `ConstantProductV1` curves over SOL collateral
//! are supported; every other curve/currency combination is refused.
//!
//! # The curve formula: found in the reference SDK, not fitted
//!
//! An earlier pass at this venue tried to FIT a virtual-reserve constant
//! product model from two replayed trades on the same pool and got within
//! ~2×10⁻¹¹ relative precision but never bit-identical — close enough to be
//! money-safe (an under-quote can only revert-safe, never over-pay) but not
//! enough to trust blindly. The actual formula is public: Moonit's own
//! `@wen-moon-ser/moonshot-sdk` (npm) delegates its `ConstantProductV1` curve
//! to `@heliofi/launchpad-common`'s `ConstantProductCurve`, whose source is
//! plain, unminified JS. It is NOT a continuously-tracked reserve — it
//! RECOMPUTES the virtual collateral reserve fresh from the token position on
//! every trade, discarding the fractional remainder each time:
//!
//! ```text
//! INITIAL_VIRTUAL_TOKEN_RESERVES      = 1_073_000_000_000_000_000  (raw, 9dp)
//! INITIAL_VIRTUAL_COLLATERAL_RESERVES =         30_000_000_000  (lamports)
//! CONSTANT_PRODUCT = INITIAL_VIRTUAL_TOKEN_RESERVES * INITIAL_VIRTUAL_COLLATERAL_RESERVES
//!
//! curve_position = total_supply - curve_amount        (tokens sold so far)
//! vtr = INITIAL_VIRTUAL_TOKEN_RESERVES - curve_position
//! vcr = floor(CONSTANT_PRODUCT / vtr)
//!
//! BUY  (net collateral in): new_vcr = vcr + net;  tokens_out = vtr - floor(CONSTANT_PRODUCT / new_vcr)
//! SELL (tokens in):         new_vtr = vtr + amount; collateral_out = vcr - floor(CONSTANT_PRODUCT / new_vtr)
//! ```
//!
//! Both constants are GLOBAL, protocol-wide — not derived from a pool's own
//! `coef_b`/`price_increase` fields, which this venue does not read for
//! pricing at all. Replaying this formula against FOUR real trades across TWO
//! independent pools (two buys on one curve, a buy and a sell on another)
//! reproduced every one of them to the EXACT raw unit — see the module's
//! offline test for the pool addresses and expected deltas.
//!
//! # Live on-chain proof, not just a replay
//!
//! Because the SDK is a client convenience and not the programme itself, the
//! formula was additionally proven against the LIVE program via
//! `simulateTransaction` (`sigVerify: false`, no real signature needed): a
//! buy whose `token_amount` threshold was set to the formula's exact
//! predicted output SUCCEEDED, and the identical buy with the threshold one
//! raw unit higher FAILED with `SlippageOverflow` (Anchor error `6003`,
//! `curve_state.rs:469`). The same pair (exact succeeds, exact+1 fails) was
//! also confirmed for a sell's `collateral_amount` threshold
//! (`curve_state.rs:529`). This is the strongest proof this engine has for
//! any venue: not an inference from a fixture, a live node enforcing the
//! exact number this module computes.
//!
//! # Rounding is deliberately conservative, matching every other venue
//!
//! Both internal divisions (`vcr` and the post-trade ratio) floor, exactly as
//! the reference implementation does. `quote()` never rounds an output up.
//! The safety property this engine's zero-slippage tier actually proves is
//! NON-OVERSTATEMENT: the on-chain `SlippageOverflow` check can only reject
//! a quote that promises too much, never one that promises too little, so an
//! honest floor at every step is both correct here (proven exactly above) and
//! the safe default if a future curve variant reintroduces any rounding
//! ambiguity.
//!
//! # Fee: read from the programme's own `ConfigAccount`, never hardcoded
//!
//! `ConfigAccount` is a PDA of seed `"config_account"` (verified: derives
//! `36Eru7v11oU5Pfrojyn5oY3nETA1a1iqsw2WUu6afkM9`, bump 251, matching the
//! account's own stored `bump` byte exactly). Its `fee_bps` (100 on every
//! observed trade) and `dex_fee`/`helio_fee` addresses are read fresh at
//! `load()` time rather than hardcoded, even though the reference SDK itself
//! hardcodes them — reading them costs nothing extra (same batched
//! `get_multiple_accounts` as the mint) and survives the protocol changing
//! its rate. The fee is charged on the GROSS amount and rounds DOWN, split
//! between `dex_fee`/`helio_fee` by the programme itself (`dex_fee_share`,
//! observed 50/50); this venue does not need to replicate that split, only
//! name both accounts. Verified to the lamport against real trades: on a
//! BUY the fee is deducted BEFORE the collateral reaches the curve (the curve
//! only ever sees `collateral_amount - fee`); on a SELL the curve pays out
//! its full GROSS constant-product output and the fee is deducted AFTERWARD,
//! from what the seller receives.
//!
//! # `CurveAccount`, verified byte-for-byte against two live pools
//!
//! Matches the on-chain IDL's `CurveAccount` struct exactly (409-byte
//! account, most of it unused reserved padding):
//!
//! ```text
//!  8 total_supply u64        16 curve_amount u64        24 mint pubkey
//! 56 decimals u8             57 collateral_currency u8  58 curve_type u8
//! 59 marketcap_threshold u64 67 marketcap_currency u8   68 migration_fee u64
//! 76 coef_b u32              80 bump u8                 81 migration_target u8
//! ```
//!
//! `collateral_currency == 0` is SOL (the only value this venue trades);
//! `curve_type == 1` is `ConstantProductV1` (the only value this formula was
//! verified against — `curve_type == 0`, `LinearV1`, uses a DIFFERENT SDK
//! class this venue does not implement and refuses rather than guesses at).
//! `curve_amount` was independently confirmed to equal the curve's own SPL
//! token account balance to the raw unit on both live pools, so this venue
//! reads it directly rather than fetching a second account for the same
//! number. `coef_b` and the undocumented trailing `price_increase` field
//! (`u16` @82, the deployed programme's account carries this one field
//! beyond its own published IDL, mirroring the existing price DECODER at
//! `pools/decoders/moonit_amm.rs`) play NO role in swap pricing — verified by
//! the exact four-trade replay reproducing outputs without ever reading
//! either.
//!
//! The programme itself enforces `total_supply == 1_000_000_000_000_000_000`
//! and `decimals == 9` for `ConstantProductV1` (Anchor errors
//! `IncorrectMaxSupply` / `IncorrectDecimals`), so `curve_position` can never
//! reach `INITIAL_VIRTUAL_TOKEN_RESERVES` and `vtr` can never go non-positive
//! — this venue still checks defensively rather than relying on that.
//!
//! # Instruction — `buy`/`sell`, 11 accounts, always `fixedSide::In`
//!
//! Discriminators confirmed against real transactions on two pools (a
//! Jupiter-routed CPI, not a top-level call, but the account list and args
//! are identical either way): `buy` = `sha256("global:buy")[..8]`
//! (`66063d1201daebea`), `sell` = `sha256("global:sell")[..8]`
//! (`33e685a4017f83ad`, coincidentally the same bytes as `pumpfun_amm.rs`'s
//! `sell` — the discriminator hashes only the instruction NAME, not the
//! programme). Args: `token_amount u64, collateral_amount u64, fixed_side
//! u8, slippage_bps u64` (33 bytes with the 8-byte disc).
//!
//! Every real trade replayed used `fixed_side = 0` (`FixedSide::In`), and the
//! reference SDK confirms what that means: the side that is the TRADE'S OWN
//! input is exact, the other is the enforced threshold. For a buy that is
//! `collateral_amount` exact / `token_amount` as the minimum tokens out; for
//! a sell, `token_amount` exact / `collateral_amount` as the minimum SOL out
//! — confirmed live (see above) to revert with `SlippageOverflow` when the
//! threshold is not met and to succeed at the exact computed value.
//! `slippage_bps` is sent as `0`: the exact threshold already carries the
//! full protection this engine needs, and every real trade observed used a
//! near-zero value here regardless of size, suggesting real integrators treat
//! it the same way.
//!
//! 11 accounts, exactly the on-chain IDL's list, confirmed identical across
//! three real CPI calls (two buys, one sell, two different pools) and via a
//! DIRECT (non-routed) buy that used exactly these 11 with no extras:
//!
//! ```text
//! 0 sender (signer)        1 senderTokenAccount     2 curveAccount
//! 3 curveTokenAccount      4 dexFee                 5 helioFee
//! 6 mint                   7 configAccount          8 tokenProgram
//! 9 associatedTokenProgram 10 systemProgram
//! ```
//!
//! `senderTokenAccount` is the wallet's OWN token account for the base mint
//! on both directions (receiving on a buy, spending on a sell) — `sender`
//! itself carries the native SOL leg directly, exactly like
//! `pumpfun_legacy.rs`. Routed transactions passed up to three MORE trailing
//! accounts (a spare WSOL ATA, an authority, the WSOL mint); a direct buy
//! with none of them still succeeded, so these are Jupiter's own routing
//! artifacts landing in Anchor's `remaining_accounts`, unused by Moonit
//! itself, not a requirement this venue needs to build.
//!
//! # Native SOL settlement
//!
//! Confirmed directly: on a sell, the curve's lamports drop and the seller's
//! own lamport balance rises by the exact net-of-fee amount via a same-owner
//! lamport credit inside the Moonit CPI itself (any programme may increase
//! any account's lamports; only the owner may decrease). No WSOL account is
//! ever read or written by Moonit — the WSOL account seen in ROUTED
//! transactions belongs to Jupiter's own subsequent leg (a `syncNative` +
//! System transfer in a SEPARATE instruction, after the Moonit CPI returns).
//! `settles_native_sol()` returns `true`; `plan.rs`'s existing fee-conduit
//! wrap (built for `pumpfun_legacy.rs`) needs no further change.
//!
//! # What this venue refuses, and why
//!
//! * `curve_type != 1` (not `ConstantProductV1`) — `LinearV1` is a different
//!   curve class in the reference SDK this venue does not implement.
//! * `collateral_currency != 0` (not SOL) — no observed pool trades anything
//!   else, and the settlement path assumes native SOL throughout.
//! * a mint not owned by the legacy SPL Token programme — every observed
//!   Moonit mint is legacy (the programme's own `tokenMint` instruction wires
//!   a fixed `tokenProgram` account, not a caller-chosen one), and this
//!   venue's account order was verified only for that case.
//! * `curve_amount > total_supply` or `total_supply` leaving no headroom
//!   under `INITIAL_VIRTUAL_TOKEN_RESERVES` — both should be structurally
//!   impossible given the programme's own `ConstantProductV1` invariants, but
//!   are checked rather than assumed.

use super::layout::{mint_decimals, pubkey_at, u16_at, u64_at, u8_at};
use crate::chains::solana::constants::{
    ASSOCIATED_TOKEN_PROGRAM_ID, MOONIT_AMM_PROGRAM_ID, SOL_MINT, SYSTEM_PROGRAM_ID,
};
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::chains::solana::spl_associated_token_account::get_associated_token_address_with_program_id;
use crate::chains::solana::swaps::direct::error::{DirectSwapError, DirectSwapResult};
use crate::chains::solana::swaps::direct::venue::{
    PoolMarket, PoolVenue, SwapAccounts, VenueQuote,
};
use async_trait::async_trait;
use std::str::FromStr;
use std::sync::LazyLock;

/// `sha256("global:buy")[..8]`, confirmed against two real mainnet buys and a
/// live zero-slippage `simulateTransaction` proof (see module docs).
const BUY: [u8; 8] = [0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea];
/// `sha256("global:sell")[..8]`, confirmed against a real mainnet sell and a
/// live zero-slippage `simulateTransaction` proof.
const SELL: [u8; 8] = [0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad];

/// `FixedSide::In` from the reference SDK -- the trade's own input side is
/// exact, the other side is the enforced on-chain threshold. Every real trade
/// replayed while building this venue used this value.
const FIXED_SIDE_IN: u8 = 0;

/// `curve_type` byte for `ConstantProductV1`, the only curve this venue quotes.
const CURVE_TYPE_CONSTANT_PRODUCT_V1: u8 = 1;
/// `collateral_currency` byte for SOL, the only collateral this venue quotes.
const COLLATERAL_CURRENCY_SOL: u8 = 0;

/// Seed of `ConfigAccount`'s PDA. Verified: derives
/// `36Eru7v11oU5Pfrojyn5oY3nETA1a1iqsw2WUu6afkM9` with bump 251, matching the
/// account's own stored `bump` byte read live.
const CONFIG_ACCOUNT_SEED: &[u8] = b"config_account";

/// The reference implementation's global virtual reserves
/// (`@heliofi/launchpad-common`'s `ConstantProductCurveV1`), reproduced
/// exactly rather than fitted -- see the module docs for the four-trade
/// replay and the live on-chain proof.
const INITIAL_VIRTUAL_TOKEN_RESERVES: u128 = 1_073_000_000_000_000_000;
const INITIAL_VIRTUAL_COLLATERAL_RESERVES: u128 = 30_000_000_000;

/// Denominator the programme's `fee_bps` is expressed over (basis points).
const BPS: u128 = 10_000;

/// Compute units this venue's swap needs. Real `simulateTransaction` runs
/// while verifying this venue consumed 24,935-38,482 CU for a buy/sell with
/// no ATA creation (the wallet's own token account already existed; `plan.rs`
/// creates it idempotently BEFORE this instruction runs either way).
const COMPUTE_UNITS: u32 = 90_000;

static WSOL_MINT: LazyLock<Pubkey> =
    LazyLock::new(|| Pubkey::from_str(SOL_MINT).expect("SOL_MINT constant is a valid pubkey"));

/// The venue adapter.
pub struct MoonitVenue;

#[async_trait]
impl PoolVenue for MoonitVenue {
    fn program(&self) -> ProgramKind {
        ProgramKind::Moonit
    }

    fn program_id(&self) -> Pubkey {
        moonit_program_id()
    }

    async fn load(
        &self,
        pool: &Pubkey,
        pool_account: &Account,
    ) -> DirectSwapResult<Box<dyn PoolMarket>> {
        let curve = CurveAccountState::decode(*pool, &pool_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: format!(
                    "Moonit curve account did not match the expected layout ({} bytes)",
                    pool_account.data.len()
                ),
            }
        })?;

        if curve.curve_type != CURVE_TYPE_CONSTANT_PRODUCT_V1 {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: format!(
                    "curve_type {} is not ConstantProductV1 (1); this venue's formula was \
                     verified only for that curve",
                    curve.curve_type
                ),
            });
        }
        if curve.collateral_currency != COLLATERAL_CURRENCY_SOL {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: format!(
                    "collateral_currency {} is not SOL (0); this venue trades SOL-collateral \
                     curves only",
                    curve.collateral_currency
                ),
            });
        }
        if curve.curve_amount > curve.total_supply {
            return Err(DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "curve_amount exceeds total_supply".to_owned(),
            });
        }
        if curve.total_supply as u128 >= INITIAL_VIRTUAL_TOKEN_RESERVES {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: format!(
                    "total_supply {} leaves no headroom under the protocol's virtual token \
                     reserve; unverified for this venue's formula",
                    curve.total_supply
                ),
            });
        }

        let config = config_account_address();
        let addresses = [curve.mint, config];
        let accounts = get_rpc_client()
            .get_multiple_accounts(&addresses)
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: *pool,
                detail: format!("Moonit accounts could not be read: {e}"),
            })?;

        let required = |index: usize| -> DirectSwapResult<&Account> {
            accounts.get(index).and_then(Option::as_ref).ok_or(
                DirectSwapError::AccountUnavailable {
                    address: addresses[index],
                    detail: "account does not exist".to_owned(),
                },
            )
        };

        let mint_account = required(0)?;
        if mint_account.owner != crate::chains::solana::spl_token::id() {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "the mint is not owned by the legacy SPL token programme; this venue's \
                         account order was verified only for legacy-token Moonit mints"
                    .to_owned(),
            });
        }
        let decimals =
            mint_decimals(&mint_account.data).ok_or_else(|| DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "the curve's mint is not a mint account".to_owned(),
            })?;

        let config_account = required(1)?;
        let config_state = ConfigAccountState::decode(&config_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "Moonit ConfigAccount did not match the expected layout".to_owned(),
            }
        })?;

        Ok(Box::new(MoonitMarket::new(
            curve,
            mint_account.owner,
            decimals,
            config_state.dex_fee,
            config_state.helio_fee,
            config_state.fee_bps,
        )))
    }
}

/// The parts of Moonit's `CurveAccount` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurveAccountState {
    pub pool: Pubkey,
    pub total_supply: u64,
    pub curve_amount: u64,
    pub mint: Pubkey,
    pub decimals: u8,
    pub collateral_currency: u8,
    pub curve_type: u8,
}

impl CurveAccountState {
    /// Decode a `CurveAccount`. Pure. Layout verified byte-for-byte against
    /// two live pools -- see the module docs.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        if data.len() < 82 {
            return None;
        }
        Some(Self {
            pool,
            total_supply: u64_at(data, 8)?,
            curve_amount: u64_at(data, 16)?,
            mint: pubkey_at(data, 24)?,
            decimals: u8_at(data, 56)?,
            collateral_currency: u8_at(data, 57)?,
            curve_type: u8_at(data, 58)?,
        })
    }

    /// Tokens sold so far, the reference curve's own `curvePosition`.
    fn curve_position(&self) -> u128 {
        (self.total_supply as u128).saturating_sub(self.curve_amount as u128)
    }
}

/// The parts of Moonit's global `ConfigAccount` a swap needs -- fetched fresh
/// at `load()` time rather than hardcoded, even though the reference SDK
/// hardcodes its own copies of these same values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigAccountState {
    pub helio_fee: Pubkey,
    pub dex_fee: Pubkey,
    pub fee_bps: u16,
}

impl ConfigAccountState {
    /// Decode a `ConfigAccount`. Pure. Verified against the live account:
    /// `fee_bps = 100`, `dex_fee_share = 50`, and `helio_fee`/`dex_fee` match
    /// the addresses every replayed real trade paid.
    pub fn decode(data: &[u8]) -> Option<Self> {
        Some(Self {
            // migration_authority @8, backend_authority @40, config_authority @72
            helio_fee: pubkey_at(data, 104)?,
            dex_fee: pubkey_at(data, 136)?,
            fee_bps: u16_at(data, 168)?,
        })
    }
}

/// The current virtual reserves at `curve_position`, the reference SDK's own
/// `getCurrentReserves`: RECOMPUTED fresh from the token position rather than
/// tracked continuously, floor-rounded, exactly as the live programme does.
fn current_reserves(curve_position: u128) -> Option<(u128, u128)> {
    let vtr = INITIAL_VIRTUAL_TOKEN_RESERVES.checked_sub(curve_position)?;
    if vtr == 0 {
        return None;
    }
    let constant_product = INITIAL_VIRTUAL_TOKEN_RESERVES * INITIAL_VIRTUAL_COLLATERAL_RESERVES;
    Some((vtr, constant_product / vtr))
}

/// Tokens returned for `net_collateral` (already net of the programme's own
/// fee) in, the reference SDK's `buyInCollateral`. Floors, matching the live
/// programme exactly -- verified to the raw unit on two real buys.
fn buy_in_collateral(net_collateral: u128, curve_position: u128) -> Option<u128> {
    let (vtr, vcr) = current_reserves(curve_position)?;
    let new_vcr = vcr.checked_add(net_collateral)?;
    if new_vcr == 0 {
        return None;
    }
    let constant_product = INITIAL_VIRTUAL_TOKEN_RESERVES * INITIAL_VIRTUAL_COLLATERAL_RESERVES;
    vtr.checked_sub(constant_product / new_vcr)
}

/// Gross collateral (BEFORE the programme's own fee) returned for
/// `token_amount` in, the reference SDK's `sellInToken`. Floors -- verified
/// to the raw unit on a real sell.
fn sell_in_token(token_amount: u128, curve_position: u128) -> Option<u128> {
    let (vtr, vcr) = current_reserves(curve_position)?;
    let new_vtr = vtr.checked_add(token_amount)?;
    if new_vtr == 0 {
        return None;
    }
    let constant_product = INITIAL_VIRTUAL_TOKEN_RESERVES * INITIAL_VIRTUAL_COLLATERAL_RESERVES;
    vcr.checked_sub(constant_product / new_vtr)
}

/// The programme's own fee on `gross`, rounded DOWN -- verified exactly
/// against real trades (`floor(gross * fee_bps / 10_000)`).
fn fee_on_gross(gross: u64, fee_bps: u16) -> u64 {
    if fee_bps == 0 {
        return 0;
    }
    (((gross as u128) * (fee_bps as u128)) / BPS).min(gross as u128) as u64
}

/// A decoded, quotable Moonit `ConstantProductV1` curve.
#[derive(Debug, Clone)]
pub struct MoonitMarket {
    curve: CurveAccountState,
    token_program: Pubkey,
    mint_decimals: u8,
    curve_token_account: Pubkey,
    dex_fee: Pubkey,
    helio_fee: Pubkey,
    fee_bps: u16,
}

impl MoonitMarket {
    /// Build a market directly from decoded parts, for the offline test tier.
    pub fn new(
        curve: CurveAccountState,
        token_program: Pubkey,
        mint_decimals: u8,
        dex_fee: Pubkey,
        helio_fee: Pubkey,
        fee_bps: u16,
    ) -> Self {
        let curve_token_account =
            get_associated_token_address_with_program_id(&curve.pool, &curve.mint, &token_program);
        Self {
            curve,
            token_program,
            mint_decimals,
            curve_token_account,
            dex_fee,
            helio_fee,
            fee_bps,
        }
    }

    fn is_base(&self, mint: &Pubkey) -> Option<bool> {
        if *mint == self.curve.mint {
            Some(true)
        } else if *mint == *WSOL_MINT {
            Some(false)
        } else {
            None
        }
    }
}

impl PoolMarket for MoonitMarket {
    fn program(&self) -> ProgramKind {
        ProgramKind::Moonit
    }

    fn pool(&self) -> Pubkey {
        self.curve.pool
    }

    fn mints(&self) -> (Pubkey, Pubkey) {
        (self.curve.mint, *WSOL_MINT)
    }

    fn token_program(&self, mint: &Pubkey) -> Option<Pubkey> {
        self.is_base(mint).map(|base| {
            if base {
                self.token_program
            } else {
                crate::chains::solana::spl_token::id()
            }
        })
    }

    fn decimals(&self, mint: &Pubkey) -> Option<u8> {
        self.is_base(mint)
            .map(|base| if base { self.mint_decimals } else { 9 })
    }

    fn settles_native_sol(&self) -> bool {
        true
    }

    fn quote(&self, input_mint: &Pubkey, amount_in: u64) -> DirectSwapResult<VenueQuote> {
        let input_is_base = self
            .is_base(input_mint)
            .ok_or(DirectSwapError::PairNotInPool {
                pool: self.curve.pool,
                input_mint: *input_mint,
                output_mint: Pubkey::default(),
            })?;
        if amount_in == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.curve.pool,
                amount_in,
                detail: "zero input".to_owned(),
            });
        }

        let curve_position = self.curve.curve_position();
        let math_error = || DirectSwapError::QuoteMath {
            detail: "Moonit curve arithmetic overflowed or produced a non-positive reserve"
                .to_owned(),
        };

        let (expected_out, lp_fee, reserve_in, reserve_out) = if input_is_base {
            // SELL: tokens in, native SOL out. The curve pays out its full
            // GROSS constant-product output; the programme's own fee comes
            // off that output afterward -- verified exactly on a real sell.
            let (vtr, vcr) = current_reserves(curve_position).ok_or_else(math_error)?;
            let gross = sell_in_token(amount_in as u128, curve_position).ok_or_else(math_error)?;
            let gross_u64 = u64::try_from(gross).map_err(|_| math_error())?;
            let fee = fee_on_gross(gross_u64, self.fee_bps);
            // `VenueQuote::lp_fee` is contracted to be in INPUT units, so this
            // output-side lamport fee is converted back into tokens at the
            // realised rate of this very fill -- never at a spot price. The
            // same conversion `meteora_damm.rs` and `meteora_dbc.rs` keep for
            // their own output-side fee modes.
            let fee_in_input = if gross_u64 == 0 {
                0
            } else {
                (((fee as u128) * (amount_in as u128)) / (gross_u64 as u128)) as u64
            };
            (gross_u64.saturating_sub(fee), fee_in_input, vtr, vcr)
        } else {
            // BUY: native SOL in. The programme's own fee is deducted BEFORE
            // the collateral reaches the curve -- verified exactly on two
            // real buys.
            let (vtr, vcr) = current_reserves(curve_position).ok_or_else(math_error)?;
            let fee = fee_on_gross(amount_in, self.fee_bps);
            let net_collateral = amount_in.saturating_sub(fee);
            let tokens =
                buy_in_collateral(net_collateral as u128, curve_position).ok_or_else(math_error)?;
            let tokens_u64 = u64::try_from(tokens).map_err(|_| math_error())?;
            (tokens_u64, fee, vcr, vtr)
        };

        if expected_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.curve.pool,
                amount_in,
                detail: "the curve returns nothing at this size".to_owned(),
            });
        }

        let reserve_in_u64 = u64::try_from(reserve_in).map_err(|_| math_error())?;
        let reserve_out_u64 = u64::try_from(reserve_out).map_err(|_| math_error())?;

        Ok(VenueQuote {
            amount_in,
            expected_out,
            lp_fee,
            price_impact_pct: super::math::price_impact_pct(
                reserve_in_u64,
                reserve_out_u64,
                amount_in,
                expected_out,
            ),
        })
    }

    fn swap_instruction(
        &self,
        accounts: &SwapAccounts,
        amount_in: u64,
        min_out: u64,
    ) -> DirectSwapResult<Instruction> {
        let input_is_base =
            self.is_base(&accounts.input_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.curve.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        let output_is_base =
            self.is_base(&accounts.output_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.curve.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        if input_is_base == output_is_base {
            return Err(DirectSwapError::PairNotInPool {
                pool: self.curve.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            });
        }

        // `senderTokenAccount` is the wallet's OWN token account for the base
        // mint on both directions; the native SOL leg has no token account of
        // its own, `sender` (the signer) carries it directly.
        let sender_token_account = if input_is_base {
            accounts.input_token_account
        } else {
            accounts.output_token_account
        };

        // `FixedSide::In`: the trade's own input side is exact, the other is
        // the enforced minimum -- confirmed live to revert with
        // `SlippageOverflow` when unmet (see module docs).
        let (disc, token_amount, collateral_amount) = if input_is_base {
            (SELL, amount_in, min_out)
        } else {
            (BUY, min_out, amount_in)
        };

        let mut data = Vec::with_capacity(33);
        data.extend_from_slice(&disc);
        data.extend_from_slice(&token_amount.to_le_bytes());
        data.extend_from_slice(&collateral_amount.to_le_bytes());
        data.push(FIXED_SIDE_IN);
        data.extend_from_slice(&0u64.to_le_bytes()); // slippage_bps: redundant with the exact threshold above

        let metas = vec![
            AccountMeta::new(accounts.owner, true),
            AccountMeta::new(sender_token_account, false),
            AccountMeta::new(self.curve.pool, false),
            AccountMeta::new(self.curve_token_account, false),
            AccountMeta::new(self.dex_fee, false),
            AccountMeta::new(self.helio_fee, false),
            AccountMeta::new_readonly(self.curve.mint, false),
            AccountMeta::new_readonly(config_account_address(), false),
            AccountMeta::new_readonly(self.token_program, false),
            AccountMeta::new_readonly(associated_token_program_id(), false),
            AccountMeta::new_readonly(system_program_id(), false),
        ];

        Ok(Instruction {
            program_id: moonit_program_id(),
            accounts: metas,
            data,
        })
    }

    fn compute_units(&self) -> u32 {
        COMPUTE_UNITS
    }
}

fn moonit_program_id() -> Pubkey {
    Pubkey::from_str(MOONIT_AMM_PROGRAM_ID).expect("Moonit program id constant is valid")
}

fn associated_token_program_id() -> Pubkey {
    Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID).expect("ATA program id constant is valid")
}

fn system_program_id() -> Pubkey {
    Pubkey::from_str(SYSTEM_PROGRAM_ID).expect("system program id constant is valid")
}

/// `ConfigAccount`'s PDA: `["config_account"]`. Verified: derives
/// `36Eru7v11oU5Pfrojyn5oY3nETA1a1iqsw2WUu6afkM9` with bump 251, matching the
/// live account's own stored `bump` byte.
fn config_account_address() -> Pubkey {
    Pubkey::find_program_address(&[CONFIG_ACCOUNT_SEED], &moonit_program_id()).0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program_id_for_test() -> Pubkey {
        Pubkey::from_str("MoonCVVNZFSYkqNXP6bxHLPL6QQJiMagDL3qcqUQTrG").unwrap()
    }

    #[test]
    fn config_account_pda_matches_the_live_address_and_bump() {
        let (address, bump) =
            Pubkey::find_program_address(&[CONFIG_ACCOUNT_SEED], &program_id_for_test());
        assert_eq!(
            address.to_string(),
            "36Eru7v11oU5Pfrojyn5oY3nETA1a1iqsw2WUu6afkM9"
        );
        assert_eq!(bump, 251);
    }

    /// The exact real trade `2yhdUUHyW9BgoJyX4MfkV5UQRc4NNnt8Ag3m6iRKcTn4LrZ6FJfyvcnFP64paAafYD3EEAjNnkhxy38gasAgkwfJ`:
    /// a buy on curve `Fiw2hDFe4YW4acj1pxpEwXVFf2aBHBBnog6qzoA5pCdW`
    /// (`total_supply = 1_000_000_000_000_000_000`,
    /// `curve_amount = 862_374_540_156_582_982` pre-trade), net collateral
    /// `29_447_550` lamports (gross `29_745_000` minus the 100bps fee),
    /// reproduced to the raw unit: `799_701_317_842_959` tokens out. The very
    /// NEXT trade on the same curve (`KnwknjBxhBLiHwKX6NwHJYyEyzkKa8F4R5LS2vAqWS1wQuWfHe4v8MMkeNgo5eUmz26oCoVQQwgwCAeKdFZcpF1`,
    /// exercised by `buy_quote_orientation_and_native_sol_settlement` below)
    /// reproduces exactly too, so the formula holds across two consecutive
    /// trades on the same pool, not just one.
    #[test]
    fn buy_in_collateral_reproduces_a_real_trade_to_the_raw_unit() {
        let total_supply = 1_000_000_000_000_000_000u64;
        let curve_amount = 862_374_540_156_582_982u64;
        let curve_position = (total_supply - curve_amount) as u128;
        let net_collateral = 29_447_550u128;
        let tokens_out = buy_in_collateral(net_collateral, curve_position).unwrap();
        assert_eq!(tokens_out, 799_701_317_842_959);
    }

    /// The exact real trade `3eKffGDSTtcJqELsNVQ8tFTjFPJzzDXWt9qFMrQ8krF1Q8JwCZxn2MjSTfBNpTAjCZPETMABeSzqrjZkvJvu4yY`:
    /// a buy on curve `8fZo2Lybm3Y3VucDcL7oQxaR8b1NnjoAAbQ3JnAPB9CA`
    /// (`curve_amount = 997_650_577_122_392_181` pre-trade), net collateral
    /// `37_080_208` lamports, reproduced to the raw unit.
    #[test]
    fn buy_in_collateral_reproduces_a_second_pool_to_the_raw_unit() {
        let total_supply = 1_000_000_000_000_000_000u64;
        let curve_amount = 997_650_577_122_392_181u64;
        let curve_position = (total_supply - curve_amount) as u128;
        let net_collateral = 37_080_208u128;
        let tokens_out = buy_in_collateral(net_collateral, curve_position).unwrap();
        assert_eq!(tokens_out, 1_318_807_478_320_357);
    }

    /// The exact real trade `5xwomwg9W77cFAQ7rAvEVHSu76eN7qWsecUiGMXhtaXpBg4Sys1CBhYDEB1HBKpEUu2G1jt9SWSzMpvPJ99UmDUg`:
    /// a sell on the SAME pool as above, post-buy state
    /// (`curve_amount = 996_331_769_644_071_824` pre-trade), `1_679_967_300_000_000`
    /// tokens in, reproduced to the raw unit as the GROSS collateral out
    /// (before the programme's own fee).
    #[test]
    fn sell_in_token_reproduces_a_real_trade_to_the_raw_unit() {
        let total_supply = 1_000_000_000_000_000_000u64;
        let curve_amount = 996_331_769_644_071_824u64;
        let curve_position = (total_supply - curve_amount) as u128;
        let tokens_in = 1_679_967_300_000_000u128;
        let gross_out = sell_in_token(tokens_in, curve_position).unwrap();
        assert_eq!(gross_out, 47_218_818);
    }

    #[test]
    fn fee_on_gross_rounds_down_and_matches_real_trades() {
        assert_eq!(fee_on_gross(29_745_000, 100), 297_450);
        assert_eq!(fee_on_gross(28_719_041, 100), 287_190); // 287_190.41 floored
        assert_eq!(fee_on_gross(0, 100), 0);
        assert_eq!(fee_on_gross(100, 0), 0);
    }

    #[test]
    fn a_fresh_curve_starts_at_the_initial_virtual_reserves() {
        let (vtr, vcr) = current_reserves(0).unwrap();
        assert_eq!(vtr, INITIAL_VIRTUAL_TOKEN_RESERVES);
        assert_eq!(vcr, INITIAL_VIRTUAL_COLLATERAL_RESERVES);
    }

    #[test]
    fn buy_quote_orientation_and_native_sol_settlement() {
        let curve = CurveAccountState {
            pool: Pubkey::new_unique(),
            total_supply: 1_000_000_000_000_000_000,
            curve_amount: 861_574_838_838_740_023,
            mint: Pubkey::new_unique(),
            decimals: 9,
            collateral_currency: 0,
            curve_type: 1,
        };
        let market = MoonitMarket::new(
            curve,
            crate::chains::solana::spl_token::id(),
            9,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            100,
        );
        assert!(market.settles_native_sol());
        let (mint, sol) = market.mints();
        assert_eq!(sol, *WSOL_MINT);

        let quote = market.quote(&sol, 28_719_041).expect("buy quotes");
        assert_eq!(quote.expected_out, 770_821_784_971_935);
        assert_eq!(quote.lp_fee, 287_190);

        let sell = market
            .quote(&mint, quote.expected_out)
            .expect("reverse direction quotes");
        assert!(sell.expected_out > 0);

        // The sell's fee is charged on the lamport OUTPUT, but `lp_fee` is
        // contracted to be in INPUT units -- so it must be ~1% of the TOKENS
        // going in, not ~1% of the lamports coming out. Reporting the raw
        // lamport figure here would understate the fee by the token price, a
        // factor of tens of millions on this curve.
        let one_percent_of_input = quote.expected_out / 100;
        assert!(
            sell.lp_fee > one_percent_of_input * 9 / 10
                && sell.lp_fee < one_percent_of_input * 11 / 10,
            "sell lp_fee {} should be ~1% of the {} tokens in, in INPUT units",
            sell.lp_fee,
            quote.expected_out
        );
    }

    #[test]
    fn quote_refuses_a_mint_outside_the_pair() {
        let curve = CurveAccountState {
            pool: Pubkey::new_unique(),
            total_supply: 1_000_000_000_000_000_000,
            curve_amount: 500_000_000_000_000_000,
            mint: Pubkey::new_unique(),
            decimals: 9,
            collateral_currency: 0,
            curve_type: 1,
        };
        let market = MoonitMarket::new(
            curve,
            crate::chains::solana::spl_token::id(),
            9,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            100,
        );
        let stranger = Pubkey::new_unique();
        assert!(matches!(
            market.quote(&stranger, 1_000_000),
            Err(DirectSwapError::PairNotInPool { .. })
        ));
    }
}
