// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Fixed-offset reads over raw Solana account data.
//!
//! The one reader set for program account layouts, shared by the direct swap
//! venues and the pool price decoders. Every read is bounds-checked and returns
//! `None` (or, for the cursor readers, an error) rather than panicking. A pool
//! account that is one byte short, or a program that changed its layout, must
//! degrade into a decode failure the caller can report — never into a panic
//! inside a task that is about to spend money.
//!
//! The `read_*_at_offset` cursor readers walk a layout field by field: each one
//! performs the matching fixed-offset read and advances the cursor only when
//! that read succeeded.

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::{Error, Result};

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

/// Read a little-endian `i128` at `offset`.
pub fn i128_at(data: &[u8], offset: usize) -> Option<i128> {
    Some(i128::from_le_bytes(
        data.get(offset..offset + 16)?.try_into().ok()?,
    ))
}

/// Read a little-endian `u16` at `offset`.
pub fn u16_at(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

/// Read a little-endian `u32` at `offset`.
pub fn u32_at(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
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

/// Decimals out of an SPL mint account, at the one offset both token
/// programmes share: a 36-byte `COption<Pubkey>` mint authority, then an
/// 8-byte supply. Token-2022's extensions all live PAST the base layout, so
/// the same read serves both.
pub fn mint_decimals(data: &[u8]) -> Option<u8> {
    u8_at(data, 44)
}

/// The SPL token account balance, which lives at a fixed offset in both the
/// legacy and the Token-2022 account layouts.
pub fn token_account_amount(data: &[u8]) -> Option<u64> {
    u64_at(data, 64)
}

/// Read a single byte at `*offset`, advancing the cursor by 1.
pub fn read_u8_at_offset(data: &[u8], offset: &mut usize) -> Result<u8> {
    let value = u8_at(data, *offset).ok_or_else(|| Error::Decode {
        payload: "u8",
        detail: format!(
            "offset {} runs past {} bytes of account data",
            *offset,
            data.len()
        ),
    })?;
    *offset += 1;
    Ok(value)
}

/// Read a little-endian `u64` at `*offset`, advancing the cursor by 8.
pub fn read_u64_at_offset(data: &[u8], offset: &mut usize) -> Result<u64> {
    let value = u64_at(data, *offset).ok_or_else(|| Error::Decode {
        payload: "u64",
        detail: format!(
            "offset {} + 8 runs past {} bytes of account data",
            *offset,
            data.len()
        ),
    })?;
    *offset += 8;
    Ok(value)
}

/// Read a one-byte flag at `*offset` (any non-zero byte is `true`), advancing
/// the cursor by 1.
pub fn read_bool_at_offset(data: &[u8], offset: &mut usize) -> Result<bool> {
    let value = u8_at(data, *offset).ok_or_else(|| Error::Decode {
        payload: "bool",
        detail: format!(
            "offset {} runs past {} bytes of account data",
            *offset,
            data.len()
        ),
    })?;
    *offset += 1;
    Ok(value != 0)
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
        assert!(i128_at(&short, 0).is_none());
        assert!(u32_at(&short, 1).is_none());
        assert!(u8_at(&short, 9).is_none());
        assert!(token_account_amount(&short).is_none());
    }

    #[test]
    fn a_cursor_read_past_the_end_errors_and_leaves_the_cursor_in_place() {
        let short = [0u8; 4];
        let mut offset = 4;
        assert!(read_u8_at_offset(&short, &mut offset).is_err());
        assert!(read_bool_at_offset(&short, &mut offset).is_err());
        assert_eq!(offset, 4);
        let mut offset = 0;
        assert!(read_u64_at_offset(&short, &mut offset).is_err());
        assert_eq!(offset, 0);
    }

    #[test]
    fn cursor_reads_walk_the_layout_field_by_field() {
        let mut data = vec![7u8, 2u8];
        data.extend_from_slice(&9_876_543_210u64.to_le_bytes());
        let mut offset = 0;
        assert_eq!(read_u8_at_offset(&data, &mut offset).unwrap(), 7);
        assert!(read_bool_at_offset(&data, &mut offset).unwrap());
        assert_eq!(
            read_u64_at_offset(&data, &mut offset).unwrap(),
            9_876_543_210
        );
        assert_eq!(offset, 10);
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
