//! Fixed-offset reads over raw account data.
//!
//! Every venue decodes a program's account layout by absolute offset, and every
//! read here is bounds-checked and returns `None` rather than panicking. A pool
//! account that is one byte short, or a program that changed its layout, must
//! degrade into a decode failure the caller can report — never into a panic
//! inside a task that is about to spend money.

use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// Read a 32-byte pubkey at `offset`.
pub fn pubkey_at(data: &[u8], offset: usize) -> Option<Pubkey> {
    let bytes: [u8; 32] = data.get(offset..offset + 32)?.try_into().ok()?;
    Some(Pubkey::new_from_array(bytes))
}

/// Read a little-endian `u64` at `offset`.
pub fn u64_at(data: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        data.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

/// Read a little-endian `u128` at `offset`.
pub fn u128_at(data: &[u8], offset: usize) -> Option<u128> {
    Some(u128::from_le_bytes(
        data.get(offset..offset + 16)?.try_into().ok()?,
    ))
}

/// Read a little-endian `u16` at `offset`.
pub fn u16_at(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

/// Read a little-endian `i32` at `offset`.
pub fn i32_at(data: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

/// Read a single byte at `offset`.
pub fn u8_at(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

/// The SPL token account balance, which lives at a fixed offset in both the
/// legacy and the Token-2022 account layouts.
pub fn token_account_amount(data: &[u8]) -> Option<u64> {
    u64_at(data, 64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_read_past_the_end_returns_none_instead_of_panicking() {
        let short = [0u8; 4];
        assert!(pubkey_at(&short, 0).is_none());
        assert!(u64_at(&short, 0).is_none());
        assert!(u128_at(&short, 0).is_none());
        assert!(u16_at(&short, 3).is_none());
        assert!(i32_at(&short, 1).is_none());
        assert!(u8_at(&short, 9).is_none());
        assert!(token_account_amount(&short).is_none());
    }

    #[test]
    fn reads_are_little_endian_at_the_requested_offset() {
        let mut data = vec![0u8; 80];
        data[8..16].copy_from_slice(&1_234_567_890u64.to_le_bytes());
        data[64..72].copy_from_slice(&42u64.to_le_bytes());
        assert_eq!(u64_at(&data, 8), Some(1_234_567_890));
        assert_eq!(token_account_amount(&data), Some(42));
    }

    #[test]
    fn a_pubkey_round_trips_through_its_own_offset() {
        let key = Pubkey::new_unique();
        let mut data = vec![0u8; 100];
        data[40..72].copy_from_slice(&key.to_bytes());
        assert_eq!(pubkey_at(&data, 40), Some(key));
    }
}
