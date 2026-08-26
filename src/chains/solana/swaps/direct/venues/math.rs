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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(dust < 0.01, "dust barely moves the pool, got {dust}");
        assert!(whale > 30.0, "half the reserve is a huge impact, got {whale}");
    }

    #[test]
    fn price_impact_of_an_empty_or_zero_fill_is_zero_not_nan() {
        assert_eq!(price_impact_pct(0, 1, 1, 1), 0.0);
        assert_eq!(price_impact_pct(1, 1, 0, 0), 0.0);
    }
}
