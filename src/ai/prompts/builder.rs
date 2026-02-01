use crate::ai::types::EvaluationContext;

/// Dynamic prompt builder that formats token data into prompts
pub struct PromptBuilder;

impl PromptBuilder {
    /// Build a user prompt from evaluation context
    pub fn build_user_prompt(context: &EvaluationContext) -> String {
        let mut prompt = format!("Token: {}\n\n", context.mint);

        // Add DexScreener data
        if let Some(ref data) = context.dexscreener_data {
            prompt.push_str("=== DexScreener Data ===\n");
            prompt.push_str(&serde_json::to_string_pretty(data).unwrap_or_default());
            prompt.push_str("\n\n");
        }

        // Add GeckoTerminal data
        if let Some(ref data) = context.geckoterminal_data {
            prompt.push_str("=== GeckoTerminal Data ===\n");
            prompt.push_str(&serde_json::to_string_pretty(data).unwrap_or_default());
            prompt.push_str("\n\n");
        }

        // Add RugCheck data
        if let Some(ref data) = context.rugcheck_data {
            prompt.push_str("=== RugCheck Data ===\n");
            prompt.push_str(&serde_json::to_string_pretty(data).unwrap_or_default());
            prompt.push_str("\n\n");
        }

        // Add pool data
        if let Some(ref data) = context.pool_data {
            prompt.push_str("=== Pool Data ===\n");
            prompt.push_str(&serde_json::to_string_pretty(data).unwrap_or_default());
            prompt.push_str("\n\n");
        }

        // Add opening snapshot
        if let Some(ref data) = context.opening_snapshot {
            prompt.push_str("=== Opening Snapshot ===\n");
            prompt.push_str(&serde_json::to_string_pretty(data).unwrap_or_default());
            prompt.push_str("\n\n");
        }

        // Add price history
        if let Some(ref history) = context.price_history {
            prompt.push_str("=== Price History ===\n");
            prompt.push_str(&format!("{:?}", history));
            prompt.push_str("\n\n");
        }

        prompt.push_str("Analyze this data and provide your decision.");
        prompt
    }

    /// Build a compact user prompt with minimal data
    pub fn build_compact_prompt(mint: &str, key_data: &serde_json::Value) -> String {
        format!(
            "Token: {}\n\nKey Data:\n{}\n\nProvide your analysis.",
            mint,
            serde_json::to_string_pretty(key_data).unwrap_or_default()
        )
    }
}
