// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Impact level of a config field. Serialized as the id its catalog key is
//! built from (`config-impact-<id>`).

use serde::Serialize;

/// Impact level of a config field.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum ConfigImpact {
    Critical,
    High,
    Medium,
    Low,
}

impl ConfigImpact {
    pub const ALL: [ConfigImpact; 4] = [
        ConfigImpact::Critical,
        ConfigImpact::High,
        ConfigImpact::Medium,
        ConfigImpact::Low,
    ];

    /// Id used in serialized metadata and in the catalog key.
    pub const fn as_str(self) -> &'static str {
        match self {
            ConfigImpact::Critical => "critical",
            ConfigImpact::High => "high",
            ConfigImpact::Medium => "medium",
            ConfigImpact::Low => "low",
        }
    }
}

/// Impact catalog key (`config-impact-<id>`).
pub(crate) fn impact_key(impact: ConfigImpact) -> String {
    format!("config-impact-{}", impact.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impact_ids_serialize_as_their_key_ids() {
        for impact in ConfigImpact::ALL {
            let serialized = serde_json::to_value(impact).unwrap();
            assert_eq!(
                serialized,
                serde_json::Value::String(impact.as_str().to_string())
            );
        }
    }
}
