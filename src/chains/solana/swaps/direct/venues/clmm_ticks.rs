//! Tick arrays for a Raydium CLMM swap.
//!
//! A concentrated-liquidity swap walks a price range, and the programme needs
//! the accounts holding the ticks it might cross passed in as remaining
//! accounts. Working out WHICH accounts those are is the whole content of this
//! module, and it is the piece the previous CLMM implementation omitted
//! entirely — which is why no CLMM swap it built could ever have executed.
//!
//! # Where a tick lives
//!
//! Ticks are stored 60 to an account. An array's `start_index` is the tick
//! floor-divided by `tick_spacing * 60`, and the account is a PDA of
//! `["tick_array", pool, start_index_big_endian]`.
//!
//! # Which arrays exist
//!
//! `PoolState.tick_array_bitmap` is 1024 bits covering array indices −512..511
//! around zero. A pool whose price has travelled further keeps the rest in a
//! separate `TickArrayBitmapExtension` account: 14 blocks of 512 bits either
//! side, each block covering one more `tick_spacing * 60 * 512` span of ticks.
//!
//! Scanning the bitmap rather than deriving neighbours arithmetically matters:
//! an uninitialised tick array is an account that does not exist, and passing
//! one fails the instruction on deserialisation.

use super::layout::{i128_at, i32_at, pubkey_at, u128_at, u64_at};
use super::math::{ceil_div, mul_div_ceil, mul_div_floor};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::swaps::direct::error::{DirectSwapError, DirectSwapResult};

/// Ticks stored per tick-array account.
pub const TICK_ARRAY_SIZE: i32 = 60;

/// Bits in the pool's own bitmap, i.e. array indices −512..511.
const POOL_BITMAP_BITS: i32 = 1_024;

/// Blocks of 512 bits either side of the pool bitmap in the extension account.
const EXTENSION_BLOCKS: usize = 14;

/// Bits per extension block.
const EXTENSION_BLOCK_BITS: i32 = 512;

/// Tick arrays a swap instruction carries. Raydium's own client passes three.
pub const TICK_ARRAYS_PER_SWAP: usize = 3;

const TICK_ARRAY_SEED: &[u8] = b"tick_array";
const BITMAP_EXTENSION_SEED: &[u8] = b"pool_tick_array_bitmap_extension";

/// Ticks spanned by one tick-array account.
pub fn ticks_per_array(tick_spacing: u16) -> i32 {
    TICK_ARRAY_SIZE * (tick_spacing as i32)
}

/// Ticks covered by the pool's own bitmap, either side of zero.
pub fn pool_bitmap_reach(tick_spacing: u16) -> i32 {
    ticks_per_array(tick_spacing) * (POOL_BITMAP_BITS / 2)
}

/// The start index of the tick array containing `tick`.
///
/// Floor division, not truncation: `-1 / 60` truncates to `0`, which would put
/// every negative tick just below a boundary into the array ABOVE it.
pub fn tick_array_start_index(tick: i32, tick_spacing: u16) -> i32 {
    let span = ticks_per_array(tick_spacing);
    let mut index = tick / span;
    if tick < 0 && tick % span != 0 {
        index -= 1;
    }
    index * span
}

/// The PDA of a tick array account. `start_index` is serialised BIG-endian.
pub fn tick_array_address(program: &Pubkey, pool: &Pubkey, start_index: i32) -> Pubkey {
    Pubkey::find_program_address(
        &[TICK_ARRAY_SEED, pool.as_ref(), &start_index.to_be_bytes()],
        program,
    )
    .0
}

/// The PDA of a pool's tick-array bitmap extension.
pub fn bitmap_extension_address(program: &Pubkey, pool: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[BITMAP_EXTENSION_SEED, pool.as_ref()], program).0
}

/// The smallest and largest tick the programme's own price math is defined
/// over. Verified against Raydium's published `tick_math.rs`
/// (`raydium-io/raydium-clmm`), not re-derived.
pub const MIN_TICK: i32 = -443_636;
pub const MAX_TICK: i32 = 443_636;

/// `1.0001^(tick/2)` as a Q64.64 sqrt price, computed as the exact integer
/// chain of 128-bit magic-constant multiplications Raydium's own programme
/// uses (`get_sqrt_price_at_tick` in `libraries/tick_math.rs`), not the
/// `1.0001_f64.powf(...)` approximation the previous version of this venue
/// used as a range GUARD. An `f64` carries 53 bits of mantissa; a Q64.64 value
/// needs all 128, and this is no longer only a guard -- it decides which tick
/// a swap step lands on, so an approximation here is a wrong `min_out`, not
/// just an early refusal.
///
/// Verified against live mainnet state: for the SOL/USDC CLMM pool
/// `3ucNos4NbumPLZNWztqGHNFFgkHeRMBQAVemeeomsUxv` at `tick_current = -22882`,
/// `sqrt_price_x64 = 5876023812037193314`, this function brackets the pool's
/// own price exactly: `get_sqrt_price_at_tick(-22882) = 5875816817492017904 <=
/// 5876023812037193314 < get_sqrt_price_at_tick(-22881) = 5876110600988489675`.
pub fn get_sqrt_price_at_tick(tick: i32) -> Option<u128> {
    if tick < MIN_TICK || tick > MAX_TICK {
        return None;
    }
    let abs_tick = tick.unsigned_abs();

    // Each magic factor is `2^64 / (1.0001^(2^(i-1)))` for i in 0..19,
    // matching the constants in Raydium's `get_sqrt_price_at_tick` bit for bit.
    const MAGIC: [(u32, u128); 19] = [
        (0x1, 0xfffcb933bd6fb800),
        (0x2, 0xfff97272373d4000),
        (0x4, 0xfff2e50f5f657000),
        (0x8, 0xffe5caca7e10f000),
        (0x10, 0xffcb9843d60f7000),
        (0x20, 0xff973b41fa98e800),
        (0x40, 0xff2ea16466c9b000),
        (0x80, 0xfe5dee046a9a3800),
        (0x100, 0xfcbe86c7900bb000),
        (0x200, 0xf987a7253ac65800),
        (0x400, 0xf3392b0822bb6000),
        (0x800, 0xe7159475a2caf000),
        (0x1000, 0xd097f3bdfd2f2000),
        (0x2000, 0xa9f746462d9f8000),
        (0x4000, 0x70d869a156f31c00),
        (0x8000, 0x31be135f97ed3200),
        (0x10000, 0x9aa508b5b85a500),
        (0x20000, 0x5d6af8dedc582c),
        (0x40000, 0x2216e584f5fa),
    ];

    let mut ratio: u128 = if abs_tick & 0x1 != 0 {
        0xfffcb933bd6fb800
    } else {
        1u128 << 64
    };
    for &(mask, magic) in &MAGIC[1..] {
        if abs_tick & mask != 0 {
            ratio = ratio.checked_mul(magic)?.checked_shr(64)?;
        }
    }

    if tick > 0 {
        ratio = u128::MAX.checked_div(ratio)?;
    }
    Some(ratio)
}

/// One initialised tick out of a decoded `TickArrayState`: the entries whose
/// `liquidity_gross` is non-zero, which is what an active position actually
/// requires -- a slot with `tick == 0, liquidity_net == 0, liquidity_gross ==
/// 0` is simply an unused array slot, not a real tick at index 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitializedTick {
    pub tick: i32,
    pub liquidity_net: i128,
}

/// Bytes from the start of a `TickArrayState` account to its first `TickState`
/// entry: an 8-byte Anchor discriminator, a 32-byte `pool_id`, and the 4-byte
/// `start_tick_index`.
///
/// Verified against live mainnet bytes, NOT the offset originally guessed for
/// this module (`start_tick_index` at 8): the real `TickArrayState` carries a
/// `pool_id: Pubkey` field between the discriminator and `start_tick_index`
/// that the guess omitted. Confirmed two ways against tick array
/// `7KGRHr8gSwVqmJVv3sdnUEmKM3jRC551SMCt9ZxmCXsb` (start index -22920) and
/// `5LuEHwAuoPAEJunEvBcDnTwRGnXWtC7JQmAgeXzA44cV` (start index -22980), both
/// derived PDAs of the SOL/USDC pool above: the on-chain Anchor IDL (pulled per
/// `adding-a-venue.md`) declares this exact layout, and decoding both accounts
/// at this offset reproduces `start_tick_index + i * tick_spacing` for all 60
/// `tick` fields with `tick_spacing = 1`.
const TICKS_OFFSET: usize = 44;

/// One `TickState` entry's byte size. `168` was the size named in the
/// unverified starting hypothesis and IS correct — confirmed by IDL field
/// layout (i32 + i128 + u128 + u128 + u128 + u128*3 + u64*3 + u128 + u32*3 =
/// 168 bytes) and independently by live bytes: `TICKS_OFFSET + 60 * 168 + 1 +
/// 8 + 107 == 10240`, the exact account length fetched from chain.
const TICK_STATE_SIZE: usize = 168;

/// Decode the initialised ticks out of a live `TickArrayState` account.
///
/// Returns `None` when the account does not belong to `pool` or is too short
/// to hold a full array -- a decode failure, never a partially-wrong swap.
pub fn decode_tick_array(pool: &Pubkey, data: &[u8]) -> Option<Vec<InitializedTick>> {
    if pubkey_at(data, 8)? != *pool {
        return None;
    }
    let mut ticks = Vec::new();
    for i in 0..(TICK_ARRAY_SIZE as usize) {
        let offset = TICKS_OFFSET + i * TICK_STATE_SIZE;
        let tick = i32_at(data, offset)?;
        let liquidity_net = i128_at(data, offset + 4)?;
        let liquidity_gross = u128_at(data, offset + 20)?;
        if liquidity_gross != 0 {
            ticks.push(InitializedTick {
                tick,
                liquidity_net,
            });
        }
    }
    Some(ticks)
}

/// A pool's initialised-tick-array bitmap: the pool's own 1024 bits plus, when
/// the extension account exists, the blocks either side of it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TickArrayBitmap {
    pool: [u64; 16],
    positive: Vec<[u64; 8]>,
    negative: Vec<[u64; 8]>,
}

impl TickArrayBitmap {
    /// Read the pool's own bitmap out of `PoolState` at offset 904.
    pub fn from_pool_state(data: &[u8]) -> Option<Self> {
        let mut pool = [0u64; 16];
        for (index, word) in pool.iter_mut().enumerate() {
            *word = u64_at(data, 904 + index * 8)?;
        }
        Some(Self {
            pool,
            positive: Vec::new(),
            negative: Vec::new(),
        })
    }

    /// Attach a `TickArrayBitmapExtension` account, if the pool has one.
    ///
    /// Layout: discriminator, `pool_id`, then 14 positive blocks of `[u64; 8]`
    /// followed by 14 negative blocks. A wrong `pool_id` is ignored rather than
    /// trusted — the extension is a PDA, but reading someone else's bitmap would
    /// send the swap to tick arrays that belong to another pool.
    pub fn with_extension(mut self, pool_id: &Pubkey, data: &[u8]) -> Self {
        if pubkey_at(data, 8) != Some(*pool_id) {
            return self;
        }
        let base = 40;
        let block = |offset: usize| -> Option<[u64; 8]> {
            let mut words = [0u64; 8];
            for (index, word) in words.iter_mut().enumerate() {
                *word = u64_at(data, offset + index * 8)?;
            }
            Some(words)
        };
        for i in 0..EXTENSION_BLOCKS {
            match block(base + i * 64) {
                Some(words) => self.positive.push(words),
                None => return self,
            }
        }
        for i in 0..EXTENSION_BLOCKS {
            match block(base + EXTENSION_BLOCKS * 64 + i * 64) {
                Some(words) => self.negative.push(words),
                None => return self,
            }
        }
        self
    }

    /// Whether the tick array starting at `start_index` is initialised.
    ///
    /// An index the bitmap cannot describe reads as NOT initialised. That is the
    /// safe answer: the caller skips it rather than passing an account that may
    /// not exist.
    pub fn is_initialised(&self, start_index: i32, tick_spacing: u16) -> bool {
        let span = ticks_per_array(tick_spacing);
        if span == 0 {
            return false;
        }
        let array_index = start_index / span;

        if (-(POOL_BITMAP_BITS / 2)..(POOL_BITMAP_BITS / 2)).contains(&array_index) {
            let bit = (array_index + POOL_BITMAP_BITS / 2) as usize;
            return self.pool[bit / 64] & (1u64 << (bit % 64)) != 0;
        }

        let (blocks, distance) = if array_index >= 0 {
            (&self.positive, array_index - POOL_BITMAP_BITS / 2)
        } else {
            (&self.negative, -array_index - POOL_BITMAP_BITS / 2 - 1)
        };
        let block_index = (distance / EXTENSION_BLOCK_BITS) as usize;
        let Some(block) = blocks.get(block_index) else {
            return false;
        };
        let bit = (distance % EXTENSION_BLOCK_BITS) as usize;
        block[bit / 64] & (1u64 << (bit % 64)) != 0
    }

    /// The tick arrays a swap starting at `tick_current` may touch, in the order
    /// the instruction expects: the array holding the current tick first, then
    /// the next initialised ones in the direction the price is moving.
    ///
    /// `zero_for_one` means token_0 is being sold, which moves the price DOWN.
    pub fn arrays_for_swap(
        &self,
        tick_current: i32,
        tick_spacing: u16,
        zero_for_one: bool,
    ) -> Vec<i32> {
        let span = ticks_per_array(tick_spacing);
        if span == 0 {
            return Vec::new();
        }
        let step = if zero_for_one { -span } else { span };
        let reach = pool_bitmap_reach(tick_spacing)
            + span * EXTENSION_BLOCK_BITS * (EXTENSION_BLOCKS as i32);

        let mut found = Vec::with_capacity(TICK_ARRAYS_PER_SWAP);
        let mut start = tick_array_start_index(tick_current, tick_spacing);
        while found.len() < TICK_ARRAYS_PER_SWAP && start.abs() <= reach {
            if self.is_initialised(start, tick_spacing) {
                found.push(start);
            }
            let Some(next) = start.checked_add(step) else {
                break;
            };
            start = next;
        }
        found
    }
}

// ============================================================================
// SHARED TICK-WALK STEP MATH
// ============================================================================
//
// Everything below is the constant-liquidity, Q64.64 step arithmetic that a
// concentrated-liquidity swap performs between two initialised ticks. It is
// program-agnostic -- Raydium CLMM and Orca Whirlpool both implement the same
// Uniswap-v3-style curve over the same sqrt-price representation -- so it is
// lifted here rather than duplicated per venue. A venue's own `walk()` still
// owns its fee rate, its tick source and its pool identity for error
// reporting; only the per-step formulas live here.

/// The initialised ticks a walk may cross, in the order it will meet them:
/// descending from (and including) the current tick when selling the "0"/"A"
/// side (price falling), ascending above it when selling the "1"/"B" side
/// (price rising). `ticks` must already be sorted ascending by `tick`.
pub fn ticks_ahead(
    ticks: &[InitializedTick],
    tick_current: i32,
    zero_for_one: bool,
) -> Vec<InitializedTick> {
    if zero_for_one {
        ticks
            .iter()
            .rev()
            .filter(|t| t.tick <= tick_current)
            .copied()
            .collect()
    } else {
        ticks
            .iter()
            .filter(|t| t.tick > tick_current)
            .copied()
            .collect()
    }
}

/// The output produced by moving the price at constant `liquidity` from
/// `sqrt_from` to `sqrt_to`, rounded DOWN -- the pool never owes more than it
/// computed.
pub fn output_for_move(
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
        let numerator = liquidity
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

/// The sqrt price reached by spending `amount_in` (already net of the fee) at
/// constant `liquidity` from `sqrt_price` -- the partial-step case, when the
/// input runs out before the next tick.
pub fn next_sqrt_price_from_input(
    zero_for_one: bool,
    liquidity: u128,
    sqrt_price: u128,
    amount_in: u128,
) -> DirectSwapResult<u128> {
    let overflow = || DirectSwapError::QuoteMath {
        detail: "the concentrated-liquidity step overflowed 256 bits".to_owned(),
    };
    if zero_for_one {
        let numerator = liquidity
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
/// Understating this would cross a tick boundary -- and apply its
/// `liquidity_net` -- without having paid for reaching it, which is a step the
/// programme never takes.
pub fn input_for_move(
    zero_for_one: bool,
    liquidity: u128,
    sqrt_from: u128,
    sqrt_to: u128,
) -> DirectSwapResult<u128> {
    let overflow = || DirectSwapError::QuoteMath {
        detail: "the concentrated-liquidity input overflowed 256 bits".to_owned(),
    };
    if zero_for_one {
        // Side 0/A in, price falling: sqrt_from (high) > sqrt_to (low).
        if sqrt_from <= sqrt_to {
            return Ok(0);
        }
        let numerator = liquidity
            .checked_shl(64)
            .ok_or_else(|| DirectSwapError::QuoteMath {
                detail: "liquidity is too large to scale to Q64.64".to_owned(),
            })?;
        let spread = sqrt_from - sqrt_to;
        let step1 = mul_div_ceil(numerator, spread, sqrt_from).ok_or_else(overflow)?;
        Ok(ceil_div(step1, sqrt_to.max(1)))
    } else {
        // Side 1/B in, price rising: sqrt_to (high) > sqrt_from (low).
        if sqrt_to <= sqrt_from {
            return Ok(0);
        }
        let spread = sqrt_to - sqrt_from;
        mul_div_ceil(liquidity, spread, 1u128 << 64).ok_or_else(overflow)
    }
}

/// Apply a crossed tick's `liquidity_net` to `liquidity`: ADDED when the price
/// is moving up (`!zero_for_one`), SUBTRACTED when it is moving down --
/// `liquidity_net` is always defined for an upward crossing, so the downward
/// direction negates it.
pub fn cross_tick(
    pool: Pubkey,
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
            pool,
            amount_in: 0,
            detail: format!(
                "crossing tick {} would take pool liquidity negative -- the loaded tick data \
                 is inconsistent with the pool's own reported liquidity",
                tick.tick
            ),
        })
}

/// Walk a swap tick by tick from `sqrt_price`/`liquidity`, exactly as a
/// concentrated-liquidity programme's own loop does: constant liquidity
/// between initialised ticks, a fee charged per step, `liquidity_net` applied
/// at every crossing. `amount_in` is already net of any Token-2022 transfer
/// fee on the input -- it is what the pool itself receives to swap and to fee.
///
/// `candidates` must already be ordered the way the walk will meet them (see
/// [`ticks_ahead`]). Returns `(total_output, total_lp_fee, ending_sqrt_price)`.
/// Refuses, rather than approximates, once the walk would cross beyond the
/// last candidate tick.
#[allow(clippy::too_many_arguments)]
pub fn walk_ticks(
    pool: Pubkey,
    candidates: &[InitializedTick],
    mut liquidity: u128,
    mut sqrt_price: u128,
    fee_rate: u64,
    fee_rate_denominator: u64,
    zero_for_one: bool,
    amount_in: u64,
) -> DirectSwapResult<(u64, u64, u128)> {
    let rate = fee_rate as u128;
    let denom = fee_rate_denominator as u128;
    if rate >= denom {
        return Err(DirectSwapError::QuoteMath {
            detail: "the trade fee rate is not below its own denominator".to_owned(),
        });
    }

    let mut remaining = amount_in as u128;
    let mut total_out: u128 = 0;
    let mut total_fee: u128 = 0;
    let mut candidates = candidates.iter();

    while remaining > 0 {
        if liquidity == 0 || sqrt_price == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool,
                amount_in,
                detail: "liquidity was exhausted mid-walk".to_owned(),
            });
        }

        let Some(next_tick) = candidates.next() else {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool,
                amount_in,
                detail: "the size travels further than the loaded tick arrays cover".to_owned(),
            });
        };
        let target_sqrt =
            get_sqrt_price_at_tick(next_tick.tick).ok_or_else(|| DirectSwapError::QuoteMath {
                detail: format!(
                    "tick {} lies outside the programme's own range",
                    next_tick.tick
                ),
            })?;

        let amount_to_target = input_for_move(zero_for_one, liquidity, sqrt_price, target_sqrt)?;
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
            let fee = (gross_needed - amount_to_target).min(u64::MAX as u128);
            let out = output_for_move(zero_for_one, liquidity, sqrt_price, target_sqrt)?;
            total_out += out as u128;
            total_fee += fee;
            remaining -= gross_needed;
            sqrt_price = target_sqrt;
            liquidity = cross_tick(pool, liquidity, next_tick, zero_for_one)?;
        } else {
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
            let sqrt_next =
                next_sqrt_price_from_input(zero_for_one, liquidity, sqrt_price, swappable)?;
            let out = output_for_move(zero_for_one, liquidity, sqrt_price, sqrt_next)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sqrt_price_at_tick_brackets_a_real_live_pools_own_price() {
        // Pool 3ucNos4NbumPLZNWztqGHNFFgkHeRMBQAVemeeomsUxv (Raydium CLMM
        // SOL/USDC), fetched live: tick_current = -22882, sqrt_price_x64 =
        // 5876023812037193314. The current price always sits between the sqrt
        // price of the current tick and the next one -- if this function's
        // integer chain were wrong, it would not bracket a real pool's own
        // reported price.
        let tick_current = -22_882;
        let actual_sqrt_price: u128 = 5_876_023_812_037_193_314;
        let lower = get_sqrt_price_at_tick(tick_current).unwrap();
        let upper = get_sqrt_price_at_tick(tick_current + 1).unwrap();
        assert!(
            lower <= actual_sqrt_price,
            "lower bound must not exceed the real price"
        );
        assert!(
            upper > actual_sqrt_price,
            "upper bound must exceed the real price"
        );
    }

    #[test]
    fn tick_zero_is_the_identity_price() {
        assert_eq!(get_sqrt_price_at_tick(0), Some(1u128 << 64));
    }

    #[test]
    fn a_tick_outside_the_programmes_own_range_is_refused() {
        assert!(get_sqrt_price_at_tick(MIN_TICK - 1).is_none());
        assert!(get_sqrt_price_at_tick(MAX_TICK + 1).is_none());
        assert!(get_sqrt_price_at_tick(MIN_TICK).is_some());
        assert!(get_sqrt_price_at_tick(MAX_TICK).is_some());
    }

    #[test]
    fn a_positive_and_its_negative_tick_are_reciprocal_prices() {
        // sqrt(1.0001^t) * sqrt(1.0001^-t) == 1, which in Q64.64 means
        // up * down == 2^128. That product does not fit a u128, so it is
        // divided back down through the same 256-bit helper the venue itself
        // uses rather than multiplied directly -- `up * down` would overflow,
        // and comparing as f64 would hide a real sign or branch bug in
        // `get_sqrt_price_at_tick`.
        for tick in [1_000i32, 50_000, MAX_TICK - 1, MAX_TICK] {
            let up = get_sqrt_price_at_tick(tick).unwrap();
            let down = get_sqrt_price_at_tick(-tick).unwrap();
            let round_trip = super::super::math::mul_div_floor(up, down, 1u128 << 64)
                .expect("the product of reciprocal sqrt prices fits back into Q64.64");
            let expected = 1u128 << 64;
            let diff = round_trip.abs_diff(expected);
            assert!(
                diff <= 2,
                "tick {tick}: sqrt(1.0001^t) * sqrt(1.0001^-t) should be 1 within a \
                 couple of ULP, got round trip {round_trip} vs {expected}"
            );
        }
    }

    #[test]
    fn decode_tick_array_reads_only_the_entries_with_gross_liquidity() {
        let pool = Pubkey::new_unique();
        let mut data = vec![0u8; TICKS_OFFSET + (TICK_ARRAY_SIZE as usize) * TICK_STATE_SIZE + 200];
        data[8..40].copy_from_slice(&pool.to_bytes());
        data[40..44].copy_from_slice(&(-60i32).to_le_bytes());

        // Slot 0: a real initialised tick.
        let slot0 = TICKS_OFFSET;
        data[slot0..slot0 + 4].copy_from_slice(&(-60i32).to_le_bytes());
        data[slot0 + 4..slot0 + 20].copy_from_slice(&(12_345i128).to_le_bytes());
        data[slot0 + 20..slot0 + 36].copy_from_slice(&(999_999u128).to_le_bytes());

        // Slot 1: left at all zero -- an unused slot, not tick 0.
        // Slot 2: a negative liquidity_net, still initialised.
        let slot2 = TICKS_OFFSET + 2 * TICK_STATE_SIZE;
        data[slot2..slot2 + 4].copy_from_slice(&(-58i32).to_le_bytes());
        data[slot2 + 4..slot2 + 20].copy_from_slice(&(-500i128).to_le_bytes());
        data[slot2 + 20..slot2 + 36].copy_from_slice(&(1u128).to_le_bytes());

        let ticks = decode_tick_array(&pool, &data).expect("a full-length array decodes");
        assert_eq!(ticks.len(), 2, "the untouched zero slot must not appear");
        assert_eq!(
            ticks[0],
            InitializedTick {
                tick: -60,
                liquidity_net: 12_345
            }
        );
        assert_eq!(
            ticks[1],
            InitializedTick {
                tick: -58,
                liquidity_net: -500
            }
        );
    }

    #[test]
    fn decode_tick_array_refuses_an_account_belonging_to_another_pool() {
        let pool = Pubkey::new_unique();
        let stranger = Pubkey::new_unique();
        let mut data = vec![0u8; TICKS_OFFSET + (TICK_ARRAY_SIZE as usize) * TICK_STATE_SIZE];
        data[8..40].copy_from_slice(&stranger.to_bytes());
        assert!(decode_tick_array(&pool, &data).is_none());
    }

    #[test]
    fn decode_tick_array_refuses_a_truncated_account_rather_than_reading_short() {
        let pool = Pubkey::new_unique();
        let mut data = vec![0u8; TICKS_OFFSET + 10];
        data[8..40].copy_from_slice(&pool.to_bytes());
        assert!(decode_tick_array(&pool, &data).is_none());
    }

    #[test]
    fn a_tick_array_start_index_floors_towards_negative_infinity() {
        // Spacing 1 -> 60 ticks per array.
        assert_eq!(tick_array_start_index(0, 1), 0);
        assert_eq!(tick_array_start_index(59, 1), 0);
        assert_eq!(tick_array_start_index(60, 1), 60);
        assert_eq!(
            tick_array_start_index(-1, 1),
            -60,
            "truncation would put -1 in the array starting at 0"
        );
        assert_eq!(tick_array_start_index(-60, 1), -60);
        assert_eq!(tick_array_start_index(-61, 1), -120);
    }

    #[test]
    fn the_live_sol_usdc_tick_lands_in_the_array_the_chain_uses() {
        // tick_current -23426, spacing 1 -> -23426 / 60 = -390.4 -> -391 -> -23460.
        assert_eq!(tick_array_start_index(-23_426, 1), -23_460);
    }

    #[test]
    fn wider_spacing_widens_the_array_span() {
        assert_eq!(ticks_per_array(1), 60);
        assert_eq!(ticks_per_array(60), 3_600);
        assert_eq!(tick_array_start_index(3_599, 60), 0);
        assert_eq!(tick_array_start_index(3_600, 60), 3_600);
    }

    #[test]
    fn a_zero_spacing_pool_cannot_produce_arrays_instead_of_dividing_by_zero() {
        let bitmap = TickArrayBitmap::default();
        assert!(bitmap.arrays_for_swap(0, 0, true).is_empty());
        assert!(!bitmap.is_initialised(0, 0));
    }

    fn bitmap_with_pool_bits(bits: &[i32]) -> TickArrayBitmap {
        let mut pool = [0u64; 16];
        for array_index in bits {
            let bit = (array_index + POOL_BITMAP_BITS / 2) as usize;
            pool[bit / 64] |= 1u64 << (bit % 64);
        }
        TickArrayBitmap {
            pool,
            positive: Vec::new(),
            negative: Vec::new(),
        }
    }

    #[test]
    fn only_the_arrays_the_bitmap_marks_are_reported_as_initialised() {
        let bitmap = bitmap_with_pool_bits(&[0, -1, 5]);
        assert!(bitmap.is_initialised(0, 1));
        assert!(bitmap.is_initialised(-60, 1), "array index -1");
        assert!(bitmap.is_initialised(300, 1), "array index 5");
        assert!(!bitmap.is_initialised(60, 1), "array index 1 is not set");
    }

    #[test]
    fn an_index_beyond_the_bitmap_reads_as_uninitialised_rather_than_panicking() {
        let bitmap = bitmap_with_pool_bits(&[0]);
        assert!(!bitmap.is_initialised(i32::MAX / 2, 1));
        assert!(!bitmap.is_initialised(i32::MIN / 2, 1));
    }

    #[test]
    fn a_downward_swap_walks_to_lower_arrays_and_an_upward_swap_to_higher_ones() {
        let bitmap = bitmap_with_pool_bits(&[-2, -1, 0, 1, 2]);
        assert_eq!(
            bitmap.arrays_for_swap(10, 1, true),
            vec![0, -60, -120],
            "selling token_0 moves the price down"
        );
        assert_eq!(
            bitmap.arrays_for_swap(10, 1, false),
            vec![0, 60, 120],
            "buying token_0 moves the price up"
        );
    }

    #[test]
    fn uninitialised_arrays_are_skipped_rather_than_passed_to_the_programme() {
        let bitmap = bitmap_with_pool_bits(&[0, 3, 7]);
        assert_eq!(
            bitmap.arrays_for_swap(0, 1, false),
            vec![0, 180, 420],
            "an account that does not exist would fail the instruction"
        );
    }

    #[test]
    fn a_pool_with_no_initialised_arrays_yields_none() {
        let bitmap = TickArrayBitmap::default();
        assert!(bitmap.arrays_for_swap(0, 1, true).is_empty());
    }

    #[test]
    fn the_extension_is_ignored_when_it_belongs_to_another_pool() {
        let pool = Pubkey::new_unique();
        let stranger = Pubkey::new_unique();
        let mut data = vec![0u8; 1_832];
        data[8..40].copy_from_slice(&stranger.to_bytes());
        // Set every extension bit; none of them may be believed.
        for byte in data.iter_mut().skip(40) {
            *byte = 0xFF;
        }
        let bitmap = TickArrayBitmap::default().with_extension(&pool, &data);
        assert!(
            !bitmap.is_initialised(600 * 60, 1),
            "another pool's bitmap must never route our swap"
        );
    }

    #[test]
    fn an_extension_for_this_pool_extends_the_reach_past_the_pool_bitmap() {
        let pool = Pubkey::new_unique();
        let mut data = vec![0u8; 1_832];
        data[8..40].copy_from_slice(&pool.to_bytes());
        // Positive block 0, bit 0 -> array index 512.
        data[40] = 0x01;
        let bitmap = TickArrayBitmap::default().with_extension(&pool, &data);
        assert!(bitmap.is_initialised(512 * 60, 1));
        assert!(!bitmap.is_initialised(513 * 60, 1));
    }

    #[test]
    fn a_truncated_extension_leaves_the_pool_bitmap_intact() {
        let pool = Pubkey::new_unique();
        let mut data = vec![0u8; 100];
        data[8..40].copy_from_slice(&pool.to_bytes());
        let bitmap = bitmap_with_pool_bits(&[0]).with_extension(&pool, &data);
        assert!(
            bitmap.is_initialised(0, 1),
            "the pool's own bits still work"
        );
    }

    #[test]
    fn the_pool_bitmap_is_read_from_offset_904() {
        let mut data = vec![0u8; 1_544];
        data[904..912].copy_from_slice(&0b1010u64.to_le_bytes());
        let bitmap = TickArrayBitmap::from_pool_state(&data).expect("full-length state decodes");
        // Word 0, bits 1 and 3 -> array indices -511 and -509.
        assert!(bitmap.is_initialised(-511 * 60, 1));
        assert!(bitmap.is_initialised(-509 * 60, 1));
        assert!(!bitmap.is_initialised(-512 * 60, 1));
    }

    #[test]
    fn a_short_pool_account_has_no_bitmap_rather_than_a_wrong_one() {
        assert!(TickArrayBitmap::from_pool_state(&[0u8; 500]).is_none());
    }
}
