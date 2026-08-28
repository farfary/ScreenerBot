//! Meteora DAMM v2 (`cpamdpZC…`), the programme Meteora calls `cp-amm`.
//!
//! # Layout, verified against mainnet
//!
//! `Pool` is 1112 bytes. Every offset below was read back off live pools and
//! cross-checked against the programme's own on-chain Anchor IDL:
//!
//! ```text
//!   8 pool_fees.base_fee (32 bytes, a mode-tagged union)
//!  48 protocol_fee_percent   50 referral_fee_percent   54 compounding_fee_bps
//!  56 dynamic_fee (initialized u8, then the volatility state)
//! 168 token_a_mint  200 token_b_mint  232 token_a_vault  264 token_b_vault
//! 360 liquidity u128 (Q64.64)         392 protocol_a_fee  400 protocol_b_fee
//! 424 sqrt_min_price 440 sqrt_max_price 456 sqrt_price (all Q64.64)
//! 472 activation_point  480 activation_type  481 pool_status
//! 482 token_a_flag      483 token_b_flag     484 collect_fee_mode
//! ```
//!
//! # The curve
//!
//! A DAMM v2 pool is concentrated liquidity with ONE position spanning
//! `sqrt_min_price..sqrt_max_price`, so the active liquidity is the same at
//! every price inside that band. That is what makes a single constant-liquidity
//! step EXACT here, unlike a tick-based CLMM where the step is only exact inside
//! the current tick range.
//!
//! Both `liquidity` and `sqrt_price` are Q64.64, so:
//!
//! ```text
//! Δb = L · Δ√P >> 128
//! Δa = L · Δ√P / (√P_low · √P_high)
//! ```
//!
//! # The fee
//!
//! `base_fee` is a 32-byte union tagged by `base_fee_mode` at its own offset 8.
//! Mode 0/1 are a time scheduler that only ever REDUCES the fee from its cliff;
//! mode 2 is a rate limiter that raises the fee with size for a bounded window
//! after activation. Anything else falls back to the cliff, which is the highest
//! fee any scheduler can charge — that under-states the output, which lowers
//! `min_out`, which is the direction that still fills.
//!
//! The dynamic fee is added on top from the pool's stored volatility
//! accumulator. That value decays with time and is read here WITHOUT applying
//! the decay, which again can only over-state the fee.
//!
//! `collect_fee_mode` decides which leg pays it: mode 1 (`OnlyB`) charges the
//! input when token B is being spent and the output otherwise; modes 0 and 2
//! always charge the output.

use super::layout::{
    mint_decimals, pubkey_at, token_account_amount, u128_at, u16_at, u32_at, u64_at, u8_at,
};
use super::math::{mul_div_ceil, mul_div_floor};
use super::token2022::{transfer_fee_schedule, TransferFeeSchedule};
use crate::chains::solana::constants::METEORA_DAMM_PROGRAM_ID;
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

/// `sha256("global:swap")[..8]`, confirmed against live mainnet swaps.
const SWAP: [u8; 8] = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];

/// The programme's single, global pool authority. It is a fixed address rather
/// than a per-pool PDA, and live swaps pass exactly this account.
const POOL_AUTHORITY: &str = "HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC";

/// Seed of the Anchor event-CPI authority every cp-amm instruction carries.
const EVENT_AUTHORITY_SEED: &[u8] = b"__event_authority";

/// Denominator every cp-amm fee numerator is expressed over.
const FEE_DENOMINATOR: u64 = 1_000_000_000;

/// Highest fee numerator the programme will charge, i.e. 50%.
const MAX_FEE_NUMERATOR: u64 = 500_000_000;

/// Basis-point denominator used by the fee schedulers.
const BASIS_POINT_MAX: u64 = 10_000;

/// `collect_fee_mode` values.
const COLLECT_FEE_BOTH_TOKEN: u8 = 0;
const COLLECT_FEE_ONLY_B: u8 = 1;
const COLLECT_FEE_COMPOUNDING: u8 = 2;

/// `activation_type` values: the point a fee scheduler measures against.
const ACTIVATION_TYPE_SLOT: u8 = 0;

/// Compute units a cp-amm swap needs. Measured against mainnet simulations.
const COMPUTE_UNITS: u32 = 180_000;

/// The venue adapter.
pub struct MeteoraDammVenue;

#[async_trait]
impl PoolVenue for MeteoraDammVenue {
    fn program(&self) -> ProgramKind {
        ProgramKind::MeteoraDamm
    }

    fn program_id(&self) -> Pubkey {
        damm_program_id()
    }

    async fn load(
        &self,
        pool: &Pubkey,
        pool_account: &Account,
    ) -> DirectSwapResult<Box<dyn PoolMarket>> {
        let state = DammPoolState::decode(*pool, &pool_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: format!(
                    "DAMM v2 pool state did not match the expected layout ({} bytes)",
                    pool_account.data.len()
                ),
            }
        })?;

        if state.pool_status != 0 {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: format!("pool_status {} is not enabled", state.pool_status),
            });
        }
        if state.liquidity == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: *pool,
                amount_in: 0,
                detail: "the pool holds no liquidity".to_owned(),
            });
        }

        let addresses = [state.vault_a, state.vault_b, state.mint_a, state.mint_b];
        let accounts = get_rpc_client()
            .get_multiple_accounts(&addresses)
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: *pool,
                detail: format!("DAMM v2 pool accounts could not be read: {e}"),
            })?;

        let required = |index: usize| -> DirectSwapResult<&Account> {
            accounts.get(index).and_then(Option::as_ref).ok_or(
                DirectSwapError::AccountUnavailable {
                    address: addresses[index],
                    detail: "account does not exist".to_owned(),
                },
            )
        };

        let vault_a_balance = token_account_amount(&required(0)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_a vault is not a token account".to_owned(),
            }
        })?;
        let vault_b_balance = token_account_amount(&required(1)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_b vault is not a token account".to_owned(),
            }
        })?;
        let mint_a = required(2)?;
        let mint_b = required(3)?;
        let decimals_a =
            mint_decimals(&mint_a.data).ok_or_else(|| DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_a mint is not a mint account".to_owned(),
            })?;
        let decimals_b =
            mint_decimals(&mint_b.data).ok_or_else(|| DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_b mint is not a mint account".to_owned(),
            })?;

        // The fee schedulers measure against either the slot or the unix clock,
        // which is the loader's job to read so the market itself stays pure.
        let current_point = if state.activation_type == ACTIVATION_TYPE_SLOT {
            get_rpc_client().get_slot().await.unwrap_or(u64::MAX)
        } else {
            chrono::Utc::now().timestamp().max(0) as u64
        };

        Ok(Box::new(DammMarket {
            state,
            token_program_a: mint_a.owner,
            token_program_b: mint_b.owner,
            decimals_a,
            decimals_b,
            vault_a_balance,
            vault_b_balance,
            transfer_fee_a: transfer_fee_schedule(mint_a),
            transfer_fee_b: transfer_fee_schedule(mint_b),
            current_point,
        }))
    }
}

/// The base-fee union, already resolved to the one variant its tag selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseFee {
    /// A fee that starts at `cliff` and steps DOWN over time. `linear` selects
    /// between subtracting `reduction_factor` per period and compounding a
    /// `reduction_factor` basis-point cut per period.
    TimeScheduler {
        cliff: u64,
        periods: u16,
        period_frequency: u64,
        reduction_factor: u64,
        linear: bool,
    },
    /// A fee that steps UP with trade size, for a bounded window after the pool
    /// activates. Only the leg that spends token B is rate-limited.
    RateLimiter {
        cliff: u64,
        fee_increment_bps: u16,
        max_limiter_duration: u32,
        max_fee_bps: u32,
        reference_amount: u64,
    },
    /// Every other tag, including the market-cap scheduler: the cliff is the
    /// highest fee any scheduler charges, so quoting at it can only under-state
    /// the output.
    Cliff { cliff: u64 },
}

impl BaseFee {
    /// Decode the 32-byte union at `Pool::pool_fees.base_fee`.
    pub fn decode(data: &[u8], offset: usize) -> Option<Self> {
        let cliff = u64_at(data, offset)?;
        let mode = u8_at(data, offset + 8)?;
        Some(match mode {
            0 | 1 => Self::TimeScheduler {
                cliff,
                periods: u16_at(data, offset + 14)?,
                period_frequency: u64_at(data, offset + 16)?,
                reduction_factor: u64_at(data, offset + 24)?,
                linear: mode == 0,
            },
            2 => Self::RateLimiter {
                cliff,
                fee_increment_bps: u16_at(data, offset + 14)?,
                max_limiter_duration: u32_at(data, offset + 16)?,
                max_fee_bps: u32_at(data, offset + 20)?,
                reference_amount: u64_at(data, offset + 24)?,
            },
            _ => Self::Cliff { cliff },
        })
    }

    /// The fee numerator this base fee charges for `amount_in`.
    ///
    /// `b_to_a` is the only direction the rate limiter applies to; the other
    /// direction always pays the cliff.
    pub fn numerator(
        &self,
        amount_in: u64,
        current_point: u64,
        activation_point: u64,
        b_to_a: bool,
    ) -> u64 {
        match *self {
            Self::Cliff { cliff } => cliff,
            Self::TimeScheduler {
                cliff,
                periods,
                period_frequency,
                reduction_factor,
                linear,
            } => {
                if period_frequency == 0 || periods == 0 {
                    return cliff;
                }
                let elapsed = current_point.saturating_sub(activation_point);
                let period = (elapsed / period_frequency).min(periods as u64);
                if linear {
                    cliff.saturating_sub(reduction_factor.saturating_mul(period))
                } else {
                    exponential_reduction(cliff, reduction_factor, period)
                }
            }
            Self::RateLimiter {
                cliff,
                fee_increment_bps,
                max_limiter_duration,
                max_fee_bps,
                reference_amount,
            } => {
                let in_window =
                    current_point < activation_point.saturating_add(max_limiter_duration as u64);
                if !b_to_a || !in_window || reference_amount == 0 || fee_increment_bps == 0 {
                    return cliff;
                }
                rate_limited_numerator(
                    amount_in,
                    cliff,
                    fee_increment_bps,
                    max_fee_bps,
                    reference_amount,
                )
            }
        }
    }
}

/// `cliff · ((10_000 − reduction) / 10_000)^period`, in Q64 fixed point.
fn exponential_reduction(cliff: u64, reduction_factor: u64, period: u64) -> u64 {
    if reduction_factor >= BASIS_POINT_MAX {
        return 0;
    }
    let one = 1u128 << 64;
    let ratio = (one * ((BASIS_POINT_MAX - reduction_factor) as u128)) / (BASIS_POINT_MAX as u128);
    let mut result = one;
    let mut base = ratio;
    let mut exponent = period;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = (result * base) >> 64;
        }
        base = (base * base) >> 64;
        exponent >>= 1;
        if base == 0 {
            break;
        }
    }
    (((cliff as u128) * result) >> 64).min(u64::MAX as u128) as u64
}

/// The effective numerator a rate limiter charges for `amount_in`.
///
/// The limiter prices the input in blocks of `reference_amount`: the first block
/// pays the cliff, each further block pays one more `fee_increment` on top, and
/// once the increment would exceed the programme's maximum every remaining unit
/// pays that maximum. The result is the TOTAL fee re-expressed as a single
/// numerator so the rest of this venue can charge it like any other rate.
fn rate_limited_numerator(
    amount_in: u64,
    cliff: u64,
    fee_increment_bps: u16,
    max_fee_bps: u32,
    reference_amount: u64,
) -> u64 {
    if amount_in <= reference_amount {
        return cliff;
    }
    let ceiling = ((max_fee_bps as u64) * FEE_DENOMINATOR / BASIS_POINT_MAX).min(MAX_FEE_NUMERATOR);
    let increment =
        (fee_increment_bps as u128) * (FEE_DENOMINATOR as u128) / (BASIS_POINT_MAX as u128);
    if increment == 0 || ceiling <= cliff {
        return cliff;
    }

    let x0 = reference_amount as u128;
    let c = cliff as u128;
    let excess = (amount_in as u128) - x0;
    let blocks = excess / x0;
    let remainder = excess % x0;
    let max_index = ((ceiling as u128) - c) / increment;

    let fee = if blocks < max_index {
        // x0·(c + c·a + i·a·(a+1)/2) + b·(c + i·(a+1))
        let stepped = c + c * blocks + increment * blocks * (blocks + 1) / 2;
        x0 * stepped + remainder * (c + increment * (blocks + 1))
    } else {
        let capped = c + c * max_index + increment * max_index * (max_index + 1) / 2;
        let beyond = (blocks - max_index) * x0 + remainder;
        x0 * capped + beyond * (ceiling as u128)
    };

    // Re-express the total as a rate over the whole input, rounding UP so the
    // pool is never short-changed by the conversion.
    let numerator = fee.div_ceil(amount_in as u128);
    numerator.min(ceiling as u128) as u64
}

/// The pool's stored dynamic-fee state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DynamicFee {
    pub initialized: bool,
    pub variable_fee_control: u32,
    pub bin_step: u16,
    pub volatility_accumulator: u128,
}

impl DynamicFee {
    /// Decode the dynamic-fee block at `Pool::pool_fees.dynamic_fee`.
    pub fn decode(data: &[u8], offset: usize) -> Option<Self> {
        Some(Self {
            initialized: u8_at(data, offset)? != 0,
            variable_fee_control: u32_at(data, offset + 12)?,
            bin_step: u16_at(data, offset + 16)?,
            volatility_accumulator: u128_at(data, offset + 64)?,
        })
    }

    /// The variable fee numerator this volatility implies.
    ///
    /// Deliberately computed from the STORED accumulator without applying the
    /// time decay the programme would apply first. That can only over-state the
    /// fee, which under-states the output — the safe direction.
    pub fn numerator(&self) -> u64 {
        if !self.initialized || self.variable_fee_control == 0 {
            return 0;
        }
        let scaled = self
            .volatility_accumulator
            .saturating_mul(self.bin_step as u128);
        let square = scaled.saturating_mul(scaled);
        let raw = square.saturating_mul(self.variable_fee_control as u128);
        let fee = raw.saturating_add(99_999_999_999) / 100_000_000_000;
        fee.min(MAX_FEE_NUMERATOR as u128) as u64
    }
}

/// The parts of the cp-amm `Pool` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DammPoolState {
    pub pool: Pubkey,
    pub base_fee: BaseFee,
    pub dynamic_fee: DynamicFee,
    pub collect_fee_mode: u8,
    pub compounding_fee_bps: u16,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub liquidity: u128,
    pub protocol_fee_a: u64,
    pub protocol_fee_b: u64,
    pub sqrt_min_price: u128,
    pub sqrt_max_price: u128,
    pub sqrt_price: u128,
    pub activation_point: u64,
    pub activation_type: u8,
    pub pool_status: u8,
}

impl DammPoolState {
    /// Decode a cp-amm pool account. Pure: no RPC, no cache, no clock.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        Some(Self {
            pool,
            base_fee: BaseFee::decode(data, 8)?,
            dynamic_fee: DynamicFee::decode(data, 56)?,
            compounding_fee_bps: u16_at(data, 54)?,
            mint_a: pubkey_at(data, 168)?,
            mint_b: pubkey_at(data, 200)?,
            vault_a: pubkey_at(data, 232)?,
            vault_b: pubkey_at(data, 264)?,
            liquidity: u128_at(data, 360)?,
            protocol_fee_a: u64_at(data, 392)?,
            protocol_fee_b: u64_at(data, 400)?,
            sqrt_min_price: u128_at(data, 424)?,
            sqrt_max_price: u128_at(data, 440)?,
            sqrt_price: u128_at(data, 456)?,
            activation_point: u64_at(data, 472)?,
            activation_type: u8_at(data, 480)?,
            pool_status: u8_at(data, 481)?,
            collect_fee_mode: u8_at(data, 484)?,
        })
    }
}

/// A decoded, quotable DAMM v2 pool.
#[derive(Debug, Clone)]
pub struct DammMarket {
    state: DammPoolState,
    token_program_a: Pubkey,
    token_program_b: Pubkey,
    decimals_a: u8,
    decimals_b: u8,
    vault_a_balance: u64,
    vault_b_balance: u64,
    transfer_fee_a: Option<TransferFeeSchedule>,
    transfer_fee_b: Option<TransferFeeSchedule>,
    current_point: u64,
}

impl DammMarket {
    /// Build a market directly from decoded parts, for the offline test tier.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: DammPoolState,
        token_program_a: Pubkey,
        token_program_b: Pubkey,
        decimals_a: u8,
        decimals_b: u8,
        vault_a_balance: u64,
        vault_b_balance: u64,
        transfer_fee_a: Option<TransferFeeSchedule>,
        transfer_fee_b: Option<TransferFeeSchedule>,
        current_point: u64,
    ) -> Self {
        Self {
            state,
            token_program_a,
            token_program_b,
            decimals_a,
            decimals_b,
            vault_a_balance,
            vault_b_balance,
            transfer_fee_a,
            transfer_fee_b,
            current_point,
        }
    }

    /// Whether `mint` is the pool's token A side.
    fn is_side_a(&self, mint: &Pubkey) -> Option<bool> {
        if *mint == self.state.mint_a {
            Some(true)
        } else if *mint == self.state.mint_b {
            Some(false)
        } else {
            None
        }
    }

    fn transfer_fee(&self, side_a: bool) -> Option<&TransferFeeSchedule> {
        if side_a {
            self.transfer_fee_a.as_ref()
        } else {
            self.transfer_fee_b.as_ref()
        }
    }

    /// Whether the trade fee is taken off the INPUT rather than the output.
    ///
    /// `OnlyB` collects in token B, so it charges the input exactly when token B
    /// is what is being spent. Every other mode charges the output.
    fn fee_on_input(&self, input_is_a: bool) -> bool {
        match self.state.collect_fee_mode {
            COLLECT_FEE_ONLY_B => !input_is_a,
            COLLECT_FEE_BOTH_TOKEN | COLLECT_FEE_COMPOUNDING => false,
            _ => false,
        }
    }

    /// The total trade-fee numerator for a fill of `amount` in this direction.
    fn fee_numerator(&self, amount: u64, input_is_a: bool) -> u64 {
        let base = self.state.base_fee.numerator(
            amount,
            self.current_point,
            self.state.activation_point,
            !input_is_a,
        );
        base.saturating_add(self.state.dynamic_fee.numerator())
            .min(MAX_FEE_NUMERATOR)
    }

    /// The fee charged on `amount`, rounded UP.
    fn fee_on(&self, amount: u64, input_is_a: bool) -> u64 {
        let numerator = self.fee_numerator(amount, input_is_a);
        if numerator == 0 {
            return 0;
        }
        (((amount as u128) * (numerator as u128)).div_ceil(FEE_DENOMINATOR as u128))
            .min(amount as u128) as u64
    }

    /// One constant-liquidity step: the sqrt price the pool ends at and the raw
    /// output it pays.
    ///
    /// Exact for any size that stays inside `sqrt_min_price..sqrt_max_price`,
    /// because a DAMM v2 pool has a single position spanning that whole band.
    fn step(&self, a_for_b: bool, amount_in: u64) -> DirectSwapResult<(u128, u64)> {
        let liquidity = self.state.liquidity;
        let sqrt_price = self.state.sqrt_price;
        if liquidity == 0 || sqrt_price == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the pool holds no liquidity".to_owned(),
            });
        }
        let overflow = || DirectSwapError::QuoteMath {
            detail: "the DAMM v2 step overflowed 256 bits".to_owned(),
        };
        let amount = amount_in as u128;

        if a_for_b {
            // √P' = L·√P / (L + Δa·√P), rounded UP so the price moves at least
            // as far against the trader as the programme moves it.
            let product = amount.checked_mul(sqrt_price).ok_or_else(overflow)?;
            let denominator = liquidity.checked_add(product).ok_or_else(overflow)?;
            let sqrt_next =
                mul_div_ceil(liquidity, sqrt_price, denominator).ok_or_else(overflow)?;
            if sqrt_next < self.state.sqrt_min_price {
                return Err(DirectSwapError::InsufficientLiquidity {
                    pool: self.state.pool,
                    amount_in,
                    detail: "the size would push the price below the pool's minimum".to_owned(),
                });
            }
            let out = delta_b(sqrt_next, sqrt_price, liquidity).ok_or_else(overflow)?;
            Ok((sqrt_next, out.min(u64::MAX as u128) as u64))
        } else {
            // √P' = √P + Δb·2^128 / L, rounded DOWN.
            // Δ√P = Δb·2^128 / L. `amount << 64` is exact for any `u64` and
            // keeps the second factor a plain 2^64, so the whole thing fits the
            // 256-bit intermediate without losing the low bits.
            let delta = mul_div_floor(amount << 64, 1u128 << 64, liquidity).ok_or_else(overflow)?;
            let sqrt_next = sqrt_price.checked_add(delta).ok_or_else(overflow)?;
            if sqrt_next > self.state.sqrt_max_price {
                return Err(DirectSwapError::InsufficientLiquidity {
                    pool: self.state.pool,
                    amount_in,
                    detail: "the size would push the price above the pool's maximum".to_owned(),
                });
            }
            let out = delta_a(sqrt_price, sqrt_next, liquidity).ok_or_else(overflow)?;
            Ok((sqrt_next, out.min(u64::MAX as u128) as u64))
        }
    }

    /// The vault the output leaves, so a quote never promises more than the pool
    /// physically holds.
    /// Swappable balance of the vault the output leaves.
    ///
    /// The protocol's uncollected share sits in the SAME vault and is not
    /// swappable, so quoting off the raw vault balance would over-state what the
    /// pool can pay.
    fn output_vault_balance(&self, input_is_a: bool) -> u64 {
        if input_is_a {
            self.vault_b_balance
                .saturating_sub(self.state.protocol_fee_b)
        } else {
            self.vault_a_balance
                .saturating_sub(self.state.protocol_fee_a)
        }
    }
}

/// `Δb = L · (√upper − √lower) >> 128`, rounded DOWN. Both operands are Q64.64.
fn delta_b(sqrt_lower: u128, sqrt_upper: u128, liquidity: u128) -> Option<u128> {
    let delta = sqrt_upper.checked_sub(sqrt_lower)?;
    // 2^128 does not fit a u128 divisor, so divide by 2^64 twice: the first
    // division is exact in 256 bits, the second is a shift.
    let half = mul_div_floor(liquidity, delta, 1u128 << 64)?;
    Some(half >> 64)
}

/// `Δa = L · (√upper − √lower) / (√lower · √upper)`, rounded DOWN.
///
/// The denominator is a product of two Q64.64 values and overflows `u128` for
/// any pool trading above about 1e-2 in raw units, so it is applied as two
/// successive divisions. Both round down, which can only under-state the output
/// by a raw unit — the direction a `min_out` can survive.
fn delta_a(sqrt_lower: u128, sqrt_upper: u128, liquidity: u128) -> Option<u128> {
    let delta = sqrt_upper.checked_sub(sqrt_lower)?;
    let intermediate = mul_div_floor(liquidity, delta, sqrt_upper)?;
    Some(intermediate / sqrt_lower)
}

impl PoolMarket for DammMarket {
    fn program(&self) -> ProgramKind {
        ProgramKind::MeteoraDamm
    }

    fn pool(&self) -> Pubkey {
        self.state.pool
    }

    fn mints(&self) -> (Pubkey, Pubkey) {
        (self.state.mint_a, self.state.mint_b)
    }

    fn token_program(&self, mint: &Pubkey) -> Option<Pubkey> {
        self.is_side_a(mint).map(|side_a| {
            if side_a {
                self.token_program_a
            } else {
                self.token_program_b
            }
        })
    }

    fn decimals(&self, mint: &Pubkey) -> Option<u8> {
        self.is_side_a(mint).map(|side_a| {
            if side_a {
                self.decimals_a
            } else {
                self.decimals_b
            }
        })
    }

    fn quote(&self, input_mint: &Pubkey, amount_in: u64) -> DirectSwapResult<VenueQuote> {
        let input_is_a = self
            .is_side_a(input_mint)
            .ok_or(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: *input_mint,
                output_mint: Pubkey::default(),
            })?;

        // Only what survives the input mint's transfer fee ever reaches the pool.
        let received_by_pool =
            super::token2022::net_of_fee(self.transfer_fee(input_is_a), amount_in);
        if received_by_pool == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the input mint's transfer fee consumes the whole amount".to_owned(),
            });
        }

        let fee_on_input = self.fee_on_input(input_is_a);
        let (swappable, input_fee) = if fee_on_input {
            let fee = self.fee_on(received_by_pool, input_is_a);
            (received_by_pool.saturating_sub(fee), fee)
        } else {
            (received_by_pool, 0)
        };
        if swappable == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "fees consume the whole input at this size".to_owned(),
            });
        }

        let (sqrt_next, curve_out) = self.step(input_is_a, swappable)?;
        let after_trade_fee = if fee_on_input {
            curve_out
        } else {
            curve_out.saturating_sub(self.fee_on(curve_out, input_is_a))
        };

        let expected_out =
            super::token2022::net_of_fee(self.transfer_fee(!input_is_a), after_trade_fee);
        if expected_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the pool returns nothing at this size".to_owned(),
            });
        }
        if expected_out >= self.output_vault_balance(input_is_a) {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the output exceeds what the pool's vault holds".to_owned(),
            });
        }

        // Impact is the move in the price itself, i.e. in the SQUARED sqrt price.
        let before = (self.state.sqrt_price as f64) / (2.0_f64).powi(64);
        let after = (sqrt_next as f64) / (2.0_f64).powi(64);
        let price_impact_pct = if before > 0.0 {
            ((before * before - after * after).abs() / (before * before) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        // Report the pool's fee in INPUT units either way, which is the contract
        // of `VenueQuote::lp_fee`. An output-side fee is converted at the
        // realised rate of this very fill rather than at a spot price.
        let lp_fee = if fee_on_input {
            input_fee
        } else {
            let charged = curve_out.saturating_sub(after_trade_fee);
            if curve_out == 0 {
                0
            } else {
                (((charged as u128) * (swappable as u128)) / (curve_out as u128)) as u64
            }
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
        let input_is_a =
            self.is_side_a(&accounts.input_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.state.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        let output_is_a = self.is_side_a(&accounts.output_mint).ok_or_else(|| {
            DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            }
        })?;
        if input_is_a == output_is_a {
            return Err(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            });
        }

        let program = damm_program_id();
        let mut data = Vec::with_capacity(24);
        data.extend_from_slice(&SWAP);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&min_out.to_le_bytes());

        // The vaults and mints go in POOL order, never swap order: the programme
        // reads the direction from which of them the user's input account pairs
        // with, and reordering them silently inverts the trade.
        Ok(Instruction {
            program_id: program,
            accounts: vec![
                AccountMeta::new_readonly(pool_authority(), false),
                AccountMeta::new(self.state.pool, false),
                AccountMeta::new(accounts.input_token_account, false),
                AccountMeta::new(accounts.output_token_account, false),
                AccountMeta::new(self.state.vault_a, false),
                AccountMeta::new(self.state.vault_b, false),
                AccountMeta::new_readonly(self.state.mint_a, false),
                AccountMeta::new_readonly(self.state.mint_b, false),
                AccountMeta::new_readonly(accounts.owner, true),
                AccountMeta::new_readonly(self.token_program_a, false),
                AccountMeta::new_readonly(self.token_program_b, false),
                // `referral_token_account` is optional. Anchor's convention for
                // an absent optional account is the programme's own id, and that
                // is exactly what live mainnet swaps pass.
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

fn damm_program_id() -> Pubkey {
    Pubkey::from_str(METEORA_DAMM_PROGRAM_ID).expect("DAMM v2 program id constant is valid")
}

fn pool_authority() -> Pubkey {
    Pubkey::from_str(POOL_AUTHORITY).expect("DAMM v2 pool authority constant is valid")
}

fn event_authority() -> Pubkey {
    Pubkey::find_program_address(&[EVENT_AUTHORITY_SEED], &damm_program_id()).0
}

#[cfg(test)]
mod tests {
    use super::*;

    const Q64: u128 = 1u128 << 64;

    fn state() -> DammPoolState {
        DammPoolState {
            pool: Pubkey::new_unique(),
            base_fee: BaseFee::Cliff { cliff: 2_500_000 },
            dynamic_fee: DynamicFee::default(),
            collect_fee_mode: COLLECT_FEE_BOTH_TOKEN,
            compounding_fee_bps: 0,
            mint_a: Pubkey::new_unique(),
            mint_b: Pubkey::new_unique(),
            vault_a: Pubkey::new_unique(),
            vault_b: Pubkey::new_unique(),
            // A price of 1.0 with 1e12 raw units of depth either side.
            liquidity: 1_000_000_000_000u128 * Q64,
            protocol_fee_a: 0,
            protocol_fee_b: 0,
            sqrt_min_price: 4_295_048_016,
            sqrt_max_price: 79_226_673_521_066_979_257_578_248_091,
            sqrt_price: Q64,
            activation_point: 0,
            activation_type: 1,
            pool_status: 0,
        }
    }

    fn market(state: DammPoolState) -> DammMarket {
        DammMarket::new(
            state,
            crate::chains::solana::spl_token::id(),
            crate::chains::solana::spl_token::id(),
            9,
            9,
            10_000_000_000_000,
            10_000_000_000_000,
            None,
            None,
            1_000_000,
        )
    }

    #[test]
    fn a_swap_at_parity_returns_about_what_went_in_less_the_pool_fee() {
        let state = state();
        let market = market(state);
        let quote = market.quote(&state.mint_a, 1_000_000_000).expect("quote");
        // 0.25% fee on a pool a thousand times deeper than the trade.
        assert!(quote.expected_out > 996_000_000, "{quote:?}");
        assert!(quote.expected_out < 1_000_000_000, "{quote:?}");
    }

    #[test]
    fn both_directions_price_symmetrically_at_parity() {
        let state = state();
        let market = market(state);
        let forward = market.quote(&state.mint_a, 1_000_000_000).expect("a->b");
        let reverse = market.quote(&state.mint_b, 1_000_000_000).expect("b->a");
        let difference = forward.expected_out.abs_diff(reverse.expected_out);
        assert!(
            difference < 1_000_000,
            "a symmetric pool must price both legs alike, got {forward:?} vs {reverse:?}"
        );
    }

    #[test]
    fn the_output_is_concave_in_the_size() {
        let state = state();
        let market = market(state);
        let small = market.quote(&state.mint_a, 100_000_000_000).expect("small");
        let large = market.quote(&state.mint_a, 400_000_000_000).expect("large");
        assert!(
            (large.expected_out as f64) / 4.0 < small.expected_out as f64,
            "four times the size must return less than four times the output"
        );
        assert!(large.price_impact_pct > small.price_impact_pct);
    }

    #[test]
    fn only_b_charges_the_input_when_token_b_is_spent_and_the_output_otherwise() {
        let mut state = state();
        state.collect_fee_mode = COLLECT_FEE_ONLY_B;
        let market = market(state);
        assert!(market.fee_on_input(false), "spending token B pays on input");
        assert!(
            !market.fee_on_input(true),
            "spending token A pays on output"
        );
    }

    #[test]
    fn every_other_collect_mode_charges_the_output() {
        for mode in [COLLECT_FEE_BOTH_TOKEN, COLLECT_FEE_COMPOUNDING] {
            let mut state = state();
            state.collect_fee_mode = mode;
            let market = market(state);
            assert!(!market.fee_on_input(true));
            assert!(!market.fee_on_input(false));
        }
    }

    #[test]
    fn a_linear_time_scheduler_steps_down_and_stops_at_its_last_period() {
        let fee = BaseFee::TimeScheduler {
            cliff: 10_000_000,
            periods: 3,
            period_frequency: 100,
            reduction_factor: 2_000_000,
            linear: true,
        };
        assert_eq!(fee.numerator(0, 0, 0, false), 10_000_000);
        assert_eq!(fee.numerator(0, 250, 0, false), 6_000_000, "two periods");
        assert_eq!(
            fee.numerator(0, 10_000, 0, false),
            4_000_000,
            "it never falls past the last period"
        );
    }

    #[test]
    fn an_exponential_time_scheduler_compounds_its_reduction() {
        let fee = BaseFee::TimeScheduler {
            cliff: 10_000_000,
            periods: 10,
            period_frequency: 100,
            reduction_factor: 1_000,
            linear: false,
        };
        // 10% off per period: after two periods 0.9^2 = 0.81. The programme
        // compounds the ratio in Q64 fixed point and so does this, which lands a
        // raw unit below the decimal answer — the direction that under-charges
        // nothing and over-states no output.
        assert_eq!(fee.numerator(0, 200, 0, false), 8_099_999);
    }

    #[test]
    fn a_zero_frequency_scheduler_stays_at_its_cliff_rather_than_dividing_by_zero() {
        let fee = BaseFee::TimeScheduler {
            cliff: 7_000_000,
            periods: 5,
            period_frequency: 0,
            reduction_factor: 1_000_000,
            linear: true,
        };
        assert_eq!(fee.numerator(0, u64::MAX, 0, false), 7_000_000);
    }

    #[test]
    fn a_rate_limiter_only_bites_on_the_leg_that_spends_token_b() {
        let fee = BaseFee::RateLimiter {
            cliff: 1_000_000,
            fee_increment_bps: 100,
            max_limiter_duration: 1_000,
            max_fee_bps: 5_000,
            reference_amount: 1_000_000,
        };
        let big = 10_000_000;
        assert_eq!(
            fee.numerator(big, 10, 0, false),
            1_000_000,
            "the a->b leg always pays the cliff"
        );
        assert!(
            fee.numerator(big, 10, 0, true) > 1_000_000,
            "the b->a leg pays more the larger it is"
        );
    }

    #[test]
    fn a_rate_limiter_stops_applying_once_its_window_closes() {
        let fee = BaseFee::RateLimiter {
            cliff: 1_000_000,
            fee_increment_bps: 100,
            max_limiter_duration: 1_000,
            max_fee_bps: 5_000,
            reference_amount: 1_000_000,
        };
        assert_eq!(fee.numerator(10_000_000, 5_000, 0, true), 1_000_000);
    }

    #[test]
    fn a_rate_limited_fee_never_exceeds_its_own_ceiling() {
        let numerator = rate_limited_numerator(u64::MAX / 2, 1_000_000, 100, 5_000, 1_000_000);
        assert!(numerator <= 500_000_000, "got {numerator}");
    }

    #[test]
    fn a_trade_inside_the_reference_amount_pays_only_the_cliff() {
        assert_eq!(
            rate_limited_numerator(500_000, 1_000_000, 100, 5_000, 1_000_000),
            1_000_000
        );
    }

    #[test]
    fn a_dynamic_fee_that_is_not_initialised_charges_nothing() {
        let fee = DynamicFee {
            initialized: false,
            variable_fee_control: 100_000,
            bin_step: 10,
            volatility_accumulator: 10_000,
        };
        assert_eq!(fee.numerator(), 0);
    }

    #[test]
    fn a_dynamic_fee_grows_with_the_square_of_the_volatility() {
        let low = DynamicFee {
            initialized: true,
            variable_fee_control: 100_000,
            bin_step: 10,
            volatility_accumulator: 10_000,
        };
        let high = DynamicFee {
            volatility_accumulator: 20_000,
            ..low
        };
        assert!(high.numerator() >= low.numerator() * 3, "quadratic growth");
    }

    #[test]
    fn a_size_that_would_leave_the_price_band_is_refused_rather_than_quoted() {
        let mut state = state();
        // A band only just above the current price.
        state.sqrt_max_price = state.sqrt_price + state.sqrt_price / 1_000;
        let market = market(state);
        assert!(matches!(
            market.quote(&state.mint_b, 500_000_000_000),
            Err(DirectSwapError::InsufficientLiquidity { .. })
        ));
    }

    #[test]
    fn a_pool_with_no_liquidity_fails_rather_than_returning_zero() {
        let mut state = state();
        state.liquidity = 0;
        let market = market(state);
        assert!(matches!(
            market.quote(&state.mint_a, 1_000),
            Err(DirectSwapError::InsufficientLiquidity { .. })
        ));
    }

    #[test]
    fn a_mint_the_pool_does_not_hold_is_refused() {
        let state = state();
        let market = market(state);
        assert!(matches!(
            market.quote(&Pubkey::new_unique(), 1_000),
            Err(DirectSwapError::PairNotInPool { .. })
        ));
    }

    #[test]
    fn the_swap_instruction_passes_the_vaults_in_pool_order_for_both_directions() {
        let state = state();
        let market = market(state);
        let owner = Pubkey::new_unique();
        let account_a = Pubkey::new_unique();
        let account_b = Pubkey::new_unique();

        for (input, output, input_account, output_account) in [
            (state.mint_a, state.mint_b, account_a, account_b),
            (state.mint_b, state.mint_a, account_b, account_a),
        ] {
            let ix = market
                .swap_instruction(
                    &SwapAccounts {
                        owner,
                        input_mint: input,
                        output_mint: output,
                        input_token_account: input_account,
                        output_token_account: output_account,
                    },
                    1_000,
                    900,
                )
                .expect("instruction");
            assert_eq!(ix.accounts[2].pubkey, input_account);
            assert_eq!(ix.accounts[3].pubkey, output_account);
            assert_eq!(ix.accounts[4].pubkey, state.vault_a, "vault A stays first");
            assert_eq!(ix.accounts[5].pubkey, state.vault_b, "vault B stays second");
            assert_eq!(ix.accounts[6].pubkey, state.mint_a);
            assert_eq!(ix.accounts[7].pubkey, state.mint_b);
            assert!(ix.accounts[8].is_signer, "the payer signs");
            assert_eq!(&ix.data[..8], &SWAP);
            assert_eq!(&ix.data[8..16], &1_000u64.to_le_bytes());
            assert_eq!(&ix.data[16..24], &900u64.to_le_bytes());
        }
    }

    #[test]
    fn an_absent_referral_account_is_the_programme_itself() {
        let state = state();
        let market = market(state);
        let ix = market
            .swap_instruction(
                &SwapAccounts {
                    owner: Pubkey::new_unique(),
                    input_mint: state.mint_a,
                    output_mint: state.mint_b,
                    input_token_account: Pubkey::new_unique(),
                    output_token_account: Pubkey::new_unique(),
                },
                1_000,
                900,
            )
            .expect("instruction");
        assert_eq!(ix.accounts.len(), 14);
        assert_eq!(ix.accounts[11].pubkey, damm_program_id());
        assert_eq!(ix.accounts[13].pubkey, damm_program_id());
    }

    #[test]
    fn swapping_a_mint_for_itself_is_refused() {
        let state = state();
        let market = market(state);
        assert!(matches!(
            market.swap_instruction(
                &SwapAccounts {
                    owner: Pubkey::new_unique(),
                    input_mint: state.mint_a,
                    output_mint: state.mint_a,
                    input_token_account: Pubkey::new_unique(),
                    output_token_account: Pubkey::new_unique(),
                },
                1_000,
                900,
            ),
            Err(DirectSwapError::PairNotInPool { .. })
        ));
    }

    #[test]
    fn the_delta_helpers_round_down_and_never_panic_on_an_empty_range() {
        assert_eq!(delta_b(Q64, Q64, Q64), Some(0));
        assert_eq!(delta_a(Q64, Q64, Q64), Some(0));
        assert_eq!(
            delta_b(Q64 * 2, Q64, Q64),
            None,
            "an inverted range is None"
        );
    }
}
