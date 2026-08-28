//! Meteora Dynamic Bonding Curve (`dbcij3LW…`), the pre-graduation bonding
//! curve every DBC-launched token trades on before it migrates to a
//! post-migration AMM (usually `meteora_damm.rs`'s cp-amm). Structurally the
//! closest sibling in this engine is DAMM v2, not the constant-product
//! `meteora_dlmm.rs`/`pumpfun_legacy.rs` curves: both DBC and DAMM v2 price
//! off a Q64.64 `sqrt_price` and a `liquidity` value with the same shape of
//! step math, and both collect their trade fee once per swap via a
//! `collect_fee_mode` byte rather than per tick like Raydium CLMM/Orca
//! Whirlpool.
//!
//! # `liquidity` is DOUBLE-scaled, not single Q64.64 -- established by replay
//!
//! `clmm_ticks`'s step primitives (`output_for_move`,
//! `next_sqrt_price_from_input`, `input_for_move`) assume `liquidity` is a
//! plain integer at the SAME scale Raydium CLMM and Orca Whirlpool store it
//! at (`Δy = L·Δ√P / 2^64`). Applying them unchanged to DBC's stored
//! `liquidity` values (`~4·10^31` on the pools inspected) makes every
//! realistic trade size round `Δ√P` to exactly zero -- `output_for_move` then
//! returns 0 and every quote fails `InsufficientLiquidity`, which is what
//! this venue's first draft did before being checked against a real
//! transaction. Replaying a real buy
//! (`3p2tGddXKd7wdLtkwLvU6pH3mWAN4QGGER58xqJbLvowSRcNHSTC3WNFaErcAeQZHAKx9Nigk5vtzFQXex1s2uFB`,
//! pool `54QGogfaocKyDkvaj2Pcm7495ZNMoF9G8vU6osfyFjMR`, config
//! `BPas3hqheozAEtwyTTFKprg58JcVfUSduwyLc5BmMyq5`) against a walk that instead
//! treats the STORED value as `L_real · 2^64` (i.e. `liquidity` itself
//! carries the same `2^64` fixed-point scale as `sqrt_price`, so the combined
//! scale in `Δy = L_real·Δ√P/2^64` is `2^128`) reproduces the right ORDER OF
//! MAGNITUDE from `sqrt_start_price` (`9,409,135,741` gross base units for a
//! 40,000,000-lamport buy against an observed net receipt of `8,048,489,049`
//! after a 1% fee -- the gap is consistent with the pool not being perfectly
//! fresh at the traded slot, i.e. an unknown, unreplayable amount of prior
//! volume having already moved the price off `sqrt_start_price`, not with the
//! scale being wrong). No RPC here can read HISTORICAL account state to
//! confirm the exact pre-trade `sqrt_price`, so the REPLAY is
//! order-of-magnitude rather than exact.
//!
//! The raw-unit proof comes from the other direction instead:
//! `a_meteora_dbc_quote_is_exact_to_the_raw_unit` simulates the swap with
//! `slippage_bps = 0`, so `min_out` IS the quote, and the programme enforces
//! `min_out` itself. A node accepting that swap against CURRENT state proves
//! the walk does not over-state the output by even one raw unit -- which is
//! the direction that costs money, since an over-stated quote is an
//! unsatisfiable floor. That leaves this venue in exactly the evidentiary
//! position `meteora_dlmm.rs` is in: replay unavailable, zero-slippage
//! simulation green. It does NOT rule out under-stating, the same caveat that
//! test carries everywhere. The double scale is why this
//! venue carries its OWN step primitives (`output_for_move`,
//! `next_sqrt_price_from_input`, `input_for_move` below) built on
//! `math::mul_shr128_floor`/`mul_shr128_ceil`/`shl128_div_floor` (a division
//! by exactly `2^128`, which does not fit as a `u128` denominator to
//! `clmm_ticks`'s generic `mul_div_floor`) rather than
//! reusing `clmm_ticks`'s single-scale ones. The boundary-list WALK shape
//! (segments, partial-step-vs-full-step, refuse past the last loaded segment)
//! still mirrors `clmm_ticks::walk_ticks`.
//!
//! # Layout, verified against mainnet
//!
//! Both account layouts were read from the programme's own on-chain Anchor
//! IDL (`create_with_seed(find_program_address([], program), "anchor:idl",
//! program)`, decoded per `adding-a-venue.md`) and cross-checked field-by-field
//! against a live pool (`A95th9YTiZrGYRsZ4eBXLvMLpr7pfqPVWwY1LFQ8f27U`) and its
//! config (`7wr6arSoaxQEppcSakvouxpKF9bfcYLCRRn4HNMxj2cZ`): the IDL's own
//! discriminator for `VirtualPool` (`d5e005d16245775c`) matches the pool
//! account's first 8 bytes exactly, and `PoolConfig`'s (`1a6c0e7b74e6812b`)
//! matches the config account's.
//!
//! `VirtualPool`, 424 bytes:
//!
//! ```text
//!   0 discriminator        8 volatility_tracker (64B, unused here)
//!  72 config               104 creator            136 base_mint
//! 168 base_vault           200 quote_vault
//! 232 base_reserve u64     240 quote_reserve u64
//! 248 protocol_base_fee    256 protocol_quote_fee
//! 264 partner_base_fee     272 partner_quote_fee
//! 280 sqrt_price u128      296 activation_point u64
//! 304 pool_type u8         305 is_migrated u8
//! ```
//!
//! `PoolConfig`, 1048 bytes:
//!
//! ```text
//!   0 discriminator         8 quote_mint          40 fee_claimer
//! 104 base_fee.cliff_fee_numerator u64  112 second_factor u64
//! 120 third_factor u64      128 first_factor u16   130 base_fee_mode u8
//! 136 dynamic_fee.initialized u8
//! 144 max_volatility_accumulator u32    148 variable_fee_control u32
//! 152 bin_step u16          154 filter_period u16  156 decay_period u16
//! 158 reduction_factor u16
//! 232 collect_fee_mode u8   235 token_decimal u8
//! 392 sqrt_start_price u128
//! 408 curve: [LiquidityDistributionConfig; 20], 32B each
//!       (sqrt_price u128, liquidity u128)
//! ```
//!
//! `base_reserve`/`quote_reserve` are NOT read by this venue: unlike a
//! constant-product pool, a concentrated-liquidity curve is fully priced by
//! `sqrt_price` + the segment list, so the reserve fields (bookkeeping only)
//! would be redundant. The vaults ARE fetched, to cap a quote at what the
//! vault can actually pay out — the same safety check `meteora_damm.rs`
//! applies, since DBC's uncollected protocol/partner/creator fee also sits
//! IN the vault rather than being physically swept out on every trade
//! (confirmed below).
//!
//! # The curve is real segments, not padding
//!
//! Every real config read while building this venue carries at least two
//! non-zero points, and the LAST one always sits at, or within about one part
//! in 10^9 of (evidently an independent rounding of the same target, not a
//! decode drift — the fixture's offline test asserts the closeness rather
//! than exact equality), the config's own `migration_sqrt_price` field, at a
//! completely different byte offset (280, not 408). `A95th9Y…`
//! (`285775730356909824, …`) additionally carried a THIRD, much-thinner-
//! liquidity point sitting at Meteora's well-known `MAX_SQRT_PRICE`
//! (`79226673521066979257578248091`, the same ceiling DLMM/DAMM use) past its
//! migration price — an overflow segment so a swap can never mechanically run
//! out of curve even beyond migration. That overflow segment is NOT universal:
//! the fixture pool this venue's tests now use (`J1YvC19…`, config
//! `GUBN6yp…`) carries exactly two points and stops at the migration price
//! with no overflow segment at all. `curve[N..19]` past the last real point
//! are all-zero padding either way. Segments are taken while `sqrt_price >
//! 0`; a swap that would need to walk past the last loaded segment is refused
//! (`InsufficientLiquidity`) rather than assumed to keep going — which means a
//! pool with no overflow segment genuinely refuses a swap sized to cross its
//! own migration price, which is correct: such a pool has already migrated
//! by the time a real trade could reach that price.
//!
//! **A pitfall worth naming:** `A95th9Y…`, the pool this venue's layout and
//! fee mechanics were originally verified against, MIGRATED (crossed its
//! 12-SOL `migration_quote_threshold`) in the roughly 20 minutes between
//! being chosen and being used to capture the offline fixture — confirming
//! `testing.md`'s warning to verify a candidate pool on chain immediately
//! before use, not from an earlier snapshot. The fixture pools actually
//! shipped (`J1YvC19…`/`GUBN6yp…` for `collect_fee_mode = 0`, `F71peWVS…`/
//! `BPas3hq…` for `= 1`) were re-verified not-migrated at capture time; the
//! layout/discriminator/fee-mechanics findings above remain valid regardless
//! of what any individual cited pool does afterward, since they are facts
//! about the PROGRAMME, not about one pool's later trading.
//!
//! # Orientation and settlement
//!
//! `base_mint` is always token "0" (Uniswap-v3 orientation: price = quote per
//! base), `quote_mint` token "1". A BUY (quote in, base out) is
//! `zero_for_one = false`, price rising; a SELL is `zero_for_one = true`,
//! price falling — the same `zero_for_one` convention `clmm_ticks` uses,
//! even though the step arithmetic itself is this venue's own (see above).
//! `quote_vault` is an ordinary SPL token account (owned by
//! `TokenkegQ…`, holding WSOL on every pool inspected) — NOT a native-lamport
//! settlement like `pumpfun_legacy.rs`'s curve, which has no SOL-side token
//! account at all. `settles_native_sol` therefore stays at its default
//! `false`.
//!
//! # The fee: total once per swap, not per segment step
//!
//! `collect_fee_mode` (verified against two real pools with different values,
//! 0 and 1) decides which leg pays, the same two-state shape
//! `meteora_damm.rs`'s `collect_fee_mode` already documents and this venue's
//! `fee_on_input` mirrors: mode 0 (`QuoteToken`) always charges whichever leg
//! IS quote; mode 1 (`OutputToken`) always charges the output leg, regardless
//! of denomination. Verified against a real successful `buy`
//! (`3p2tGddXKd7wdLtkwLvU6pH3mWAN4QGGER58xqJbLvowSRcNHSTC3WNFaErcAeQZHAKx9Nigk5vtzFQXex1s2uFB`,
//! pool `54QGogfaocKyDkvaj2Pcm7495ZNMoF9G8vU6osfyFjMR`): `quote_vault`
//! increased by EXACTLY the instruction's `amount_0` (40,000,000, no fee
//! withheld on the input leg) and `base_vault` decreased by EXACTLY what the
//! user's output account received (8,048,489,049) — the fee is subtracted
//! from the curve's gross OUTPUT before payout and stays inside the vault as
//! an uncollected accumulator (`protocol_base_fee`/`partner_base_fee`/
//! `creator_base_fee`), never physically swept elsewhere on the swap itself.
//! That is consistent with `collect_fee_mode = 1` on that pool's config and
//! with `meteora_damm.rs`'s identical "fee stays in the vault" rule.
//!
//! The rate itself (`cliff_fee_numerator / 1e9`, `FEE_DENOMINATOR` confirmed
//! identical to `meteora_damm.rs`'s own constant) is charged ONCE on the whole
//! swap via `math::fee_amount`, exactly like `meteora_damm.rs` — not per
//! segment crossing like Raydium CLMM/Orca Whirlpool's per-tick fee.
//!
//! # Scope: flat fee only, by refusal rather than a guess
//!
//! Every real config read while building this venue has `dynamic_fee.
//! initialized = 0`, so the volatility-based dynamic fee has no live example
//! to replay and is refused outright (`PoolNotTradable`) rather than guessed
//! at. The base-fee scheduler is more nuanced: `BaseFee`'s `second_factor`/
//! `third_factor`/`first_factor` field NAMES are the IDL's own — deliberately
//! generic ("we can extend that later" per the struct's doc comment) rather
//! than the explicit `periods`/`period_frequency`/`reduction_factor` names
//! `meteora_damm.rs`'s own IDL uses for the same shape — so mapping them by
//! analogy to DAMM v2's scheduler, with no pool to prove the mapping
//! against, is exactly the kind of fabricated constant `adding-a-venue.md`
//! warns about. `scheduler_active()` therefore refuses on the MAGNITUDE
//! fields alone (any of the three factors non-zero), not on `base_fee_mode`:
//! a real pool (config `BPas3hq…`) carries `base_fee_mode = 1` with every
//! factor at zero, and every plausible scheduler shape -- DAMM v2's own
//! included -- degenerates to the identical flat cliff at zero magnitude, so
//! refusing that pool would exclude real volume over a distinction that
//! cannot change the fee this venue charges. A pool whose factors are
//! actually non-zero remains an open scope gap — see the skill's per-venue
//! status table.
//!
//! # The undocumented optional account
//!
//! `referral_token_account` is `optional: true` in the IDL. Both real
//! transactions replayed while building this venue pass the programme's own
//! id (`dbcij3LW…`) in that slot when no referral applies — Anchor's spelling
//! for "absent", the same rule `meteora_damm.rs` and `pumpfun_legacy.rs`
//! already follow. This venue never builds a referral relationship, so it
//! always passes the programme id there.
//!
//! # `swap` vs `swap2`
//!
//! Both are live: 27 `swap` and 30 `swap2` instructions in the 60 most recent
//! signatures against the programme at the time this venue was built. `swap`
//! (`SwapParameters { amount_in, minimum_amount_out }`) is the simpler,
//! exact-in shape that matches this engine's `VenueQuote`/`min_out` contract
//! exactly, so it is what this venue builds; `swap2` additionally supports
//! exact-out and partial-fill modes this engine has no caller for.

use super::layout::{mint_decimals, pubkey_at, token_account_amount, u128_at, u16_at, u8_at};
use super::math::{
    fee_amount, mul_div_ceil, mul_div_floor, mul_shr128_ceil, mul_shr128_floor, shl128_div_floor,
};
use super::token2022::{transfer_fee_schedule, TransferFeeSchedule};
use crate::chains::solana::constants::METEORA_DBC_PROGRAM_ID;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::chains::solana::swaps::direct::error::{DirectSwapError, DirectSwapResult};
use crate::chains::solana::swaps::direct::venue::{
    PoolMarket, PoolVenue, SwapAccounts, VenueQuote,
};
use async_trait::async_trait;
use std::str::FromStr;

/// `sha256("global:swap")[..8]`, confirmed against two live mainnet swaps.
const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];

/// The programme's single, global pool authority. A fixed address per the
/// on-chain IDL (not a derived PDA slot in the instruction), confirmed
/// identical across two different pools' live swaps.
const POOL_AUTHORITY: &str = "FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM";

/// Seed of the Anchor event-CPI authority every instruction carries.
const EVENT_AUTHORITY_SEED: &[u8] = b"__event_authority";

/// `VirtualPool`'s own Anchor discriminator, confirmed against the on-chain
/// IDL and a live pool account's first 8 bytes.
const VIRTUAL_POOL_DISCRIMINATOR: [u8; 8] = [213, 224, 5, 209, 98, 69, 119, 92];

/// `PoolConfig`'s own Anchor discriminator, confirmed the same way.
const POOL_CONFIG_DISCRIMINATOR: [u8; 8] = [26, 108, 14, 123, 116, 230, 129, 43];

/// Denominator every DBC fee numerator is expressed over. Identical to
/// `meteora_damm.rs`'s own `FEE_DENOMINATOR`, both Meteora products.
const FEE_DENOMINATOR: u64 = 1_000_000_000;

/// `collect_fee_mode` values, confirmed against two real pools carrying each.
const COLLECT_FEE_QUOTE_TOKEN: u8 = 0;
const COLLECT_FEE_OUTPUT_TOKEN: u8 = 1;

/// Curve checkpoints a `PoolConfig` may carry.
const MAX_CURVE_POINTS: usize = 20;

/// Compute units this venue's swap needs. Live simulations against both
/// fixture pools measured 57,220 and 59,866 `unitsConsumed`; this leaves
/// comfortable headroom above either without wasting the priority fee, which
/// is charged on the limit requested, not on what a swap actually consumes.
const COMPUTE_UNITS: u32 = 130_000;

/// The venue adapter.
pub struct MeteoraDbcVenue;

#[async_trait]
impl PoolVenue for MeteoraDbcVenue {
    fn program(&self) -> ProgramKind {
        ProgramKind::MeteoraDbc
    }

    fn program_id(&self) -> Pubkey {
        dbc_program_id()
    }

    async fn load(
        &self,
        pool: &Pubkey,
        pool_account: &Account,
    ) -> DirectSwapResult<Box<dyn PoolMarket>> {
        let state = VirtualPoolState::decode(*pool, &pool_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: format!(
                    "VirtualPool did not match the expected layout ({} bytes)",
                    pool_account.data.len()
                ),
            }
        })?;

        if state.is_migrated {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "the curve has migrated; trade the post-migration pool instead".to_owned(),
            });
        }

        // `quote_mint` is not stored on `VirtualPool` itself, only on the
        // config it points at -- and it is needed to build the rest of the
        // address list below, so the config is fetched on its own first
        // rather than folded into one batch with everything else.
        let config_account = get_rpc_client()
            .get_multiple_accounts(&[state.config])
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: state.config,
                detail: format!("PoolConfig could not be read: {e}"),
            })?
            .into_iter()
            .next()
            .flatten()
            .ok_or(DirectSwapError::AccountUnavailable {
                address: state.config,
                detail: "account does not exist".to_owned(),
            })?;

        let config = PoolConfigState::decode(&config_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "PoolConfig did not match the expected layout".to_owned(),
            }
        })?;

        if config.collect_fee_mode != COLLECT_FEE_QUOTE_TOKEN
            && config.collect_fee_mode != COLLECT_FEE_OUTPUT_TOKEN
        {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: format!(
                    "unrecognised collect_fee_mode {} -- refusing rather than guessing which \
                     leg pays",
                    config.collect_fee_mode
                ),
            });
        }
        if config.scheduler_active() {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "the base fee scheduler is active on this pool and this venue's \
                         field mapping for it is unverified against a live pool; refusing \
                         rather than guessing the decay formula"
                    .to_owned(),
            });
        }
        if config.dynamic_fee_initialized {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "the volatility-based dynamic fee is active on this pool and this \
                         venue's formula for it is unverified against a live pool; refusing \
                         rather than guessing"
                    .to_owned(),
            });
        }
        if config.curve.is_empty() {
            return Err(DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "PoolConfig carries no curve points".to_owned(),
            });
        }

        let addresses = [
            state.base_mint,
            config.quote_mint,
            state.base_vault,
            state.quote_vault,
        ];
        let accounts = get_rpc_client()
            .get_multiple_accounts(&addresses)
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: *pool,
                detail: format!("DBC accounts could not be read: {e}"),
            })?;

        let required = |index: usize| -> DirectSwapResult<&Account> {
            accounts.get(index).and_then(Option::as_ref).ok_or(
                DirectSwapError::AccountUnavailable {
                    address: addresses[index],
                    detail: "account does not exist".to_owned(),
                },
            )
        };

        let base_mint_account = required(0)?;
        let quote_mint_account = required(1)?;
        let base_vault_account = required(2)?;
        let quote_vault_account = required(3)?;

        let base_decimals = mint_decimals(&base_mint_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "base_mint is not a mint account".to_owned(),
            }
        })?;
        let quote_decimals = mint_decimals(&quote_mint_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "quote_mint is not a mint account".to_owned(),
            }
        })?;
        let base_vault_balance =
            token_account_amount(&base_vault_account.data).ok_or_else(|| {
                DirectSwapError::PoolUndecodable {
                    pool: *pool,
                    detail: "base_vault is not a token account".to_owned(),
                }
            })?;
        let quote_vault_balance =
            token_account_amount(&quote_vault_account.data).ok_or_else(|| {
                DirectSwapError::PoolUndecodable {
                    pool: *pool,
                    detail: "quote_vault is not a token account".to_owned(),
                }
            })?;

        Ok(Box::new(DbcMarket {
            state,
            config,
            base_decimals,
            quote_decimals,
            base_token_program: base_mint_account.owner,
            quote_token_program: quote_mint_account.owner,
            base_transfer_fee: transfer_fee_schedule(base_mint_account),
            quote_transfer_fee: transfer_fee_schedule(quote_mint_account),
            base_vault_balance,
            quote_vault_balance,
        }))
    }
}

/// The parts of a DBC `VirtualPool` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualPoolState {
    pub pool: Pubkey,
    pub config: Pubkey,
    pub base_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub sqrt_price: u128,
    pub is_migrated: bool,
}

impl VirtualPoolState {
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        if data.len() < 306 || data[0..8] != VIRTUAL_POOL_DISCRIMINATOR {
            return None;
        }
        Some(Self {
            pool,
            config: pubkey_at(data, 72)?,
            base_mint: pubkey_at(data, 136)?,
            base_vault: pubkey_at(data, 168)?,
            quote_vault: pubkey_at(data, 200)?,
            sqrt_price: u128_at(data, 280)?,
            is_migrated: u8_at(data, 305)? != 0,
        })
    }
}

/// One `LiquidityDistributionConfig` checkpoint: the curve's price at this
/// point and the constant liquidity of the segment ENDING here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CurvePoint {
    sqrt_price: u128,
    liquidity: u128,
}

/// The parts of a DBC `PoolConfig` a swap needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolConfigState {
    pub quote_mint: Pubkey,
    pub cliff_fee_numerator: u64,
    pub second_factor: u64,
    pub third_factor: u64,
    pub first_factor: u16,
    pub base_fee_mode: u8,
    pub dynamic_fee_initialized: bool,
    pub collect_fee_mode: u8,
    pub sqrt_start_price: u128,
    curve: Vec<CurvePoint>,
}

impl PoolConfigState {
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 1048 || data[0..8] != POOL_CONFIG_DISCRIMINATOR {
            return None;
        }
        let mut curve = Vec::with_capacity(MAX_CURVE_POINTS);
        for i in 0..MAX_CURVE_POINTS {
            let offset = 408 + i * 32;
            let sqrt_price = u128_at(data, offset)?;
            let liquidity = u128_at(data, offset + 16)?;
            if sqrt_price == 0 {
                break;
            }
            curve.push(CurvePoint {
                sqrt_price,
                liquidity,
            });
        }
        Some(Self {
            quote_mint: pubkey_at(data, 8)?,
            cliff_fee_numerator: super::layout::u64_at(data, 104)?,
            second_factor: super::layout::u64_at(data, 112)?,
            third_factor: super::layout::u64_at(data, 120)?,
            first_factor: u16_at(data, 128)?,
            base_fee_mode: u8_at(data, 130)?,
            dynamic_fee_initialized: u8_at(data, 136)? != 0,
            collect_fee_mode: u8_at(data, 232)?,
            sqrt_start_price: u128_at(data, 392)?,
            curve,
        })
    }

    /// Whether the base fee is anything other than the flat `cliff_fee_numerator`.
    ///
    /// Only the MAGNITUDE fields decide this, not `base_fee_mode` alone: a
    /// real pool found while building this venue (config `BPas3hq…`) carries
    /// `base_fee_mode = 1` with every one of `second_factor`/`third_factor`/
    /// `first_factor` at zero. Whatever unverified scheduler shape mode 1
    /// selects, EVERY plausible one (DAMM v2's own `TimeScheduler`, this
    /// venue's closest sibling, included) degenerates to the flat cliff when
    /// its step size or period count is zero -- a zero-magnitude "linear"
    /// and a zero-magnitude "exponential" decay are the identical constant.
    /// Refusing this pool anyway would be refusing to trade a large share of
    /// real DBC pools over a distinction that cannot affect the fee this
    /// venue actually charges.
    pub fn scheduler_active(&self) -> bool {
        self.second_factor != 0 || self.third_factor != 0 || self.first_factor != 0
    }

    /// The real, non-zero curve checkpoints this config carries -- exposed for
    /// the offline test tier to assert they are genuine segments, not padding.
    pub fn curve_points(&self) -> Vec<(u128, u128)> {
        self.curve
            .iter()
            .map(|p| (p.sqrt_price, p.liquidity))
            .collect()
    }

    /// Ascending sqrt-price boundaries: `[sqrt_start_price, curve[0].sqrt_price,
    /// curve[1].sqrt_price, ...]`, one more entry than there are segments.
    fn boundaries(&self) -> Vec<u128> {
        let mut b = Vec::with_capacity(self.curve.len() + 1);
        b.push(self.sqrt_start_price);
        b.extend(self.curve.iter().map(|p| p.sqrt_price));
        b
    }

    fn liquidities(&self) -> Vec<u128> {
        self.curve.iter().map(|p| p.liquidity).collect()
    }
}

/// A decoded, quotable DBC bonding curve.
#[derive(Debug, Clone)]
pub struct DbcMarket {
    state: VirtualPoolState,
    config: PoolConfigState,
    base_decimals: u8,
    quote_decimals: u8,
    base_token_program: Pubkey,
    quote_token_program: Pubkey,
    base_transfer_fee: Option<TransferFeeSchedule>,
    quote_transfer_fee: Option<TransferFeeSchedule>,
    base_vault_balance: u64,
    quote_vault_balance: u64,
}

impl DbcMarket {
    /// Build a market directly from decoded parts, for the offline test tier.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: VirtualPoolState,
        config: PoolConfigState,
        base_decimals: u8,
        quote_decimals: u8,
        base_token_program: Pubkey,
        quote_token_program: Pubkey,
        base_transfer_fee: Option<TransferFeeSchedule>,
        quote_transfer_fee: Option<TransferFeeSchedule>,
        base_vault_balance: u64,
        quote_vault_balance: u64,
    ) -> Self {
        Self {
            state,
            config,
            base_decimals,
            quote_decimals,
            base_token_program,
            quote_token_program,
            base_transfer_fee,
            quote_transfer_fee,
            base_vault_balance,
            quote_vault_balance,
        }
    }

    fn is_base(&self, mint: &Pubkey) -> Option<bool> {
        if *mint == self.state.base_mint {
            Some(true)
        } else if *mint == self.config.quote_mint {
            Some(false)
        } else {
            None
        }
    }

    /// Whether the platform fee is taken off the INPUT leg for this direction.
    /// `QuoteToken` always charges whichever leg is quote; `OutputToken`
    /// always charges the output leg. Mirrors `meteora_damm.rs::fee_on_input`.
    fn fee_on_input(&self, input_is_base: bool) -> bool {
        match self.config.collect_fee_mode {
            COLLECT_FEE_QUOTE_TOKEN => !input_is_base,
            COLLECT_FEE_OUTPUT_TOKEN => false,
            _ => false,
        }
    }

    fn fee_on(&self, amount: u64) -> u64 {
        fee_amount(amount, self.config.cliff_fee_numerator, FEE_DENOMINATOR)
    }

    /// Walk the curve's segments from the pool's current price, in the
    /// direction `zero_for_one` implies, consuming `amount_in` (already net of
    /// any platform fee and Token-2022 transfer fee on the input leg).
    /// Refuses rather than approximates once the walk would need a segment
    /// past what `PoolConfig` actually carried.
    fn walk(&self, zero_for_one: bool, amount_in: u64) -> DirectSwapResult<(u64, u128)> {
        let boundaries = self.config.boundaries();
        let liquidities = self.config.liquidities();
        let segments = liquidities.len();

        let mut idx = current_segment_index(&boundaries, self.state.sqrt_price);
        let mut sqrt_price = self.state.sqrt_price;
        let mut remaining = amount_in as u128;
        let mut total_out: u128 = 0;

        while remaining > 0 {
            if idx >= segments {
                return Err(DirectSwapError::InsufficientLiquidity {
                    pool: self.state.pool,
                    amount_in,
                    detail: "the swap would travel past the curve segments this venue loaded"
                        .to_owned(),
                });
            }
            let liquidity = liquidities[idx];
            if liquidity == 0 {
                return Err(DirectSwapError::InsufficientLiquidity {
                    pool: self.state.pool,
                    amount_in,
                    detail: "a loaded curve segment carries zero liquidity".to_owned(),
                });
            }
            let boundary = if zero_for_one {
                boundaries[idx]
            } else {
                boundaries[idx + 1]
            };
            let input_needed = input_for_move(zero_for_one, liquidity, sqrt_price, boundary)?;

            if remaining < input_needed {
                let next_price =
                    next_sqrt_price_from_input(zero_for_one, liquidity, sqrt_price, remaining)?;
                let out = output_for_move(zero_for_one, liquidity, sqrt_price, next_price)?;
                total_out += out as u128;
                sqrt_price = next_price;
                remaining = 0;
            } else {
                let out = output_for_move(zero_for_one, liquidity, sqrt_price, boundary)?;
                total_out += out as u128;
                remaining = remaining.saturating_sub(input_needed);
                sqrt_price = boundary;
                if zero_for_one {
                    match idx.checked_sub(1) {
                        Some(next) => idx = next,
                        None => {
                            if remaining > 0 {
                                return Err(DirectSwapError::InsufficientLiquidity {
                                    pool: self.state.pool,
                                    amount_in,
                                    detail: "the sell would push the curve below its own \
                                             starting price"
                                        .to_owned(),
                                });
                            }
                        }
                    }
                } else {
                    idx += 1;
                }
            }
        }

        Ok((total_out.min(u64::MAX as u128) as u64, sqrt_price))
    }
}

impl PoolMarket for DbcMarket {
    fn program(&self) -> ProgramKind {
        ProgramKind::MeteoraDbc
    }

    fn pool(&self) -> Pubkey {
        self.state.pool
    }

    fn mints(&self) -> (Pubkey, Pubkey) {
        (self.state.base_mint, self.config.quote_mint)
    }

    fn token_program(&self, mint: &Pubkey) -> Option<Pubkey> {
        self.is_base(mint).map(|base| {
            if base {
                self.base_token_program
            } else {
                self.quote_token_program
            }
        })
    }

    fn decimals(&self, mint: &Pubkey) -> Option<u8> {
        self.is_base(mint).map(|base| {
            if base {
                self.base_decimals
            } else {
                self.quote_decimals
            }
        })
    }

    fn quote(&self, input_mint: &Pubkey, amount_in: u64) -> DirectSwapResult<VenueQuote> {
        let input_is_base = self
            .is_base(input_mint)
            .ok_or(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: *input_mint,
                output_mint: Pubkey::default(),
            })?;
        if amount_in == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "zero input".to_owned(),
            });
        }

        let input_transfer_fee = if input_is_base {
            self.base_transfer_fee.as_ref()
        } else {
            self.quote_transfer_fee.as_ref()
        };
        let output_transfer_fee = if input_is_base {
            self.quote_transfer_fee.as_ref()
        } else {
            self.base_transfer_fee.as_ref()
        };

        let received_by_pool = super::token2022::net_of_fee(input_transfer_fee, amount_in);
        let zero_for_one = input_is_base;
        let fee_on_input = self.fee_on_input(input_is_base);

        let (curve_input, input_side_fee) = if fee_on_input {
            let fee = self.fee_on(received_by_pool);
            (received_by_pool.saturating_sub(fee), fee)
        } else {
            (received_by_pool, 0)
        };

        let (curve_out, sqrt_price_next) = self.walk(zero_for_one, curve_input)?;

        let (gross_out, output_side_fee) = if fee_on_input {
            (curve_out, 0)
        } else {
            let fee = self.fee_on(curve_out);
            (curve_out.saturating_sub(fee), fee)
        };

        let output_vault_balance = if input_is_base {
            self.quote_vault_balance
        } else {
            self.base_vault_balance
        };
        if gross_out >= output_vault_balance {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the output exceeds what the pool's vault holds".to_owned(),
            });
        }

        let expected_out = super::token2022::net_of_fee(output_transfer_fee, gross_out);
        if expected_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the curve returns nothing at this size".to_owned(),
            });
        }

        // Report the platform fee in INPUT units either way, the contract
        // `VenueQuote::lp_fee` documents and `meteora_damm.rs` already keeps:
        // an output-side fee is converted back at this fill's own realised
        // rate rather than at a stale spot price.
        let lp_fee = if fee_on_input {
            input_side_fee
        } else if curve_out == 0 {
            0
        } else {
            (((output_side_fee as u128) * (curve_input as u128)) / (curve_out as u128)) as u64
        };

        // Impact is the move in the price itself, i.e. in the SQUARED sqrt
        // price -- identical formula to `meteora_damm.rs`, the sibling this
        // curve shape matches.
        let before = (self.state.sqrt_price as f64) / (2.0_f64).powi(64);
        let after = (sqrt_price_next as f64) / (2.0_f64).powi(64);
        let price_impact_pct = if before > 0.0 {
            ((before * before - after * after).abs() / (before * before) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        Ok(VenueQuote {
            amount_in,
            expected_out,
            lp_fee,
            price_impact_pct,
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
                    pool: self.state.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        let output_is_base =
            self.is_base(&accounts.output_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.state.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        if input_is_base == output_is_base {
            return Err(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            });
        }

        let program = dbc_program_id();
        let mut data = Vec::with_capacity(24);
        data.extend_from_slice(&SWAP);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&min_out.to_le_bytes());

        Ok(Instruction {
            program_id: program,
            accounts: vec![
                AccountMeta::new_readonly(pool_authority(), false),
                AccountMeta::new_readonly(self.state.config, false),
                AccountMeta::new(self.state.pool, false),
                AccountMeta::new(accounts.input_token_account, false),
                AccountMeta::new(accounts.output_token_account, false),
                AccountMeta::new(self.state.base_vault, false),
                AccountMeta::new(self.state.quote_vault, false),
                AccountMeta::new_readonly(self.state.base_mint, false),
                AccountMeta::new_readonly(self.config.quote_mint, false),
                AccountMeta::new_readonly(accounts.owner, true),
                AccountMeta::new_readonly(self.base_token_program, false),
                AccountMeta::new_readonly(self.quote_token_program, false),
                // `referral_token_account` is optional. Anchor's convention for
                // an absent optional account is the programme's own id, which
                // is exactly what both real transactions replayed while
                // building this venue pass.
                AccountMeta::new_readonly(program, false),
                AccountMeta::new_readonly(event_authority(), false),
                AccountMeta::new_readonly(program, false),
            ],
            data,
        })
    }

    fn compute_units(&self) -> u32 {
        COMPUTE_UNITS
    }
}

// ============================================================================
// DOUBLE-Q64.64 STEP MATH
// ============================================================================
//
// `liquidity` here carries the SAME 2^64 fixed-point scale as `sqrt_price`
// (established by replay -- see the module docs), so the combined scale in
// every formula below is 2^128, not `clmm_ticks`'s single 2^64. `2^128` does
// not fit in a `u128` denominator, so these use `math::mul_shr128_*` /
// `math::shl128_div_*` (an exact divide-by-2^128, taken straight off the high
// limb of a 256-bit product) instead of `clmm_ticks`'s `mul_div_*`.

fn quote_math_overflow() -> DirectSwapError {
    DirectSwapError::QuoteMath {
        detail: "the DBC curve step overflowed 256 bits".to_owned(),
    }
}

/// The output produced by moving the price at constant `liquidity` from
/// `sqrt_from` to `sqrt_to`, rounded DOWN.
fn output_for_move(
    zero_for_one: bool,
    liquidity: u128,
    sqrt_from: u128,
    sqrt_to: u128,
) -> DirectSwapResult<u64> {
    let out = if zero_for_one {
        // Output is quote: Δy = L_real·Δ√P/2^64 = liquidity·Δ√P/2^128.
        let spread = sqrt_from.saturating_sub(sqrt_to);
        mul_shr128_floor(liquidity, spread)
    } else {
        // Output is base: Δx = liquidity·Δ√P/(√P_to·√P_from).
        let spread = sqrt_to.saturating_sub(sqrt_from);
        let step1 = mul_div_floor(liquidity, spread, sqrt_to).ok_or_else(quote_math_overflow)?;
        step1 / sqrt_from.max(1)
    };
    Ok(out.min(u64::MAX as u128) as u64)
}

/// The amount of the input side needed to move the price EXACTLY from
/// `sqrt_from` to `sqrt_to` at constant `liquidity`, rounded UP -- understating
/// this would cross the segment boundary without having paid for it.
fn input_for_move(
    zero_for_one: bool,
    liquidity: u128,
    sqrt_from: u128,
    sqrt_to: u128,
) -> DirectSwapResult<u128> {
    if zero_for_one {
        // Base in, price falling: sqrt_from (high) > sqrt_to (low).
        if sqrt_from <= sqrt_to {
            return Ok(0);
        }
        let spread = sqrt_from - sqrt_to;
        let step1 = mul_div_ceil(liquidity, spread, sqrt_from).ok_or_else(quote_math_overflow)?;
        Ok(step1.div_ceil(sqrt_to.max(1)))
    } else {
        // Quote in, price rising: sqrt_to (high) > sqrt_from (low).
        if sqrt_to <= sqrt_from {
            return Ok(0);
        }
        let spread = sqrt_to - sqrt_from;
        Ok(mul_shr128_ceil(liquidity, spread))
    }
}

/// The sqrt price reached by spending `amount_in` (already net of every fee)
/// at constant `liquidity` from `sqrt_price` -- the partial-step case, when
/// the input runs out before the segment boundary. Rounds in the direction
/// that under-states the resulting move, the same conservative direction
/// `clmm_ticks::next_sqrt_price_from_input` rounds in.
fn next_sqrt_price_from_input(
    zero_for_one: bool,
    liquidity: u128,
    sqrt_price: u128,
    amount_in: u128,
) -> DirectSwapResult<u128> {
    if zero_for_one {
        // Base in: sqrtQ = ceil(L·√P / (L + amount·√P)). `amount·√P` can
        // exceed a `u128` for an adversarial size against a near-max-price
        // segment; refused rather than wrapped, the same way every other
        // overflow in this walk is refused rather than guessed at.
        let product = amount_in
            .checked_mul(sqrt_price)
            .ok_or_else(quote_math_overflow)?;
        let denominator = liquidity
            .checked_add(product)
            .ok_or_else(quote_math_overflow)?;
        mul_div_ceil(liquidity, sqrt_price, denominator).ok_or_else(quote_math_overflow)
    } else {
        // Quote in: ΔsqrtP = amount·2^128/liquidity, floor.
        let delta = shl128_div_floor(amount_in, liquidity).ok_or_else(quote_math_overflow)?;
        sqrt_price
            .checked_add(delta)
            .ok_or_else(quote_math_overflow)
    }
}

/// The segment index containing `price`: the last boundary `<= price`, capped
/// to the final segment. `boundaries` has `segments + 1` entries.
fn current_segment_index(boundaries: &[u128], price: u128) -> usize {
    let segments = boundaries.len().saturating_sub(1);
    if segments == 0 {
        return 0;
    }
    for i in 0..segments {
        if price <= boundaries[i + 1] {
            return i;
        }
    }
    segments - 1
}

fn dbc_program_id() -> Pubkey {
    Pubkey::from_str(METEORA_DBC_PROGRAM_ID).expect("DBC program id constant is valid")
}

fn pool_authority() -> Pubkey {
    Pubkey::from_str(POOL_AUTHORITY).expect("DBC pool authority constant is valid")
}

fn event_authority() -> Pubkey {
    Pubkey::find_program_address(&[EVENT_AUTHORITY_SEED], &dbc_program_id()).0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(
        collect_fee_mode: u8,
        cliff_fee_numerator: u64,
        sqrt_start_price: u128,
        curve: Vec<(u128, u128)>,
    ) -> PoolConfigState {
        PoolConfigState {
            quote_mint: Pubkey::new_unique(),
            cliff_fee_numerator,
            second_factor: 0,
            third_factor: 0,
            first_factor: 0,
            base_fee_mode: 0,
            dynamic_fee_initialized: false,
            collect_fee_mode,
            sqrt_start_price,
            curve: curve
                .into_iter()
                .map(|(sqrt_price, liquidity)| CurvePoint {
                    sqrt_price,
                    liquidity,
                })
                .collect(),
        }
    }

    fn market(cfg: PoolConfigState, sqrt_price: u128) -> DbcMarket {
        let pool = Pubkey::new_unique();
        DbcMarket::new(
            VirtualPoolState {
                pool,
                config: Pubkey::new_unique(),
                base_mint: Pubkey::new_unique(),
                base_vault: Pubkey::new_unique(),
                quote_vault: Pubkey::new_unique(),
                sqrt_price,
                is_migrated: false,
            },
            cfg,
            6,
            9,
            crate::chains::solana::spl_token::id(),
            crate::chains::solana::spl_token::id(),
            None,
            None,
            u64::MAX / 2,
            u64::MAX / 2,
        )
    }

    #[test]
    fn a_single_segment_buy_and_sell_are_inverse_directions() {
        let cfg = config(
            COLLECT_FEE_OUTPUT_TOKEN,
            2_500_000, // 0.25%, the real A95th9Y… rate
            1u128 << 64,
            vec![((2u128) << 64, 1_000_000_000_000u128 << 64)],
        );
        // Sitting mid-segment, so both directions have somewhere to walk to:
        // a sell needs room below the current price, a buy room above it.
        let m = market(cfg, (1u128 << 64) + (1u128 << 63));
        let base = m.state.base_mint;
        let quote = m.config.quote_mint;

        let buy = m.quote(&quote, 1_000_000_000).expect("buy");
        assert!(buy.expected_out > 0);
        let sell = m.quote(&base, 1_000_000).expect("sell");
        assert!(sell.expected_out > 0);
    }

    #[test]
    fn a_flat_fee_at_zero_charges_nothing() {
        let cfg = config(
            COLLECT_FEE_OUTPUT_TOKEN,
            0,
            1u128 << 64,
            vec![((2u128) << 64, 1_000_000_000_000u128 << 64)],
        );
        let m = market(cfg, 1u128 << 64);
        let quote = m.config.quote_mint;
        let q = m.quote(&quote, 1_000_000_000).expect("buy");
        assert_eq!(q.lp_fee, 0);
    }

    #[test]
    fn a_buy_that_would_cross_past_the_loaded_curve_is_refused() {
        let cfg = config(
            COLLECT_FEE_OUTPUT_TOKEN,
            2_500_000,
            1u128 << 64,
            vec![((2u128) << 64, 1u128 << 64)], // tiny liquidity, one segment
        );
        let m = market(cfg, 1u128 << 64);
        let quote = m.config.quote_mint;
        let result = m.quote(&quote, u64::MAX / 4);
        assert!(matches!(
            result,
            Err(DirectSwapError::InsufficientLiquidity { .. })
        ));
    }

    #[test]
    fn a_sell_below_the_starting_price_is_refused() {
        let cfg = config(
            COLLECT_FEE_OUTPUT_TOKEN,
            2_500_000,
            1u128 << 64,
            vec![((2u128) << 64, 1_000_000_000_000u128 << 64)],
        );
        // Pool sitting exactly at its own floor: any sell has nowhere to go.
        let m = market(cfg, 1u128 << 64);
        let base = m.state.base_mint;
        let result = m.quote(&base, u64::MAX / 4);
        assert!(matches!(
            result,
            Err(DirectSwapError::InsufficientLiquidity { .. })
        ));
    }

    #[test]
    fn a_scheduler_or_dynamic_fee_pool_is_a_load_time_refusal_not_a_guess() {
        let mut cfg = config(
            COLLECT_FEE_OUTPUT_TOKEN,
            2_500_000,
            1u128 << 64,
            vec![((2u128) << 64, 1_000_000_000_000u128 << 64)],
        );
        assert!(!cfg.scheduler_active());
        cfg.first_factor = 3;
        assert!(cfg.scheduler_active());
    }

    #[test]
    fn a_nonzero_base_fee_mode_with_every_factor_at_zero_is_still_flat() {
        // A real pool (config BPas3hq…) carries base_fee_mode = 1 with every
        // factor at zero -- every plausible scheduler shape degenerates to
        // the flat cliff at zero magnitude, so this must NOT be refused.
        let mut cfg = config(
            COLLECT_FEE_OUTPUT_TOKEN,
            2_500_000,
            1u128 << 64,
            vec![((2u128) << 64, 1_000_000_000_000u128 << 64)],
        );
        cfg.base_fee_mode = 1;
        assert!(!cfg.scheduler_active());
    }

    #[test]
    fn current_segment_index_finds_the_boundary_containing_the_price() {
        let boundaries = vec![10u128, 20u128, 30u128];
        assert_eq!(current_segment_index(&boundaries, 5), 0);
        assert_eq!(current_segment_index(&boundaries, 15), 0);
        assert_eq!(current_segment_index(&boundaries, 20), 0);
        assert_eq!(current_segment_index(&boundaries, 25), 1);
        assert_eq!(current_segment_index(&boundaries, 100), 1);
    }

    #[test]
    fn the_pool_authority_and_event_authority_match_live_transactions() {
        assert_eq!(
            pool_authority().to_string(),
            "FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM"
        );
        assert_eq!(
            event_authority().to_string(),
            "8Ks12pbrD6PXxfty1hVQiE9sc289zgU1zHkvXhrSdriF"
        );
    }
}
