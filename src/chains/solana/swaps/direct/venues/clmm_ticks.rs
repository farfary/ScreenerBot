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

use super::layout::{pubkey_at, u64_at};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

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

#[cfg(test)]
mod tests {
    use super::*;

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
