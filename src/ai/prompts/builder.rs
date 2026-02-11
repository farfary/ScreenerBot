use crate::ai::db::{list_instructions, with_ai_db};
use crate::ai::types::EvaluationContext;

/// Maximum total length for all user instructions combined
const MAX_INSTRUCTION_CONTENT_LENGTH: usize = 5000;

/// Dynamic prompt builder that formats token data into prompts
pub struct PromptBuilder;

impl PromptBuilder {
    /// Build a user prompt from evaluation context
    pub fn build_user_prompt(context: &EvaluationContext) -> String {
        let mut prompt = format!("Token: {}\n\n", context.mint);

        // Add user instructions at the beginning
        if let Some(instructions) = Self::get_user_instructions() {
            if !instructions.is_empty() {
                prompt.push_str("=== User Instructions ===\n");
                prompt.push_str(&instructions);
                prompt.push_str("\n\n");
            }
        }

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

    /// Get enabled user instructions sorted by priority
    fn get_user_instructions() -> Option<String> {
        with_ai_db(|db| match list_instructions(db) {
            Ok(instructions) => {
                let enabled: Vec<_> = instructions.into_iter().filter(|i| i.enabled).collect();
                if enabled.is_empty() {
                    return Ok(None);
                }

                let formatted: Vec<String> = enabled
                    .iter()
                    .map(|i| {
                        // Sanitize content: strip lines starting with "SYSTEM:" or "==="
                        let sanitized = i.content
                            .lines()
                            .filter(|line| {
                                let trimmed = line.trim();
                                !trimmed.starts_with("SYSTEM:") && !trimmed.starts_with("===")
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        format!("[{}] {}: {}", i.category.to_uppercase(), i.name, sanitized)
                    })
                    .collect();

                let mut combined = formatted.join("\n\n");
                
                // Limit total instruction content length
                if combined.len() > MAX_INSTRUCTION_CONTENT_LENGTH {
                    combined.truncate(MAX_INSTRUCTION_CONTENT_LENGTH);
                    combined.push_str("\n\n[WARNING: Instructions truncated due to length limit]");
                }

                Ok(Some(combined))
            }
            Err(_) => Ok(None),
        })
        .ok()
        .flatten()
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
