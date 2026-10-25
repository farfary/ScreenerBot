// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The native-coin amount a buy needs in the signing wallet, summed the same way for every chain.

/// Native units a buy needs: the input, the network fee and the rent of every
/// account the swap opens. Saturates instead of wrapping.
pub(crate) fn native_needed(
    input_raw: u64,
    network_fee_raw: u64,
    account_rent_raw: u64,
    accounts_to_open: u64,
) -> u64 {
    input_raw
        .saturating_add(network_fee_raw)
        .saturating_add(account_rent_raw.saturating_mul(accounts_to_open))
}

#[cfg(test)]
mod tests {
    use super::native_needed;

    #[test]
    fn needed_is_the_input_plus_the_fee_plus_the_rent_of_every_opened_account() {
        assert_eq!(native_needed(995_000_000, 5_000, 2_039_280, 2), 999_083_560);
        assert_eq!(native_needed(995_000_000, 5_000, 2_039_280, 1), 997_044_280);
    }

    #[test]
    fn needed_without_opened_accounts_ignores_the_rent() {
        assert_eq!(native_needed(10, 3, 2_039_280, 0), 13);
    }

    #[test]
    fn needed_saturates_instead_of_wrapping() {
        assert_eq!(native_needed(u64::MAX, 1, 1, 1), u64::MAX);
        assert_eq!(native_needed(0, 0, u64::MAX, 2), u64::MAX);
    }
}
