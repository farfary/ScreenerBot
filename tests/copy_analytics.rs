//! Pure Phase 5 copy analytics and latency policy contracts.

use screenerbot::trader::copy::{
    latency_should_pause, proportional_exit_percentage, summarize_arrival_distances,
};

#[test]
fn arrival_distribution_is_deterministic() {
    let stats = summarize_arrival_distances(vec![4_000, 1_000, 2_000, 3_000, 10_000]);
    assert_eq!(stats.samples, 5);
    assert_eq!(stats.minimum_ms, Some(1_000));
    assert_eq!(stats.median_ms, Some(3_000));
    assert_eq!(stats.p95_ms, Some(10_000));
    assert_eq!(stats.maximum_ms, Some(10_000));
    assert_eq!(stats.average_ms, Some(4_000));
}

#[test]
fn latency_kill_switch_requires_a_full_trailing_window() {
    assert!(!latency_should_pause(&[9_000, 9_000], 3, 4_000));
    assert!(!latency_should_pause(&[9_000, 1_000, 1_000], 3, 4_000));
    assert!(latency_should_pause(&[1_000, 5_000, 7_000], 3, 4_000));
    assert!(latency_should_pause(
        &[100, 100, 9_000, 9_000, 9_000],
        3,
        4_000
    ));
}

#[test]
fn proportional_sell_uses_target_inventory_and_fails_closed() {
    assert_eq!(proportional_exit_percentage(30.0, 100.0), Some(30.0));
    assert_eq!(proportional_exit_percentage(150.0, 100.0), Some(100.0));
    assert_eq!(proportional_exit_percentage(30.0, 0.0), None);
}
