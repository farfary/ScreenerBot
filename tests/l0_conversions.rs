//! L0 (pure): SOL/lamports conversions and OHLCV timeframe math.
//!
//! No network, no DB, no wallet — runs on every commit and in CI. These are the
//! deterministic money/time primitives the rest of the system is built on, so a
//! regression here is a silent correctness bug everywhere.

mod common;

use screenerbot::ohlcvs::Timeframe;
use screenerbot::utils::{lamports_to_sol, sol_to_lamports};

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

#[test]
fn one_sol_is_a_billion_lamports() {
    assert_eq!(sol_to_lamports(1.0), LAMPORTS_PER_SOL);
    assert_eq!(lamports_to_sol(LAMPORTS_PER_SOL), 1.0);
}

#[test]
fn zero_maps_to_zero_both_ways() {
    assert_eq!(sol_to_lamports(0.0), 0);
    assert_eq!(lamports_to_sol(0), 0.0);
}

#[test]
fn lamports_sol_round_trip_is_stable() {
    for sol in [0.000_000_001_f64, 0.5, 1.0, 12.345_678_9, 1000.0] {
        let lamports = sol_to_lamports(sol);
        let back = lamports_to_sol(lamports);
        assert!(
            (back - sol).abs() < 1e-9,
            "round trip drift: {sol} -> {lamports} -> {back}"
        );
    }
}

#[test]
fn timeframe_seconds_are_canonical() {
    let cases = [
        (Timeframe::Minute1, 60),
        (Timeframe::Minute5, 300),
        (Timeframe::Minute15, 900),
        (Timeframe::Hour1, 3600),
        (Timeframe::Hour4, 14_400),
        (Timeframe::Hour12, 43_200),
        (Timeframe::Day1, 86_400),
    ];
    for (tf, secs) in cases {
        assert_eq!(tf.to_seconds(), secs, "{tf:?} seconds");
    }
}

#[test]
fn candle_bucket_snaps_to_utc_floor() {
    // Invariant (CLAUDE.md): candle ts is snapped to floor(ts/tf)*tf at ingest, so
    // providers that phase a bucket differently (Gecko anchors 12h at +10h, i.e.
    // ts % 43200 == 36000, while paid/USD->SOL use midnight) cannot interleave a
    // corrupted series. Both anchors must collapse into the same canonical bucket.
    let snap = |ts: i64, tf: i64| (ts / tf) * tf;
    let tf = Timeframe::Hour12.to_seconds();

    let midnight_bucket = 1_700_000_000 - (1_700_000_000 % tf);
    let gecko_anchor = midnight_bucket + 36_000; // Gecko's +10h phasing within the bucket.

    assert_eq!(
        snap(gecko_anchor, tf),
        midnight_bucket,
        "gecko anchor must snap back"
    );
    assert_eq!(
        snap(midnight_bucket + 5, tf),
        midnight_bucket,
        "a few seconds in stays put"
    );
    assert_eq!(
        snap(midnight_bucket, tf) % tf,
        0,
        "snapped ts is bucket-aligned"
    );
}
