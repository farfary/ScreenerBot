//! Numeric bound checks shared by the market-data filter sources.
//!
//! Every market rule is one of exactly two shapes, and the two answer a missing reading
//! differently — deliberately, and for a reason that is easy to state:
//!
//! - A **range** (a minimum AND a maximum) asks "is this value inside the band?". A value we
//!   do not have cannot be shown to fall outside it, so an absent reading passes.
//! - A **floor** (a minimum only) asks "does this token clear the bar?". That is a
//!   requirement the token has to demonstrate, and an unknown value demonstrates nothing, so
//!   an absent reading fails — unless the floor is zero or less, which constrains nothing.
//!
//! Before either shape was uniform, the same concept answered differently per field: an
//! absent liquidity or market cap passed while an absent FDV rejected, and a price change
//! rejected on absence even when its band accepted every possible value.
//!
//! Both shapes funnel through [`reading`], which is what keeps a corrupt number from
//! passing: NaN compares false against `<` and `>` alike, so a NaN liquidity used to clear
//! its minimum and its maximum simultaneously, on every source, for every field.

/// Normalise a provider reading. A non-finite number carries no information — NaN satisfies
/// every bound at once and an infinity is a parse artefact, not a measurement — so both are
/// reported as absent rather than compared.
pub(super) fn reading(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite())
}

/// Verdict of a [`check_range`] test.
pub(super) enum Range {
    Ok,
    TooLow,
    TooHigh,
}

/// Verdict of a [`check_floor`] test.
pub(super) enum Floor {
    Ok,
    TooLow,
    Missing,
}

/// Range check: `min <= value <= max`, both bounds inclusive. An absent or unusable reading
/// passes.
///
/// `max` is an `Option` on purpose. Some GeckoTerminal ceilings use `0` to mean "no
/// ceiling", and folding that convention into this function would silently apply it to
/// fields where zero is a real limit a user can choose — a maximum price change of 0%
/// ("only tokens that did not rise") is a legitimate setting, not an unbounded one. Callers
/// that want the convention opt into it where the convention actually holds.
pub(super) fn check_range(value: Option<f64>, min: f64, max: Option<f64>) -> Range {
    let Some(value) = reading(value) else {
        return Range::Ok;
    };

    if value < min {
        return Range::TooLow;
    }

    if max.is_some_and(|max| value > max) {
        return Range::TooHigh;
    }

    Range::Ok
}

/// A ceiling that uses `0` (or less) to mean "unbounded".
pub(super) fn optional_ceiling(max: f64) -> Option<f64> {
    (max > 0.0).then_some(max)
}

/// Floor check: `value >= min`. An absent or unusable reading fails a positive floor; a
/// floor of zero or less is inert and passes everything.
pub(super) fn check_floor(value: Option<f64>, min: f64) -> Floor {
    if min <= 0.0 {
        return Floor::Ok;
    }

    match reading(value) {
        Some(value) if value < min => Floor::TooLow,
        Some(_) => Floor::Ok,
        None => Floor::Missing,
    }
}
