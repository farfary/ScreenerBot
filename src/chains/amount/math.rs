// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Chain-neutral full-width integer ratio arithmetic.

// ============================================================================
// 256-BIT INTERMEDIATES
// ============================================================================
//
// Concentrated-liquidity math multiplies a Q64.64 sqrt price by a liquidity
// value and only then divides. Both operands reach the top of `u128`, so the
// product does not fit and the intermediate must be 256 bits wide. Dividing
// first instead would throw away the low bits that decide the output to the raw
// unit.

const LOW_64: u128 = u64::MAX as u128;

/// Full 256-bit product of two `u128`s, as `(high, low)`.
pub(crate) fn mul_full(a: u128, b: u128) -> (u128, u128) {
    let (a_hi, a_lo) = (a >> 64, a & LOW_64);
    let (b_hi, b_lo) = (b >> 64, b & LOW_64);

    let low_low = a_lo * b_lo;
    let low_high = a_lo * b_hi;
    let high_low = a_hi * b_lo;
    let high_high = a_hi * b_hi;

    let middle = (low_low >> 64) + (low_high & LOW_64) + (high_low & LOW_64);
    let low = (low_low & LOW_64) | (middle << 64);
    let high = high_high + (low_high >> 64) + (high_low >> 64) + (middle >> 64);
    (high, low)
}

/// Divide a 256-bit value by a `u128`, returning `None` when the quotient would
/// not fit in a `u128` (or the divisor is zero).
///
/// Long division one bit at a time. The loop keeps the invariant
/// `remainder < divisor`, so the only way the shifted remainder can exceed
/// `u128` is the top bit falling out — tracked as a carry, which always forces a
/// subtraction.
fn div_full(high: u128, low: u128, divisor: u128) -> Option<u128> {
    if divisor == 0 || high >= divisor {
        return None;
    }
    let mut remainder = high;
    let mut quotient: u128 = 0;
    for bit in (0..128).rev() {
        let carry = remainder >> 127;
        remainder = (remainder << 1) | ((low >> bit) & 1);
        if carry == 1 || remainder >= divisor {
            remainder = remainder.wrapping_sub(divisor);
            quotient |= 1 << bit;
        }
    }
    Some(quotient)
}

/// `a * b / denominator`, rounded DOWN, with a 256-bit intermediate.
pub(crate) fn mul_div_floor(a: u128, b: u128, denominator: u128) -> Option<u128> {
    let (high, low) = mul_full(a, b);
    div_full(high, low, denominator)
}

/// `a * b / denominator`, rounded UP, with a 256-bit intermediate.
pub(crate) fn mul_div_ceil(a: u128, b: u128, denominator: u128) -> Option<u128> {
    let floor = mul_div_floor(a, b, denominator)?;
    let (floor_high, floor_low) = mul_full(floor, denominator);
    let (product_high, product_low) = mul_full(a, b);
    if floor_high == product_high && floor_low == product_low {
        Some(floor)
    } else {
        floor.checked_add(1)
    }
}

/// `a * b / 2^128`, rounded DOWN. Dividing by exactly `2^128` (rather than an
/// arbitrary `u128` denominator) is just the high limb of the 256-bit product,
/// with no long division needed -- used by a double-Q64.64 liquidity value
/// (liquidity itself carrying the same 2^64 scale as the sqrt price it is
/// multiplied against, so the combined scale is 2^128, which does not fit in a
/// `u128` denominator at all).
pub(crate) fn mul_shr128_floor(a: u128, b: u128) -> u128 {
    mul_full(a, b).0
}

/// `a * b / 2^128`, rounded UP.
pub(crate) fn mul_shr128_ceil(a: u128, b: u128) -> u128 {
    let (high, low) = mul_full(a, b);
    if low == 0 {
        high
    } else {
        high.saturating_add(1)
    }
}

/// `(numerator << 128) / denominator`, rounded DOWN, i.e. `numerator * 2^128 /
/// denominator` without ever materialising `2^128` itself (which does not fit
/// in a `u128`). `numerator << 128` is exactly the 256-bit value whose high
/// limb is `numerator` and whose low limb is zero.
pub(crate) fn shl128_div_floor(numerator: u128, denominator: u128) -> Option<u128> {
    div_full(numerator, 0, denominator)
}
