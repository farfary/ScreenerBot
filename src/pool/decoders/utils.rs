use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Utility functions for decoding binary data

/// Convert bytes to u16 (little-endian)
pub fn bytes_to_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

/// Convert bytes to u32 (little-endian)
pub fn bytes_to_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Convert bytes to u64 (little-endian)
pub fn bytes_to_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
    ])
}

/// Convert bytes to u128 (little-endian)
pub fn bytes_to_u128(bytes: &[u8]) -> u128 {
    u128::from_le_bytes([
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    ])
}

/// Convert bytes to Pubkey
pub fn bytes_to_pubkey(bytes: &[u8]) -> Pubkey {
    Pubkey::new_from_array([
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
        bytes[16],
        bytes[17],
        bytes[18],
        bytes[19],
        bytes[20],
        bytes[21],
        bytes[22],
        bytes[23],
        bytes[24],
        bytes[25],
        bytes[26],
        bytes[27],
        bytes[28],
        bytes[29],
        bytes[30],
        bytes[31],
    ])
}

/// Convert bytes to i32 (little-endian)
pub fn bytes_to_i32(bytes: &[u8]) -> i32 {
    i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Convert bytes to f64 (little-endian)
pub fn bytes_to_f64(bytes: &[u8]) -> f64 {
    f64::from_le_bytes([
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
    ])
}

/// Convert bytes to i64 (little-endian)
pub fn bytes_to_i64(bytes: &[u8]) -> i64 {
    i64::from_le_bytes([
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
    ])
}

/// Convert bytes to Option<Pubkey>
pub fn bytes_to_option_pubkey(bytes: &[u8]) -> Option<Pubkey> {
    if bytes.len() < 32 {
        return None;
    }
    let pubkey = bytes_to_pubkey(bytes);
    if pubkey == Pubkey::default() {
        None
    } else {
        Some(pubkey)
    }
}

/// Convert bytes to string (null-terminated)
pub fn bytes_to_string(bytes: &[u8]) -> String {
    let null_pos = bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..null_pos]).to_string()
}

/// Calculate square root of a u128 number
pub fn sqrt_u128(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }

    let mut x = n;
    let mut y = (x + 1) / 2;

    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }

    x
}

/// Calculate price from sqrt price (used in CLMM)
pub fn sqrt_price_to_price(sqrt_price_x64: u128) -> f64 {
    if sqrt_price_x64 == 0 {
        return 0.0;
    }

    // Convert from Q64.64 fixed point to floating point
    let sqrt_price = (sqrt_price_x64 as f64) / ((1u128 << 64) as f64);

    // Price = sqrt_price^2
    sqrt_price * sqrt_price
}

/// Calculate tick price for concentrated liquidity
pub fn tick_to_price(tick: i32) -> f64 {
    // Price = 1.0001^tick
    (1.0001_f64).powi(tick)
}

/// Calculate price from tick index (for CLMM)
pub fn tick_index_to_price(tick_index: i32) -> f64 {
    const TICK_SPACING: i32 = 1;
    let tick = tick_index * TICK_SPACING;
    tick_to_price(tick)
}

/// Validate discriminator bytes
pub fn validate_discriminator(data: &[u8], expected: &[u8]) -> bool {
    if data.len() < expected.len() {
        return false;
    }
    &data[0..expected.len()] == expected
}
