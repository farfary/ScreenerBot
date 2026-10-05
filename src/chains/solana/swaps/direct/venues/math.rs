// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Curve arithmetic shared by the constant-product venues.
//!
//! Everything here is integer math in `u128` and rounds in the direction that
//! cannot invent value:
//!
//! * an OUTPUT rounds down — the pool never owes more than it computed;
//! * a FEE rounds up — the pool is never short-changed.
//!
//! Rounding an output up by even one unit produces a `min_out` the pool cannot
//! satisfy, and the transaction reverts after the wrap and the ATA creations have
//! already run.

/// Ceiling division for fee amounts.
pub fn ceil_div(numerator: u128, denominator: u128) -> u128 {
    if denominator == 0 {
        return 0;
    }
    numerator.div_ceil(denominator)
}

/// Fee taken off an amount at `rate` per `denominator`, rounded UP.
pub fn fee_amount(amount: u64, rate: u64, denominator: u64) -> u64 {
    if rate == 0 || denominator == 0 {
        return 0;
    }
    ceil_div((amount as u128) * (rate as u128), denominator as u128).min(amount as u128) as u64
}

/// Constant-product output: `out = reserve_out * amount_in / (reserve_in + amount_in)`,
/// rounded DOWN.
///
/// Returns 0 when either reserve is empty — an empty pool cannot fill anything,
/// and the caller turns that into a liquidity failure rather than a zero quote.
pub fn constant_product_out(reserve_in: u64, reserve_out: u64, amount_in: u64) -> u64 {
    if reserve_in == 0 || reserve_out == 0 || amount_in == 0 {
        return 0;
    }
    let numerator = (reserve_out as u128) * (amount_in as u128);
    let denominator = (reserve_in as u128) + (amount_in as u128);
    let out = numerator / denominator;
    // A constant-product pool can never give out its whole reserve, but clamp
    // anyway: a quote above the reserve would be unfillable by construction.
    out.min((reserve_out as u128).saturating_sub(1)) as u64
}

/// Price impact of a fill, as a percentage.
///
/// Compares the realised rate against the pool's marginal rate before the trade.
/// Decimals cancel out because both rates are in the same raw units, so this is
/// safe to compute without knowing either mint's decimals.
pub fn price_impact_pct(reserve_in: u64, reserve_out: u64, amount_in: u64, amount_out: u64) -> f64 {
    if reserve_in == 0 || reserve_out == 0 || amount_in == 0 || amount_out == 0 {
        return 0.0;
    }
    let spot = (reserve_out as f64) / (reserve_in as f64);
    let realised = (amount_out as f64) / (amount_in as f64);
    if spot <= 0.0 {
        return 0.0;
    }
    ((spot - realised) / spot * 100.0).clamp(0.0, 100.0)
}

pub(crate) use crate::chains::amount::math::{
    mul_div_ceil, mul_div_floor, mul_shr128_ceil, mul_shr128_floor, shl128_div_floor,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::amount::math::mul_full;

    #[test]
    fn mul_shr128_of_operands_below_the_scale_is_zero() {
        // 5 * 3 = 15, far short of 2^128, so shifting right 128 gives 0.
        assert_eq!(mul_shr128_floor(5, 3), 0);
        assert_eq!(mul_shr128_ceil(5, 3), 1, "any non-zero remainder rounds up");
    }

    #[test]
    fn shl128_div_recovers_a_plain_ratio_when_it_divides_evenly() {
        // The quotient must itself fit in a u128, so the denominator has to be
        // large relative to the numerator -- exactly DBC's real shape, where
        // the numerator is a raw swap amount (well under 2^64) and the
        // denominator is a double-Q64.64 liquidity value (over 2^100).
        let denominator = 1u128 << 100;
        let numerator = 5u128;
        let floor = shl128_div_floor(numerator, denominator).expect("fits");
        assert_eq!(floor, numerator << 28, "(5 << 128) / 2^100 == 5 << 28");
    }

    #[test]
    fn mul_shr128_ceil_matches_floor_on_an_exact_product() {
        // 2^64 * 2^64 == 2^128 exactly, so shifting right 128 loses nothing.
        let a = 1u128 << 64;
        let b = 1u128 << 64;
        assert_eq!(mul_shr128_floor(a, b), mul_shr128_ceil(a, b));
        assert_eq!(mul_shr128_floor(a, b), 1);
    }

    #[test]
    fn a_fee_always_rounds_up_so_the_pool_is_never_short() {
        // 0.25% of 1000 = 2.5 -> 3
        assert_eq!(fee_amount(1_000, 2_500, 1_000_000), 3);
        assert_eq!(fee_amount(1_000_000, 2_500, 1_000_000), 2_500);
        assert_eq!(fee_amount(1, 2_500, 1_000_000), 1, "dust still pays a unit");
    }

    #[test]
    fn a_zero_rate_or_denominator_charges_nothing_instead_of_dividing_by_zero() {
        assert_eq!(fee_amount(1_000, 0, 1_000_000), 0);
        assert_eq!(fee_amount(1_000, 2_500, 0), 0);
        assert_eq!(ceil_div(10, 0), 0);
    }

    #[test]
    fn a_fee_can_never_exceed_the_amount_it_is_taken_from() {
        assert_eq!(fee_amount(100, 2_000_000, 1_000_000), 100);
    }

    #[test]
    fn constant_product_matches_the_textbook_result_and_rounds_down() {
        // 1000 in against 1_000_000 / 1_000_000: 1_000_000*1000/1_001_000 = 999.0
        assert_eq!(constant_product_out(1_000_000, 1_000_000, 1_000), 999);
    }

    #[test]
    fn an_empty_pool_fills_nothing_rather_than_panicking() {
        assert_eq!(constant_product_out(0, 1_000_000, 1_000), 0);
        assert_eq!(constant_product_out(1_000_000, 0, 1_000), 0);
        assert_eq!(constant_product_out(1_000_000, 1_000_000, 0), 0);
    }

    #[test]
    fn a_huge_fill_never_drains_the_whole_reserve_and_never_overflows() {
        let out = constant_product_out(1_000, u64::MAX, u64::MAX);
        assert!(out < u64::MAX, "the pool always keeps at least one unit");
        assert!(out > 0);
    }

    #[test]
    fn price_impact_grows_with_size_and_is_near_zero_for_dust() {
        let dust = price_impact_pct(
            1_000_000_000,
            1_000_000_000,
            1_000,
            constant_product_out(1_000_000_000, 1_000_000_000, 1_000),
        );
        let whale = price_impact_pct(
            1_000_000_000,
            1_000_000_000,
            500_000_000,
            constant_product_out(1_000_000_000, 1_000_000_000, 500_000_000),
        );
        assert!(dust < 0.5, "dust barely moves the pool, got {dust}");
        assert!(
            whale > 30.0,
            "half the reserve is a huge impact, got {whale}"
        );
    }

    #[test]
    fn price_impact_of_an_empty_or_zero_fill_is_zero_not_nan() {
        assert_eq!(price_impact_pct(0, 1, 1, 1), 0.0);
        assert_eq!(price_impact_pct(1, 1, 0, 0), 0.0);
    }

    #[test]
    fn a_product_that_overflows_u128_still_divides_correctly() {
        // The product of two 2^127 values needs 254 bits.
        let big = 1u128 << 127;
        assert_eq!(
            mul_div_floor(big, big, 1u128 << 126),
            None,
            "quotient overflows"
        );
        assert_eq!(mul_div_floor(big, big, big), Some(big));
    }

    #[test]
    fn mul_div_matches_plain_arithmetic_when_nothing_overflows() {
        assert_eq!(mul_div_floor(7, 5, 2), Some(17), "35/2 floors to 17");
        assert_eq!(mul_div_ceil(7, 5, 2), Some(18), "35/2 ceils to 18");
        assert_eq!(mul_div_floor(10, 10, 5), Some(20));
        assert_eq!(
            mul_div_ceil(10, 10, 5),
            Some(20),
            "an exact result never rounds up"
        );
    }

    #[test]
    fn full_width_ratios_preserve_exact_limits_and_checked_ceil() {
        let max = u128::MAX;
        assert_eq!(mul_div_floor(max, 2, 2), Some(max));
        assert_eq!(mul_div_ceil(max, 2, 2), Some(max));
        assert_eq!(mul_div_floor(max, max, max), Some(max));
        assert_eq!(mul_div_ceil(max, max, max), Some(max));
        assert_eq!(mul_div_floor(max, max, 1), None);
        assert_eq!(mul_div_ceil(max, max, 1), None);
        assert_eq!(mul_div_floor(0, max, 1), Some(0));
        assert_eq!(mul_div_ceil(0, max, 1), Some(0));
        assert_eq!(mul_div_floor(max, max, 0), None);
        assert_eq!(mul_div_ceil(max, max, 0), None);
        assert_eq!(mul_div_floor(7, 3, 2), Some(10));
        assert_eq!(mul_div_ceil(7, 3, 2), Some(11));
    }

    #[test]
    fn dividing_by_zero_is_none_rather_than_a_panic() {
        assert_eq!(mul_div_floor(1, 1, 0), None);
        assert_eq!(mul_div_ceil(1, 1, 0), None);
    }

    #[test]
    fn a_quotient_that_cannot_fit_is_refused_rather_than_wrapped() {
        assert_eq!(mul_div_floor(u128::MAX, u128::MAX, 1), None);
        assert_eq!(mul_div_floor(u128::MAX, 2, 1), None);
    }

    #[test]
    fn the_full_product_agrees_with_u128_where_both_can_represent_it() {
        for (a, b) in [
            (0u128, 0u128),
            (1, 1),
            (u64::MAX as u128, 3),
            (12_345_678_901_234_567_890, 987_654_321),
        ] {
            let (high, low) = mul_full(a, b);
            assert_eq!(high, 0, "these products fit in 128 bits");
            assert_eq!(low, a * b);
        }
    }

    #[test]
    fn a_realistic_concentrated_liquidity_step_does_not_overflow() {
        // Live SOL/USDC CLMM values: L ~ 1.4e14, sqrt_price_x64 ~ 5.7e18.
        let liquidity = 139_124_859_123_528u128;
        let sqrt_price = 5_718_259_629_277_169_978u128;
        let numerator = liquidity << 64;
        let next = mul_div_floor(numerator, sqrt_price, numerator + sqrt_price * 5_000_000)
            .expect("a real step must compute");
        assert!(
            next > 0 && next < sqrt_price,
            "selling token_0 moves the price down"
        );
    }
}
