// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The risk band of RugCheck's normalised score, the one place its bands are drawn.
//!
//! The normalised score runs 0-100 and HIGHER IS RISKIER. Every surface that
//! grades a token (featured cards, token details, position details, agent tools)
//! receives the band from here and maps it to its own label; none compares the
//! number itself, because each comparison written elsewhere drifted to its own
//! thresholds and one read the scale backwards.

use serde::Serialize;

/// A token's risk band derived from RugCheck's normalised score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityRiskBand {
    Low,
    Medium,
    High,
}

impl SecurityRiskBand {
    /// Highest normalised score still in the low band.
    pub const LOW_MAX: i32 = 30;
    /// Highest normalised score still in the medium band.
    pub const MEDIUM_MAX: i32 = 60;

    /// The band of a normalised score (0-100, higher = riskier).
    pub fn of_normalised(score: i32) -> Self {
        if score <= Self::LOW_MAX {
            Self::Low
        } else if score <= Self::MEDIUM_MAX {
            Self::Medium
        } else {
            Self::High
        }
    }

    /// The band of an optional normalised score; `None` while the token has no report.
    pub fn of_optional(score: Option<i32>) -> Option<Self> {
        score.map(Self::of_normalised)
    }
}

#[cfg(test)]
mod tests {
    use super::SecurityRiskBand;

    #[test]
    fn both_ends_of_the_scale_land_in_the_outer_bands() {
        assert_eq!(SecurityRiskBand::of_normalised(0), SecurityRiskBand::Low);
        assert_eq!(SecurityRiskBand::of_normalised(100), SecurityRiskBand::High);
    }

    #[test]
    fn each_band_edge_belongs_to_the_lower_band() {
        let low_max = SecurityRiskBand::LOW_MAX;
        let medium_max = SecurityRiskBand::MEDIUM_MAX;
        assert_eq!(
            SecurityRiskBand::of_normalised(low_max),
            SecurityRiskBand::Low
        );
        assert_eq!(
            SecurityRiskBand::of_normalised(low_max + 1),
            SecurityRiskBand::Medium
        );
        assert_eq!(
            SecurityRiskBand::of_normalised(medium_max),
            SecurityRiskBand::Medium
        );
        assert_eq!(
            SecurityRiskBand::of_normalised(medium_max + 1),
            SecurityRiskBand::High
        );
    }

    #[test]
    fn a_missing_score_has_no_band() {
        assert_eq!(SecurityRiskBand::of_optional(None), None);
        assert_eq!(
            SecurityRiskBand::of_optional(Some(5)),
            Some(SecurityRiskBand::Low)
        );
    }

    #[test]
    fn bands_serialize_as_the_ids_the_dashboard_maps() {
        let ids: Vec<String> = [
            SecurityRiskBand::Low,
            SecurityRiskBand::Medium,
            SecurityRiskBand::High,
        ]
        .iter()
        .map(|band| serde_json::to_string(band).unwrap())
        .collect();
        assert_eq!(ids, ["\"low\"", "\"medium\"", "\"high\""]);
    }
}
