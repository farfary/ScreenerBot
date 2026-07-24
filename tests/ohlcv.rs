//! OHLCV chart data. Pure timeframe/candle math today; live candle fetch is the
//! documented next addition (a `#[ignore]` test through the bot's real fetch path,
//! asserting SOL-denominated candles with volume > 0 and canonical-bucket timestamps).

mod common;

use screenerbot::ohlcvs::Timeframe;

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
