//! Pure: SOL <-> lamports conversions — the core money primitive. No I/O.
//!
//! A regression here is a silent correctness bug in every trade size and P&L figure.

mod common;

use screenerbot::chains::solana::constants::{lamports_to_sol, sol_to_lamports};

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
