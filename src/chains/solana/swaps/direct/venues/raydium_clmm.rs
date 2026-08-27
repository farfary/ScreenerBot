//! Raydium CLMM (`CAMMCzo5…`) — concentrated liquidity.
//!
//! # What was wrong before
//!
//! The previous implementation used discriminator `964318cd…`, which is the
//! hash of no instruction name in the programme, and passed none of the tick
//! arrays a concentrated-liquidity swap needs. Neither error is recoverable at
//! runtime: no CLMM swap it produced could ever have executed. The real
//! `swap_v2` discriminator is `2b04ed0b1ac91e62`, confirmed both by hashing
//! `global:swap_v2` and by decoding live mainnet swaps of the SOL/USDC pool.
//!
//! # Layout, verified against mainnet
//!
//! `PoolState` is 1544 bytes:
//!
//! ```text
//!   9 amm_config      73 token_mint_0  105 token_mint_1
//! 137 token_vault_0  169 token_vault_1 201 observation_key
//! 233 mint_decimals_0 234 mint_decimals_1 235 tick_spacing u16
//! 237 liquidity u128 253 sqrt_price_x64 u128  269 tick_current i32
//! 389 status u8      904 tick_array_bitmap [u64; 16]
//! ```
//!
//! `AmmConfig` is 117 bytes: `43 protocol_fee_rate u32 · 47 trade_fee_rate u32 ·
//! 51 tick_spacing u16 · 53 fund_fee_rate u32`, over a denominator of 1_000_000.
//!
//! # What the quote can and cannot promise
//!
//! The quote walks the real tick-by-tick swap loop the programme itself runs:
//! constant liquidity between initialised ticks, a fee charged per step, and
//! `liquidity_net` applied at every tick the swap crosses (added moving up,
//! subtracted moving down). That is exact for as long as the loaded tick
//! arrays cover the size being quoted — which is `load()`'s job, fetching
//! `TICK_ARRAYS_PER_SWAP` arrays in both directions from the pool's own
//! bitmap.
//!
//! A size that would cross beyond the last LOADED tick is refused outright
//! rather than assumed constant past that point, and every swap is simulated
//! before it is sent — an unsatisfiable `min_out` is rejected by a node for
//! free rather than by the pool for a priority fee.

use super::clmm_ticks::{
    bitmap_extension_address, decode_tick_array, get_sqrt_price_at_tick, tick_array_address,
    InitializedTick, TickArrayBitmap, TICK_ARRAYS_PER_SWAP,
};
use super::layout::{i32_at, pubkey_at, token_account_amount, u128_at, u16_at, u32_at, u8_at};
use super::math::{ceil_div, mul_div_ceil, mul_div_floor};
use super::token2022::{transfer_fee_schedule, TransferFeeSchedule};
use crate::chains::solana::constants::{MEMO_PROGRAM_ID, RAYDIUM_CLMM_PROGRAM_ID};
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

/// `sha256("global:swap_v2")[..8]`, confirmed against live mainnet swaps.
const SWAP_V2: [u8; 8] = [0x2b, 0x04, 0xed, 0x0b, 0x1a, 0xc9, 0x1e, 0x62];

/// Denominator every CLMM fee rate is expressed over.
const FEE_RATE_DENOMINATOR: u64 = 1_000_000;

/// Bit index of the swap permission inside `PoolState::status`, a DISABLE flag.
const STATUS_BIT_SWAP_DISABLED: u8 = 2;

/// Compute units a CLMM swap needs. Higher than a constant-product venue because
/// the programme may load and cross several tick arrays.
const COMPUTE_UNITS: u32 = 200_000;

/// The venue adapter.
pub struct RaydiumClmmVenue;

#[async_trait]
impl PoolVenue for RaydiumClmmVenue {
    fn program(&self) -> ProgramKind {
        ProgramKind::RaydiumClmm
    }

    fn program_id(&self) -> Pubkey {
        clmm_program_id()
    }

    async fn load(
        &self,
        pool: &Pubkey,
        pool_account: &Account,
    ) -> DirectSwapResult<Box<dyn PoolMarket>> {
        let state = ClmmPoolState::decode(*pool, &pool_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: format!(
                    "CLMM pool state did not match the expected layout ({} bytes)",
                    pool_account.data.len()
                ),
            }
        })?;

        if !state.swap_enabled() {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: format!("status byte {} has swapping disabled", state.status),
            });
        }
        if state.liquidity == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: *pool,
                amount_in: 0,
                detail: "the pool has no liquidity in range at the current tick".to_owned(),
            });
        }

        let bitmap_extension = bitmap_extension_address(&clmm_program_id(), pool);

        // The pool's own bitmap is already in hand from `pool_account.data` --
        // no RPC needed to know which nearby arrays exist. It cannot yet see
        // past the extension, but that only matters for arrays further out
        // than the `TICK_ARRAYS_PER_SWAP` this swap could ever touch anyway.
        let initial_bitmap =
            TickArrayBitmap::from_pool_state(&pool_account.data).ok_or_else(|| {
                DirectSwapError::PoolUndecodable {
                    pool: *pool,
                    detail: "CLMM pool state carries no tick-array bitmap".to_owned(),
                }
            })?;

        let mut tick_array_addresses: Vec<Pubkey> = Vec::new();
        for zero_for_one in [true, false] {
            for start in
                initial_bitmap.arrays_for_swap(state.tick_current, state.tick_spacing, zero_for_one)
            {
                let address = tick_array_address(&clmm_program_id(), pool, start);
                if !tick_array_addresses.contains(&address) {
                    tick_array_addresses.push(address);
                }
            }
        }

        let fixed_addresses = [
            state.amm_config,
            state.vault_0,
            state.vault_1,
            state.mint_0,
            state.mint_1,
            bitmap_extension,
        ];
        let mut addresses = fixed_addresses.to_vec();
        addresses.extend(tick_array_addresses.iter().copied());

        let accounts = get_rpc_client()
            .get_multiple_accounts(&addresses)
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: *pool,
                detail: format!("CLMM pool accounts could not be read: {e}"),
            })?;

        let required = |index: usize| -> DirectSwapResult<&Account> {
            accounts.get(index).and_then(Option::as_ref).ok_or(
                DirectSwapError::AccountUnavailable {
                    address: addresses[index],
                    detail: "account does not exist".to_owned(),
                },
            )
        };

        let config = ClmmFeeConfig::decode(&required(0)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "CLMM AmmConfig did not match the expected layout".to_owned(),
            }
        })?;
        let vault_0_balance = token_account_amount(&required(1)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_0 vault is not a token account".to_owned(),
            }
        })?;
        let vault_1_balance = token_account_amount(&required(2)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_1 vault is not a token account".to_owned(),
            }
        })?;
        let mint_0_account = required(3)?;
        let mint_1_account = required(4)?;
        let token_program_0 = mint_0_account.owner;
        let token_program_1 = mint_1_account.owner;
        let transfer_fee_0 = transfer_fee_schedule(mint_0_account);
        let transfer_fee_1 = transfer_fee_schedule(mint_1_account);

        // The extension is optional: a pool whose price has never left the
        // range its own bitmap covers does not have one.
        let mut bitmap = initial_bitmap;
        if let Some(Some(extension)) = accounts.get(5) {
            bitmap = bitmap.with_extension(pool, &extension.data);
        }

        // Every tick array actually returned by the batch, decoded into its
        // initialised ticks. An array that does not exist (an uninitialised
        // one, per the pool's own bitmap) is simply not there to decode -- it
        // is not a load error, because the swap may never reach it.
        let mut ticks: Vec<InitializedTick> = Vec::new();
        for (offset, address) in tick_array_addresses.iter().enumerate() {
            let Some(Some(account)) = accounts.get(fixed_addresses.len() + offset) else {
                continue;
            };
            if let Some(decoded) = decode_tick_array(pool, &account.data) {
                ticks.extend(decoded);
            } else {
                return Err(DirectSwapError::PoolUndecodable {
                    pool: *pool,
                    detail: format!("tick array {address} did not match the expected layout"),
                });
            }
        }
        ticks.sort_by_key(|tick| tick.tick);
        ticks.dedup_by_key(|tick| tick.tick);

        Ok(Box::new(ClmmMarket {
            state,
            config,
            bitmap,
            bitmap_extension,
            token_program_0,
            token_program_1,
            vault_0_balance,
            vault_1_balance,
            transfer_fee_0,
            transfer_fee_1,
            ticks,
        }))
    }
}

/// The parts of the CLMM `PoolState` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClmmPoolState {
    pub pool: Pubkey,
    pub amm_config: Pubkey,
    pub mint_0: Pubkey,
    pub mint_1: Pubkey,
    pub vault_0: Pubkey,
    pub vault_1: Pubkey,
    pub observation: Pubkey,
    pub decimals_0: u8,
    pub decimals_1: u8,
    pub tick_spacing: u16,
    pub liquidity: u128,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
    pub status: u8,
}

impl ClmmPoolState {
    /// Decode a CLMM pool account. Pure: no RPC, no cache, no clock.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        Some(Self {
            pool,
            amm_config: pubkey_at(data, 9)?,
            mint_0: pubkey_at(data, 73)?,
            mint_1: pubkey_at(data, 105)?,
            vault_0: pubkey_at(data, 137)?,
            vault_1: pubkey_at(data, 169)?,
            observation: pubkey_at(data, 201)?,
            decimals_0: u8_at(data, 233)?,
            decimals_1: u8_at(data, 234)?,
            tick_spacing: u16_at(data, 235)?,
            liquidity: u128_at(data, 237)?,
            sqrt_price_x64: u128_at(data, 253)?,
            tick_current: i32_at(data, 269)?,
            status: u8_at(data, 389)?,
        })
    }

    /// Whether the swap permission bit is clear.
    pub fn swap_enabled(&self) -> bool {
        self.status & (1 << STATUS_BIT_SWAP_DISABLED) == 0
    }
}

/// The fee rates from the CLMM `AmmConfig`. Stored as `u32`, unlike CP-Swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClmmFeeConfig {
    pub protocol_fee_rate: u32,
    pub trade_fee_rate: u32,
    pub fund_fee_rate: u32,
}

impl ClmmFeeConfig {
    /// Decode a CLMM `AmmConfig` account. Pure.
    pub fn decode(data: &[u8]) -> Option<Self> {
        Some(Self {
            protocol_fee_rate: u32_at(data, 43)?,
            trade_fee_rate: u32_at(data, 47)?,
            fund_fee_rate: u32_at(data, 53)?,
        })
    }
}

/// A decoded, quotable CLMM pool.
#[derive(Debug, Clone)]
pub struct ClmmMarket {
    state: ClmmPoolState,
    config: ClmmFeeConfig,
    bitmap: TickArrayBitmap,
    bitmap_extension: Pubkey,
    /// Owner of `mint_0`. The CLMM pool state does not record a token program
    /// per side, so it comes from the mint account itself — and it must, because
    /// a Token-2022 mint derives a different ATA address than a legacy one.
    token_program_0: Pubkey,
    /// Owner of `mint_1`.
    token_program_1: Pubkey,
    vault_0_balance: u64,
    vault_1_balance: u64,
    transfer_fee_0: Option<TransferFeeSchedule>,
    transfer_fee_1: Option<TransferFeeSchedule>,
    /// Initialised ticks from every tick array `load()` fetched, sorted
    /// ascending. Only ticks the swap could plausibly reach were fetched --
    /// this is a bound on how far the walk can speak for, not a full pool
    /// state.
    ticks: Vec<InitializedTick>,
}

impl ClmmMarket {
    /// Build a market directly from decoded parts, for the offline test tier.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: ClmmPoolState,
        config: ClmmFeeConfig,
        bitmap: TickArrayBitmap,
        bitmap_extension: Pubkey,
        token_program_0: Pubkey,
        token_program_1: Pubkey,
        vault_0_balance: u64,
        vault_1_balance: u64,
        transfer_fee_0: Option<TransferFeeSchedule>,
        transfer_fee_1: Option<TransferFeeSchedule>,
        mut ticks: Vec<InitializedTick>,
    ) -> Self {
        ticks.sort_by_key(|tick| tick.tick);
        ticks.dedup_by_key(|tick| tick.tick);
        Self {
            state,
            config,
            bitmap,
            bitmap_extension,
            token_program_0,
            token_program_1,
            vault_0_balance,
            vault_1_balance,
            transfer_fee_0,
            transfer_fee_1,
            ticks,
        }
    }

    /// Whether `mint` is the pool's token_0 side.
    fn is_side_0(&self, mint: &Pubkey) -> Option<bool> {
        if *mint == self.state.mint_0 {
            Some(true)
        } else if *mint == self.state.mint_1 {
            Some(false)
        } else {
            None
        }
    }

    fn transfer_fee(&self, side_0: bool) -> Option<&TransferFeeSchedule> {
        if side_0 {
            self.transfer_fee_0.as_ref()
        } else {
            self.transfer_fee_1.as_ref()
        }
    }

    /// The vault the output comes out of, so a quote can never promise more than
    /// the pool physically holds.
    fn output_vault_balance(&self, input_is_0: bool) -> u64 {
        if input_is_0 {
            self.vault_1_balance
        } else {
            self.vault_0_balance
        }
    }

    /// The initialised ticks the walk may cross, in the order it will meet
    /// them: descending from (and including) the current tick when selling
    /// token_0 (price falling), ascending above it when selling token_1
    /// (price rising). `self.ticks` is sorted ascending, so a plain filter
    /// plus, for the downward case, a reverse, is the whole of it.
    fn ticks_ahead(&self, zero_for_one: bool) -> Vec<InitializedTick> {
        let current = self.state.tick_current;
        if zero_for_one {
            self.ticks
                .iter()
                .rev()
                .filter(|t| t.tick <= current)
                .copied()
                .collect()
        } else {
            self.ticks
                .iter()
                .filter(|t| t.tick > current)
                .copied()
                .collect()
        }
    }

    /// The output produced by moving the price at constant `liquidity` from
    /// `sqrt_from` to `sqrt_to`, rounded DOWN — the pool never owes more than
    /// it computed.
    ///
    /// Selling token_0 moves the price DOWN and pays out token_1; selling
    /// token_1 moves it UP and pays out token_0.
    fn output_for_move(
        zero_for_one: bool,
        liquidity: u128,
        sqrt_from: u128,
        sqrt_to: u128,
    ) -> DirectSwapResult<u64> {
        let overflow = || DirectSwapError::QuoteMath {
            detail: "the concentrated-liquidity output overflowed 256 bits".to_owned(),
        };
        let out = if zero_for_one {
            mul_div_floor(liquidity, sqrt_from.saturating_sub(sqrt_to), 1u128 << 64)
                .ok_or_else(overflow)?
        } else {
            let numerator =
                liquidity
                    .checked_shl(64)
                    .ok_or_else(|| DirectSwapError::QuoteMath {
                        detail: "liquidity is too large to scale to Q64.64".to_owned(),
                    })?;
            let spread = sqrt_to.saturating_sub(sqrt_from);
            let intermediate = mul_div_floor(numerator, spread, sqrt_to).ok_or_else(overflow)?;
            intermediate / sqrt_from.max(1)
        };
        Ok(out.min(u64::MAX as u128) as u64)
    }

    /// The sqrt price reached by spending `amount_in` (already net of the fee)
    /// at constant `liquidity` from `sqrt_price` — the partial-step case, when
    /// the input runs out before the next tick.
    fn next_sqrt_price_from_input(
        zero_for_one: bool,
        liquidity: u128,
        sqrt_price: u128,
        amount_in: u128,
    ) -> DirectSwapResult<u128> {
        let overflow = || DirectSwapError::QuoteMath {
            detail: "the concentrated-liquidity step overflowed 256 bits".to_owned(),
        };
        if zero_for_one {
            let numerator =
                liquidity
                    .checked_shl(64)
                    .ok_or_else(|| DirectSwapError::QuoteMath {
                        detail: "liquidity is too large to scale to Q64.64".to_owned(),
                    })?;
            let product = amount_in.checked_mul(sqrt_price).ok_or_else(overflow)?;
            let denominator = numerator.checked_add(product).ok_or_else(overflow)?;
            mul_div_ceil(numerator, sqrt_price, denominator).ok_or_else(overflow)
        } else {
            let delta = mul_div_floor(amount_in, 1u128 << 64, liquidity).ok_or_else(overflow)?;
            sqrt_price.checked_add(delta).ok_or_else(overflow)
        }
    }

    /// The amount of the input side needed to move the price EXACTLY from
    /// `sqrt_from` to `sqrt_to` at constant `liquidity`, rounded UP.
    ///
    /// Understating this would cross a tick boundary — and apply its
    /// `liquidity_net` — without having paid for reaching it, which is a step
    /// the programme never takes.
    fn input_for_move(
        zero_for_one: bool,
        liquidity: u128,
        sqrt_from: u128,
        sqrt_to: u128,
    ) -> DirectSwapResult<u128> {
        let overflow = || DirectSwapError::QuoteMath {
            detail: "the concentrated-liquidity input overflowed 256 bits".to_owned(),
        };
        if zero_for_one {
            // token_0 in, price falling: sqrt_from (high) > sqrt_to (low).
            // amount0 = ceil(ceil(L<<64 * (hi-lo) / hi) / lo)
            if sqrt_from <= sqrt_to {
                return Ok(0);
            }
            let numerator =
                liquidity
                    .checked_shl(64)
                    .ok_or_else(|| DirectSwapError::QuoteMath {
                        detail: "liquidity is too large to scale to Q64.64".to_owned(),
                    })?;
            let spread = sqrt_from - sqrt_to;
            let step1 = mul_div_ceil(numerator, spread, sqrt_from).ok_or_else(overflow)?;
            Ok(ceil_div(step1, sqrt_to.max(1)))
        } else {
            // token_1 in, price rising: sqrt_to (high) > sqrt_from (low).
            // amount1 = ceil(L * (hi-lo) / 2^64)
            if sqrt_to <= sqrt_from {
                return Ok(0);
            }
            let spread = sqrt_to - sqrt_from;
            mul_div_ceil(liquidity, spread, 1u128 << 64).ok_or_else(overflow)
        }
    }

    /// Apply a crossed tick's `liquidity_net` to `liquidity`: ADDED when the
    /// price is moving up (`!zero_for_one`), SUBTRACTED when it is moving down
    /// -- `liquidity_net` is always defined for an upward crossing, so the
    /// downward direction negates it.
    fn cross_tick(
        &self,
        liquidity: u128,
        tick: &InitializedTick,
        zero_for_one: bool,
    ) -> DirectSwapResult<u128> {
        let delta = if zero_for_one {
            -tick.liquidity_net
        } else {
            tick.liquidity_net
        };
        liquidity
            .checked_add_signed(delta)
            .ok_or(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in: 0,
                detail: format!(
                    "crossing tick {} would take pool liquidity negative -- the loaded tick data \
                     is inconsistent with the pool's own reported liquidity",
                    tick.tick
                ),
            })
    }

    /// Walk the swap tick by tick from the pool's current price, exactly as
    /// the programme's own loop does: constant liquidity between initialised
    /// ticks, a fee charged per step, `liquidity_net` applied at every
    /// crossing. `amount_in` is already net of any Token-2022 transfer fee on
    /// the input -- it is what the pool itself receives to swap and to fee.
    ///
    /// Returns `(total_output, total_lp_fee, ending_sqrt_price)`. Refuses,
    /// rather than approximates, once the walk would cross beyond the last
    /// tick `load()` fetched.
    fn walk(&self, zero_for_one: bool, amount_in: u64) -> DirectSwapResult<(u64, u64, u128)> {
        let rate = self.config.trade_fee_rate as u128;
        let denom = FEE_RATE_DENOMINATOR as u128;
        if rate >= denom {
            return Err(DirectSwapError::QuoteMath {
                detail: "CLMM trade fee rate is not below its own denominator".to_owned(),
            });
        }

        let mut liquidity = self.state.liquidity;
        let mut sqrt_price = self.state.sqrt_price_x64;
        let mut remaining = amount_in as u128;
        let mut total_out: u128 = 0;
        let mut total_fee: u128 = 0;

        let mut candidates = self.ticks_ahead(zero_for_one).into_iter();

        while remaining > 0 {
            if liquidity == 0 || sqrt_price == 0 {
                return Err(DirectSwapError::InsufficientLiquidity {
                    pool: self.state.pool,
                    amount_in,
                    detail: "liquidity was exhausted mid-walk".to_owned(),
                });
            }

            let Some(next_tick) = candidates.next() else {
                return Err(DirectSwapError::InsufficientLiquidity {
                    pool: self.state.pool,
                    amount_in,
                    detail: "the size travels further than the loaded tick arrays cover".to_owned(),
                });
            };
            let target_sqrt = get_sqrt_price_at_tick(next_tick.tick).ok_or_else(|| {
                DirectSwapError::QuoteMath {
                    detail: format!(
                        "tick {} lies outside the programme's own range",
                        next_tick.tick
                    ),
                }
            })?;

            let amount_to_target =
                Self::input_for_move(zero_for_one, liquidity, sqrt_price, target_sqrt)?;
            let gross_needed = if amount_to_target == 0 {
                0
            } else {
                mul_div_ceil(amount_to_target, denom, denom - rate).ok_or(
                    DirectSwapError::QuoteMath {
                        detail: "the per-step fee grossing-up overflowed 256 bits".to_owned(),
                    },
                )?
            };

            if gross_needed <= remaining {
                // A full step to the boundary: the fee is the leftover between
                // what reaching the boundary cost gross and what it cost net.
                let fee = (gross_needed - amount_to_target).min(u64::MAX as u128);
                let out = Self::output_for_move(zero_for_one, liquidity, sqrt_price, target_sqrt)?;
                total_out += out as u128;
                total_fee += fee;
                remaining -= gross_needed;
                sqrt_price = target_sqrt;
                liquidity = self.cross_tick(liquidity, &next_tick, zero_for_one)?;
            } else {
                // The last, partial step: all remaining input is spent here and
                // the walk does not reach `next_tick`, so nothing is crossed.
                let swappable = mul_div_floor(remaining, denom - rate, denom).ok_or(
                    DirectSwapError::QuoteMath {
                        detail: "the per-step net-of-fee amount overflowed 256 bits".to_owned(),
                    },
                )?;
                let fee = remaining - swappable;
                if swappable == 0 {
                    total_fee += fee;
                    break;
                }
                let sqrt_next = Self::next_sqrt_price_from_input(
                    zero_for_one,
                    liquidity,
                    sqrt_price,
                    swappable,
                )?;
                let out = Self::output_for_move(zero_for_one, liquidity, sqrt_price, sqrt_next)?;
                total_out += out as u128;
                total_fee += fee;
                remaining = 0;
                sqrt_price = sqrt_next;
            }
        }

        Ok((
            total_out.min(u64::MAX as u128) as u64,
            total_fee.min(u64::MAX as u128) as u64,
            sqrt_price,
        ))
    }

    /// The tick arrays this swap direction may cross, as instruction accounts.
    fn tick_array_accounts(&self, zero_for_one: bool) -> Vec<Pubkey> {
        self.bitmap
            .arrays_for_swap(
                self.state.tick_current,
                self.state.tick_spacing,
                zero_for_one,
            )
            .into_iter()
            .map(|start| tick_array_address(&clmm_program_id(), &self.state.pool, start))
            .collect()
    }
}

impl PoolMarket for ClmmMarket {
    fn program(&self) -> ProgramKind {
        ProgramKind::RaydiumClmm
    }

    fn pool(&self) -> Pubkey {
        self.state.pool
    }

    fn mints(&self) -> (Pubkey, Pubkey) {
        (self.state.mint_0, self.state.mint_1)
    }

    fn token_program(&self, mint: &Pubkey) -> Option<Pubkey> {
        self.is_side_0(mint).map(|side_0| {
            if side_0 {
                self.token_program_0
            } else {
                self.token_program_1
            }
        })
    }

    fn decimals(&self, mint: &Pubkey) -> Option<u8> {
        self.is_side_0(mint).map(|side_0| {
            if side_0 {
                self.state.decimals_0
            } else {
                self.state.decimals_1
            }
        })
    }

    fn quote(&self, input_mint: &Pubkey, amount_in: u64) -> DirectSwapResult<VenueQuote> {
        let input_is_0 = self
            .is_side_0(input_mint)
            .ok_or(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: *input_mint,
                output_mint: Pubkey::default(),
            })?;

        let received_by_pool =
            super::token2022::net_of_fee(self.transfer_fee(input_is_0), amount_in);
        if received_by_pool == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the input transfer fee consumes the whole amount at this size".to_owned(),
            });
        }

        let (gross_out, lp_fee, sqrt_next) = self.walk(input_is_0, received_by_pool)?;
        if gross_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "fees consume the whole input at this size".to_owned(),
            });
        }

        let expected_out = super::token2022::net_of_fee(self.transfer_fee(!input_is_0), gross_out);
        if expected_out == 0 || expected_out >= self.output_vault_balance(input_is_0) {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the output exceeds what the pool's vault holds".to_owned(),
            });
        }

        // Impact is the move in the SQUARED price, which is the price itself.
        let before = (self.state.sqrt_price_x64 as f64) / (2.0_f64).powi(64);
        let after = (sqrt_next as f64) / (2.0_f64).powi(64);
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
        let input_is_0 =
            self.is_side_0(&accounts.input_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.state.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        let output_is_0 = self.is_side_0(&accounts.output_mint).ok_or_else(|| {
            DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            }
        })?;
        if input_is_0 == output_is_0 {
            return Err(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            });
        }

        let (input_vault, output_vault) = if input_is_0 {
            (self.state.vault_0, self.state.vault_1)
        } else {
            (self.state.vault_1, self.state.vault_0)
        };

        let tick_arrays = self.tick_array_accounts(input_is_0);
        if tick_arrays.is_empty() {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "no initialised tick array lies in the swap direction".to_owned(),
            });
        }

        let mut data = Vec::with_capacity(41);
        data.extend_from_slice(&SWAP_V2);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&min_out.to_le_bytes());
        // sqrt_price_limit_x64 = 0: no price limit, because `min_out` is the
        // protection and a second, redundant limit only adds a way to fail.
        data.extend_from_slice(&0u128.to_le_bytes());
        // is_base_input: the amount above is what goes IN.
        data.push(1);

        let mut metas = vec![
            AccountMeta::new_readonly(accounts.owner, true),
            AccountMeta::new_readonly(self.state.amm_config, false),
            AccountMeta::new(self.state.pool, false),
            AccountMeta::new(accounts.input_token_account, false),
            AccountMeta::new(accounts.output_token_account, false),
            AccountMeta::new(input_vault, false),
            AccountMeta::new(output_vault, false),
            AccountMeta::new(self.state.observation, false),
            AccountMeta::new_readonly(crate::chains::solana::spl_token::id(), false),
            AccountMeta::new_readonly(crate::chains::solana::spl_token_2022::id(), false),
            AccountMeta::new_readonly(memo_program_id(), false),
            AccountMeta::new_readonly(accounts.input_mint, false),
            AccountMeta::new_readonly(accounts.output_mint, false),
        ];
        // Remaining accounts: the bitmap extension first, then the tick arrays
        // in the order the price will reach them. This is the order live mainnet
        // swaps use, and the programme reads them positionally.
        metas.push(AccountMeta::new(self.bitmap_extension, false));
        for array in tick_arrays.iter().take(TICK_ARRAYS_PER_SWAP) {
            metas.push(AccountMeta::new(*array, false));
        }

        Ok(Instruction {
            program_id: clmm_program_id(),
            accounts: metas,
            data,
        })
    }

    fn compute_units(&self) -> u32 {
        COMPUTE_UNITS
    }
}

fn clmm_program_id() -> Pubkey {
    Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID).expect("CLMM program id constant is valid")
}

fn memo_program_id() -> Pubkey {
    Pubkey::from_str(MEMO_PROGRAM_ID).expect("memo program id constant is valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A market with only the fields the tick walk touches. `trade_fee_rate`
    /// defaults to zero so a test's hand-picked amounts are exact -- fee
    /// arithmetic itself is `math.rs`'s job, already covered there.
    fn market(
        liquidity: u128,
        tick_current: i32,
        trade_fee_rate: u32,
        ticks: Vec<InitializedTick>,
    ) -> ClmmMarket {
        let mint_0 = Pubkey::new_unique();
        let mint_1 = Pubkey::new_unique();
        let state = ClmmPoolState {
            pool: Pubkey::new_unique(),
            amm_config: Pubkey::new_unique(),
            mint_0,
            mint_1,
            vault_0: Pubkey::new_unique(),
            vault_1: Pubkey::new_unique(),
            observation: Pubkey::new_unique(),
            decimals_0: 9,
            decimals_1: 6,
            tick_spacing: 1,
            liquidity,
            sqrt_price_x64: get_sqrt_price_at_tick(tick_current).expect("tick in range"),
            tick_current,
            status: 0,
        };
        let config = ClmmFeeConfig {
            protocol_fee_rate: 0,
            trade_fee_rate,
            fund_fee_rate: 0,
        };
        ClmmMarket::new(
            state,
            config,
            TickArrayBitmap::default(),
            Pubkey::new_unique(),
            crate::chains::solana::spl_token::id(),
            crate::chains::solana::spl_token::id(),
            u64::MAX,
            u64::MAX,
            None,
            None,
            ticks,
        )
    }

    #[test]
    fn a_swap_entirely_within_one_liquidity_region_never_crosses_a_tick() {
        // One tick loaded, far enough away that the swap cannot possibly
        // reach it -- this is what "load() fetched more range than the swap
        // needs" looks like in practice.
        let far_tick = InitializedTick {
            tick: -1_000,
            liquidity_net: 0,
        };
        let m = market(1_000_000_000_000, 0, 0, vec![far_tick]);
        let (out, fee, sqrt_end) = m.walk(true, 1_000_000).expect("stays in range");
        assert!(out > 0, "a real swap must produce something");
        assert_eq!(fee, 0, "the fee rate here is zero");
        assert!(
            sqrt_end < m.state.sqrt_price_x64,
            "selling token_0 must move the price down"
        );
    }

    #[test]
    fn crossing_a_tick_where_liquidity_drops_yields_less_than_constant_liquidity_would() {
        // liquidity_net is positive at the crossed tick, so a downward
        // crossing SUBTRACTS it (see `cross_tick`) -- liquidity is lower for
        // the remainder of the walk than it was at the start.
        let crossed = InitializedTick {
            tick: -10,
            liquidity_net: 400_000_000_000,
        };
        let safety_net = InitializedTick {
            tick: -100_000,
            liquidity_net: 0,
        };
        let initial_liquidity = 1_000_000_000_000u128;
        let m = market(initial_liquidity, 0, 0, vec![crossed, safety_net]);

        let amount_to_cross = ClmmMarket::input_for_move(
            true,
            initial_liquidity,
            m.state.sqrt_price_x64,
            get_sqrt_price_at_tick(-10).unwrap(),
        )
        .unwrap();
        let amount_in = (amount_to_cross + 5_000_000_000) as u64;

        let (out, _fee, sqrt_end) = m
            .walk(true, amount_in)
            .expect("covered by the safety net tick");

        // The output a naive single, constant-liquidity step over the same
        // total price move would have promised -- the exact over-statement
        // this whole task exists to close.
        let naive_out =
            ClmmMarket::output_for_move(true, initial_liquidity, m.state.sqrt_price_x64, sqrt_end)
                .unwrap();

        assert!(
            out < naive_out,
            "crossing into thinner liquidity must yield less than a constant-liquidity \
             estimate would have promised: real {out} vs naive {naive_out}"
        );
    }

    #[test]
    fn crossing_a_tick_where_liquidity_rises_yields_more_than_constant_liquidity_would() {
        // A negative liquidity_net at the crossed tick means a downward
        // crossing ADDS liquidity (subtracting a negative).
        let crossed = InitializedTick {
            tick: -10,
            liquidity_net: -400_000_000_000,
        };
        let safety_net = InitializedTick {
            tick: -100_000,
            liquidity_net: 0,
        };
        let initial_liquidity = 1_000_000_000_000u128;
        let m = market(initial_liquidity, 0, 0, vec![crossed, safety_net]);

        let amount_to_cross = ClmmMarket::input_for_move(
            true,
            initial_liquidity,
            m.state.sqrt_price_x64,
            get_sqrt_price_at_tick(-10).unwrap(),
        )
        .unwrap();
        let amount_in = (amount_to_cross + 5_000_000_000) as u64;

        let (out, _fee, sqrt_end) = m
            .walk(true, amount_in)
            .expect("covered by the safety net tick");
        let naive_out =
            ClmmMarket::output_for_move(true, initial_liquidity, m.state.sqrt_price_x64, sqrt_end)
                .unwrap();

        assert!(
            out > naive_out,
            "crossing into thicker liquidity must yield more than a constant-liquidity \
             estimate would have promised: real {out} vs naive {naive_out}"
        );
    }

    #[test]
    fn a_swap_that_exhausts_every_loaded_tick_array_refuses_rather_than_guesses() {
        // Only one nearby tick is loaded at all -- once the walk crosses it
        // there is nothing left to bound the rest of the swap.
        let only_tick = InitializedTick {
            tick: -10,
            liquidity_net: 100,
        };
        let m = market(1_000_000_000_000, 0, 0, vec![only_tick]);
        let amount_to_cross = ClmmMarket::input_for_move(
            true,
            1_000_000_000_000,
            m.state.sqrt_price_x64,
            get_sqrt_price_at_tick(-10).unwrap(),
        )
        .unwrap();
        // Comfortably past the one tick this market knows about.
        let amount_in = (amount_to_cross * 10) as u64;

        let err = m
            .walk(true, amount_in)
            .expect_err("a size beyond every loaded tick must not be quoted");
        match err {
            DirectSwapError::InsufficientLiquidity { detail, .. } => {
                assert!(
                    detail.contains("loaded tick arrays"),
                    "the refusal must name the real limit, got: {detail}"
                );
            }
            other => panic!("expected InsufficientLiquidity, got {other:?}"),
        }
    }
}
