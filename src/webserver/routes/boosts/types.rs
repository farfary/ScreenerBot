//! Boost feed types — one boosted mint, and the website envelope carrying them.

use serde::{Deserialize, Serialize};

/// A mint's live boost standing, exactly as `screenerbot.io/api/boost` reports it.
///
/// A boost is a confirmed payment whose window has not expired; `boosts` is the SUM
/// of those active payments, and `golden` is true once that sum reaches the
/// website's admin-set Golden threshold. The app never computes either — the
/// website owns the ledger, we only read the standing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoostStanding {
    pub mint: String,
    #[serde(default)]
    pub boosts: u32,
    #[serde(default)]
    pub golden: bool,
}

/// The website's public boost feed envelope.
#[derive(Debug, Deserialize)]
pub(super) struct WebsiteBoostResponse {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub tokens: Vec<BoostStanding>,
}

/// Order a boost feed for display: Golden first, then most-boosted, then mint.
///
/// The website already sorts by active boosts, but every surface that pins boosted
/// tokens to the top (token tables, the featured row, the featured dialog) must
/// agree on ONE order, including the tie-break — otherwise the same two tokens swap
/// places between surfaces on every poll. Mint is the stable final key.
pub fn rank_boosts(standings: &mut [BoostStanding]) {
    standings.sort_by(|a, b| {
        b.golden
            .cmp(&a.golden)
            .then(b.boosts.cmp(&a.boosts))
            .then_with(|| a.mint.cmp(&b.mint))
    });
}

/// Drop rows the feed should never have contained: no mint, or no active boost.
///
/// A zero-boost row is not "a token with no boost" — it is a token that should not
/// be in a PAID feed at all, and letting one through would gold-mark an organic
/// token in every table.
pub fn retain_active(standings: &mut Vec<BoostStanding>) {
    standings.retain(|s| !s.mint.trim().is_empty() && s.boosts > 0);
    standings.dedup_by(|a, b| a.mint == b.mint);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standing(mint: &str, boosts: u32, golden: bool) -> BoostStanding {
        BoostStanding {
            mint: mint.to_owned(),
            boosts,
            golden,
        }
    }

    #[test]
    fn golden_outranks_a_larger_plain_boost() {
        let mut feed = vec![standing("b", 900, false), standing("a", 500, true)];
        rank_boosts(&mut feed);
        assert_eq!(feed[0].mint, "a");
    }

    #[test]
    fn equal_standing_breaks_on_mint_so_the_order_is_stable() {
        let mut feed = vec![standing("z", 10, false), standing("a", 10, false)];
        rank_boosts(&mut feed);
        assert_eq!(
            feed.iter().map(|s| s.mint.as_str()).collect::<Vec<_>>(),
            ["a", "z"]
        );
    }

    #[test]
    fn inactive_and_blank_rows_are_dropped() {
        let mut feed = vec![
            standing("a", 0, false),
            standing("  ", 5, false),
            standing("b", 5, false),
        ];
        retain_active(&mut feed);
        assert_eq!(feed, vec![standing("b", 5, false)]);
    }

    #[test]
    fn duplicate_mints_collapse_after_ranking() {
        let mut feed = vec![standing("a", 5, false), standing("a", 50, false)];
        rank_boosts(&mut feed);
        retain_active(&mut feed);
        assert_eq!(feed, vec![standing("a", 50, false)]);
    }

    #[test]
    fn a_feed_row_parses_with_only_a_mint() {
        let parsed: BoostStanding = serde_json::from_str(r#"{"mint":"a"}"#).unwrap();
        assert_eq!(parsed, standing("a", 0, false));
    }
}
