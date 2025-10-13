/// Dynamic Token Pattern Detection and Categorization System
///
/// This module automatically detects and categorizes tokens based on patterns in:
/// - Token names (prefixes/suffixes)
/// - Mint addresses (platform signatures)
/// - Symbol patterns
///
/// The system dynamically identifies patterns without hardcoding, providing insights
/// into token creation platforms and ecosystem relationships.
use crate::logger::{log, LogTag};
use crate::tokens::store::get_global_token_store;
use crate::tokens::Token;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Global pattern analyzer instance
static PATTERN_ANALYZER: Lazy<Arc<Mutex<Option<TokenPatternAnalyzer>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Minimum occurrence count for a pattern to be considered significant
const MIN_PATTERN_OCCURRENCES: usize = 3;

/// Maximum length for pattern analysis (performance optimization)
const MAX_PATTERN_LENGTH: usize = 8;

/// Minimum pattern length to avoid noise
const MIN_PATTERN_LENGTH: usize = 3;

#[derive(Debug, Clone)]
pub struct TokenPattern {
    pub pattern: String,
    pub pattern_type: PatternType,
    pub count: usize,
    pub examples: Vec<String>, // Store mint addresses as examples
    pub confidence_score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    /// Patterns detected in mint addresses (platform signatures)
    MintSuffix,
    MintPrefix,
    /// Patterns detected in token names
    NameSuffix,
    NamePrefix,
    /// Patterns detected in token symbols
    SymbolSuffix,
    SymbolPrefix,
}

#[derive(Debug, Clone)]
pub struct PlatformCategory {
    pub platform_name: String,
    pub identification_pattern: String,
    pub pattern_type: PatternType,
    pub token_count: usize,
    pub examples: Vec<String>,
}

#[derive(Debug)]
pub struct TokenPatternAnalyzer {
    /// Detected patterns across all types
    patterns: HashMap<String, TokenPattern>,
    /// Platform categories identified from patterns
    platforms: HashMap<String, PlatformCategory>,
    /// Last analysis timestamp
    last_analysis: chrono::DateTime<chrono::Utc>,
    /// Token count at last analysis
    last_token_count: usize,
}

impl TokenPatternAnalyzer {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            platforms: HashMap::new(),
            last_analysis: chrono::Utc::now(),
            last_token_count: 0,
        }
    }

    /// Analyze all tokens in database and detect patterns
    pub async fn analyze_patterns(&mut self) -> Result<(), String> {
        let start_time = std::time::Instant::now();

        log(
            LogTag::Discovery,
            "START",
            "Starting dynamic token pattern analysis",
        );

        // Get all tokens from the in-memory cache
        let store = get_global_token_store();
        let snapshots = store.all();

        if snapshots.is_empty() {
            log(
                LogTag::Discovery,
                "WARN",
                "No tokens found for pattern analysis",
            );
            return Ok(());
        }

        let tokens: Vec<Token> = snapshots
            .into_iter()
            .map(|snapshot| snapshot.data)
            .collect();

        log(
            LogTag::Discovery,
            "INFO",
            &format!("Analyzing patterns for {} tokens", tokens.len()),
        );

        // Clear previous analysis
        self.patterns.clear();
        self.platforms.clear();

        // Collect patterns from different token fields
        let mut mint_suffixes: HashMap<String, Vec<String>> = HashMap::new();
        let mut mint_prefixes: HashMap<String, Vec<String>> = HashMap::new();
        let mut name_suffixes: HashMap<String, Vec<String>> = HashMap::new();
        let mut name_prefixes: HashMap<String, Vec<String>> = HashMap::new();
        let mut symbol_suffixes: HashMap<String, Vec<String>> = HashMap::new();
        let mut symbol_prefixes: HashMap<String, Vec<String>> = HashMap::new();

        for token in &tokens {
            // Analyze mint address patterns
            self.extract_patterns(&token.mint, &mut mint_suffixes, &mut mint_prefixes);

            // Analyze name patterns
            if !token.name.trim().is_empty() {
                self.extract_patterns(&token.name, &mut name_suffixes, &mut name_prefixes);
            }

            // Analyze symbol patterns
            if !token.symbol.trim().is_empty() {
                self.extract_patterns(&token.symbol, &mut symbol_suffixes, &mut symbol_prefixes);
            }
        }

        // Convert collected patterns to TokenPattern structs
        self.build_patterns(mint_suffixes, PatternType::MintSuffix);
        self.build_patterns(mint_prefixes, PatternType::MintPrefix);
        self.build_patterns(name_suffixes, PatternType::NameSuffix);
        self.build_patterns(name_prefixes, PatternType::NamePrefix);
        self.build_patterns(symbol_suffixes, PatternType::SymbolSuffix);
        self.build_patterns(symbol_prefixes, PatternType::SymbolPrefix);

        // Identify platform categories from patterns
        self.identify_platforms();

        // Update analysis metadata
        self.last_analysis = chrono::Utc::now();
        self.last_token_count = tokens.len();

        let elapsed = start_time.elapsed();
        log(
            LogTag::Discovery,
            "COMPLETE",
            &format!(
                "Pattern analysis complete: {} patterns, {} platforms identified in {:.2}ms",
                self.patterns.len(),
                self.platforms.len(),
                elapsed.as_millis()
            ),
        );

        Ok(())
    }

    /// Extract patterns (prefixes and suffixes) from a text field
    fn extract_patterns(
        &self,
        text: &str,
        suffixes: &mut HashMap<String, Vec<String>>,
        prefixes: &mut HashMap<String, Vec<String>>,
    ) {
        let clean_text = text.trim();
        if clean_text.chars().count() < MIN_PATTERN_LENGTH {
            return;
        }

        let chars: Vec<char> = clean_text.chars().collect();
        let char_count = chars.len();

        // Extract suffixes of different lengths
        for len in MIN_PATTERN_LENGTH..=MAX_PATTERN_LENGTH.min(char_count) {
            if len <= char_count {
                let suffix: String = chars[char_count - len..].iter().collect();
                suffixes
                    .entry(suffix)
                    .or_insert_with(Vec::new)
                    .push(text.to_string());
            }
        }

        // Extract prefixes of different lengths
        for len in MIN_PATTERN_LENGTH..=MAX_PATTERN_LENGTH.min(char_count) {
            if len <= char_count {
                let prefix: String = chars[..len].iter().collect();
                prefixes
                    .entry(prefix)
                    .or_insert_with(Vec::new)
                    .push(text.to_string());
            }
        }
    }

    /// Convert pattern collections to TokenPattern structs
    fn build_patterns(
        &mut self,
        pattern_map: HashMap<String, Vec<String>>,
        pattern_type: PatternType,
    ) {
        for (pattern, examples) in pattern_map {
            if examples.len() >= MIN_PATTERN_OCCURRENCES {
                // Calculate confidence score based on frequency and pattern characteristics
                let confidence =
                    self.calculate_confidence_score(&pattern, examples.len(), &pattern_type);

                let token_pattern = TokenPattern {
                    pattern: pattern.clone(),
                    pattern_type: pattern_type.clone(),
                    count: examples.len(),
                    examples: examples.into_iter().take(10).collect(), // Keep max 10 examples
                    confidence_score: confidence,
                };

                let key = format!("{:?}:{}", pattern_type, pattern);
                self.patterns.insert(key, token_pattern);
            }
        }
    }

    /// Calculate confidence score for a pattern
    fn calculate_confidence_score(
        &self,
        pattern: &str,
        count: usize,
        pattern_type: &PatternType,
    ) -> f64 {
        let mut score = 0.0;

        // Base score from frequency
        score += (count as f64).ln() * 10.0;

        // Bonus for mint address patterns (more reliable for platform identification)
        if matches!(
            pattern_type,
            PatternType::MintSuffix | PatternType::MintPrefix
        ) {
            score += 20.0;
        }

        // Bonus for known platform indicators
        if pattern.to_lowercase().contains("pump") {
            score += 30.0;
        } else if pattern.to_lowercase().contains("dao") {
            score += 25.0;
        } else if pattern.to_lowercase().contains("bonk") {
            score += 20.0;
        }

        // Penalty for very short patterns (more likely to be noise)
        if pattern.len() < 4 {
            score -= 10.0;
        }

        // Normalize to 0-100 scale
        score.max(0.0).min(100.0)
    }

    /// Identify platform categories from detected patterns
    fn identify_platforms(&mut self) {
        // Known platform patterns
        let platform_indicators = vec![
            ("Pump.fun", "pump", PatternType::MintSuffix),
            ("Moonshot/DAO", "daos", PatternType::MintSuffix),
            ("Bonk Ecosystem", "bonk", PatternType::MintSuffix),
            ("Jupiter Ecosystem", "jups", PatternType::MintSuffix),
            ("BAGS Platform", "BAGS", PatternType::MintSuffix),
            ("Moon Platform", "moon", PatternType::MintSuffix),
        ];

        for (platform_name, indicator, pattern_type) in platform_indicators {
            let pattern_key = format!("{:?}:{}", pattern_type, indicator);
            if let Some(pattern) = self.patterns.get(&pattern_key) {
                let platform = PlatformCategory {
                    platform_name: platform_name.to_string(),
                    identification_pattern: indicator.to_string(),
                    pattern_type: pattern_type.clone(),
                    token_count: pattern.count,
                    examples: pattern.examples.clone(),
                };
                self.platforms.insert(platform_name.to_string(), platform);
            }
        }

        // Auto-detect additional platforms from high-confidence patterns
        for (key, pattern) in &self.patterns {
            if pattern.confidence_score > 60.0
                && pattern.count > 50
                && matches!(pattern.pattern_type, PatternType::MintSuffix)
            {
                // Check if this pattern isn't already categorized
                let already_categorized = self
                    .platforms
                    .values()
                    .any(|p| p.identification_pattern == pattern.pattern);

                if !already_categorized {
                    let platform_name = format!("Platform-{}", pattern.pattern.to_uppercase());
                    let platform = PlatformCategory {
                        platform_name: platform_name.clone(),
                        identification_pattern: pattern.pattern.clone(),
                        pattern_type: pattern.pattern_type.clone(),
                        token_count: pattern.count,
                        examples: pattern.examples.clone(),
                    };
                    self.platforms.insert(platform_name, platform);
                }
            }
        }
    }

    /// Get all detected patterns
    pub fn get_patterns(&self) -> &HashMap<String, TokenPattern> {
        &self.patterns
    }

    /// Get identified platform categories
    pub fn get_platforms(&self) -> &HashMap<String, PlatformCategory> {
        &self.platforms
    }

    /// Get pattern analysis summary
    pub fn get_analysis_summary(&self) -> PatternAnalysisSummary {
        let total_patterns = self.patterns.len();
        let high_confidence_patterns = self
            .patterns
            .values()
            .filter(|p| p.confidence_score > 70.0)
            .count();

        let platform_count = self.platforms.len();
        let total_categorized_tokens: usize = self.platforms.values().map(|p| p.token_count).sum();

        PatternAnalysisSummary {
            total_patterns,
            high_confidence_patterns,
            platform_count,
            total_categorized_tokens,
            total_analyzed_tokens: self.last_token_count,
            last_analysis: self.last_analysis,
        }
    }

    /// Categorize a specific token based on detected patterns
    pub fn categorize_token(
        &self,
        mint: &str,
        name: &str,
        symbol: &str,
    ) -> Vec<TokenCategorization> {
        let mut categorizations = Vec::new();

        // Check against all detected patterns
        for (key, pattern) in &self.patterns {
            let matches = match pattern.pattern_type {
                PatternType::MintSuffix => mint.ends_with(&pattern.pattern),
                PatternType::MintPrefix => mint.starts_with(&pattern.pattern),
                PatternType::NameSuffix => name.ends_with(&pattern.pattern),
                PatternType::NamePrefix => name.starts_with(&pattern.pattern),
                PatternType::SymbolSuffix => symbol.ends_with(&pattern.pattern),
                PatternType::SymbolPrefix => symbol.starts_with(&pattern.pattern),
            };

            if matches {
                // Check if this pattern corresponds to a known platform
                let platform = self
                    .platforms
                    .values()
                    .find(|p| {
                        p.identification_pattern == pattern.pattern
                            && p.pattern_type == pattern.pattern_type
                    })
                    .map(|p| p.platform_name.clone());

                categorizations.push(TokenCategorization {
                    category: platform.unwrap_or_else(|| format!("Pattern-{}", pattern.pattern)),
                    pattern: pattern.pattern.clone(),
                    pattern_type: pattern.pattern_type.clone(),
                    confidence: pattern.confidence_score,
                    pattern_popularity: pattern.count,
                });
            }
        }

        categorizations
    }
}

#[derive(Debug, Clone)]
pub struct TokenCategorization {
    pub category: String,
    pub pattern: String,
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub pattern_popularity: usize,
}

#[derive(Debug)]
pub struct PatternAnalysisSummary {
    pub total_patterns: usize,
    pub high_confidence_patterns: usize,
    pub platform_count: usize,
    pub total_categorized_tokens: usize,
    pub total_analyzed_tokens: usize,
    pub last_analysis: chrono::DateTime<chrono::Utc>,
}

/// Initialize the global pattern analyzer
pub async fn initialize_pattern_analyzer() -> Result<(), String> {
    let mut analyzer = TokenPatternAnalyzer::new();
    analyzer.analyze_patterns().await?;

    let mut global_analyzer = PATTERN_ANALYZER.lock().unwrap();
    *global_analyzer = Some(analyzer);

    Ok(())
}

/// Get the global pattern analyzer instance
pub fn get_pattern_analyzer() -> Option<std::sync::MutexGuard<'static, Option<TokenPatternAnalyzer>>>
{
    PATTERN_ANALYZER.lock().ok()
}

/// Refresh pattern analysis (should be called periodically)
pub async fn refresh_pattern_analysis() -> Result<(), String> {
    let mut global_analyzer = PATTERN_ANALYZER.lock().unwrap();
    if let Some(ref mut analyzer) = *global_analyzer {
        analyzer.analyze_patterns().await?;
    } else {
        // Initialize if not already done
        drop(global_analyzer);
        initialize_pattern_analyzer().await?;
    }
    Ok(())
}

/// Get pattern categorization for a specific token
pub fn categorize_token(mint: &str, name: &str, symbol: &str) -> Vec<TokenCategorization> {
    if let Some(analyzer_guard) = get_pattern_analyzer() {
        if let Some(ref analyzer) = *analyzer_guard {
            return analyzer.categorize_token(mint, name, symbol);
        }
    }
    Vec::new()
}

/// Log pattern analysis results for monitoring
pub fn log_pattern_analysis_results() {
    if let Some(analyzer_guard) = get_pattern_analyzer() {
        if let Some(ref analyzer) = *analyzer_guard {
            let summary = analyzer.get_analysis_summary();

            log(
                LogTag::Discovery,
                "PATTERNS",
                &format!(
                    "Pattern Analysis: {} total patterns ({} high-confidence), {} platforms identified, {}/{} tokens categorized",
                    summary.total_patterns,
                    summary.high_confidence_patterns,
                    summary.platform_count,
                    summary.total_categorized_tokens,
                    summary.total_analyzed_tokens
                )
            );

            // Log top platforms
            let mut platforms: Vec<_> = analyzer.get_platforms().values().collect();
            platforms.sort_by(|a, b| b.token_count.cmp(&a.token_count));

            for (i, platform) in platforms.iter().take(10).enumerate() {
                log(
                    LogTag::Discovery,
                    "PLATFORM",
                    &format!(
                        "{}. {}: {} tokens (pattern: '{}')",
                        i + 1,
                        platform.platform_name,
                        platform.token_count,
                        platform.identification_pattern
                    ),
                );
            }
        }
    }
}
