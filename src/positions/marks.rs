// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The mark of a position: its current price, and the peaks and trough that price moves.

use chrono::{DateTime, Utc};

use super::types::{Position, PriceSource};

/// Marks `position` at `price` from `source` at `now`: the current price, the round's
/// running high and low, and the basis peak the trailing stop measures from. A low of
/// zero is an unset low and takes the price. The caller validates the price; the live
/// and the paper book both mark through here.
pub fn apply_mark(position: &mut Position, price: f64, source: PriceSource, now: DateTime<Utc>) {
    position.current_price = Some(price);
    position.current_price_updated = Some(now);
    position.current_price_source = Some(source);

    if price > position.price_highest {
        position.price_highest = price;
    }
    if price > position.basis_peak_price {
        position.basis_peak_price = price;
    }
    if price < position.price_lowest || position.price_lowest == 0.0 {
        position.price_lowest = price;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::positions::types::{Book, PositionManagement, PositionOrigin};
    use crate::positions::Opening;

    fn position(entry: f64) -> Position {
        Position::opening(Opening {
            mint: "Mint1111111111111111111111111111111111111111".to_owned(),
            symbol: "MINT".to_owned(),
            name: "Mint".to_owned(),
            entry_price: entry,
            entry_time: Utc::now(),
            size_native: 0.1,
            entry_signature: None,
            origin: PositionOrigin::Manual,
            management: PositionManagement::AutoTrader,
            entry_liquidity: None,
            book: Book::Live,
        })
    }

    /// The mark the live updater applied before the owner was extracted, kept as the
    /// reference the extracted owner is measured against.
    fn reference_mark(pos: &mut Position, price: f64, source: PriceSource, now: DateTime<Utc>) {
        pos.current_price = Some(price);
        pos.current_price_updated = Some(now);
        pos.current_price_source = Some(source);
        if price > pos.price_highest {
            pos.price_highest = price;
        }
        if price > pos.basis_peak_price {
            pos.basis_peak_price = price;
        }
        if price < pos.price_lowest || pos.price_lowest == 0.0 {
            pos.price_lowest = price;
        }
    }

    #[test]
    fn marks_match_the_live_updater_over_a_price_path() {
        let now = Utc::now();
        let path = [1.0, 1.4, 0.7, 0.7, 2.0, 1.9, 0.5, 3.0];
        for (entry, unset_low) in [(1.0, false), (1.0, true), (0.2, false)] {
            let mut marked = position(entry);
            let mut reference = position(entry);
            if unset_low {
                marked.price_lowest = 0.0;
                reference.price_lowest = 0.0;
            }
            for (step, price) in path.into_iter().enumerate() {
                let at = now + chrono::Duration::seconds(step as i64);
                let source = if step % 2 == 0 {
                    PriceSource::Pool
                } else {
                    PriceSource::Api
                };
                apply_mark(&mut marked, price, source, at);
                reference_mark(&mut reference, price, source, at);
                assert_eq!(marked.current_price, reference.current_price);
                assert_eq!(
                    marked.current_price_updated,
                    reference.current_price_updated
                );
                assert_eq!(marked.current_price_source, reference.current_price_source);
                assert_eq!(marked.price_highest, reference.price_highest);
                assert_eq!(marked.price_lowest, reference.price_lowest);
                assert_eq!(marked.basis_peak_price, reference.basis_peak_price);
            }
        }
    }

    #[test]
    fn a_mark_raises_the_peaks_and_lowers_the_trough_only() {
        let mut marked = position(1.0);
        marked.basis_peak_price = 1.2;
        apply_mark(&mut marked, 1.1, PriceSource::Pool, Utc::now());
        assert_eq!(marked.price_highest, 1.1);
        assert_eq!(marked.basis_peak_price, 1.2);
        assert_eq!(marked.price_lowest, 1.0);
        apply_mark(&mut marked, 0.9, PriceSource::Pool, Utc::now());
        assert_eq!(marked.price_highest, 1.1);
        assert_eq!(marked.price_lowest, 0.9);
    }
}
