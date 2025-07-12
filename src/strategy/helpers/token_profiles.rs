use crate::prelude::*;

/// Token-specific trading configuration and characteristics
pub struct TokenProfile {
    pub mint: String,
    pub symbol: String,
    pub is_famous: bool,
    pub holder_base_size: HolderBaseSize,
    pub volatility_level: VolatilityLevel,
    pub liquidity_stability: LiquidityStability,
}

#[derive(Debug, Clone)]
pub enum HolderBaseSize {
    VerySmall, // < 50 holders
    Small, // 50-200 holders
    Medium, // 200-1000 holders
    Large, // 1000-5000 holders
    VeryLarge, // > 5000 holders
}

#[derive(Debug, Clone)]
pub enum VolatilityLevel {
    Low, // Stable price movements
    Medium, // Normal volatility
    High, // High volatility
    Extreme, // Extreme volatility
}

#[derive(Debug, Clone)]
pub enum LiquidityStability {
    Stable, // Consistent liquidity
    Variable, // Fluctuating liquidity
    Unstable, // Unpredictable liquidity
}

impl TokenProfile {
    /// Create profile for a token based on current data
    pub fn from_token(token: &Token) -> Self {
        let holder_base = Self::classify_holder_base(token.rug_check.total_holders);
        let volatility = Self::classify_volatility(token);
        let liquidity_stability = Self::classify_liquidity_stability(token);
        let is_famous = Self::is_famous_token(&token.symbol, &token.name);

        Self {
            mint: token.mint.clone(),
            symbol: token.symbol.clone(),
            is_famous,
            holder_base_size: holder_base,
            volatility_level: volatility,
            liquidity_stability,
        }
    }

    fn classify_holder_base(holder_count: u64) -> HolderBaseSize {
        match holder_count {
            0..=49 => HolderBaseSize::VerySmall,
            50..=199 => HolderBaseSize::Small,
            200..=999 => HolderBaseSize::Medium,
            1000..=4999 => HolderBaseSize::Large,
            _ => HolderBaseSize::VeryLarge,
        }
    }

    fn classify_volatility(token: &Token) -> VolatilityLevel {
        let price_changes = vec![
            token.price_change.m5.abs(),
            token.price_change.h1.abs(),
            token.price_change.h6.abs()
        ];

        let avg_volatility = price_changes.iter().sum::<f64>() / (price_changes.len() as f64);

        match avg_volatility {
            v if v < 2.0 => VolatilityLevel::Low,
            v if v < 8.0 => VolatilityLevel::Medium,
            v if v < 20.0 => VolatilityLevel::High,
            _ => VolatilityLevel::Extreme,
        }
    }

    fn classify_liquidity_stability(token: &Token) -> LiquidityStability {
        let liquidity_sol = token.liquidity.base + token.liquidity.quote;
        let volume_to_liquidity_ratio = token.volume.h24 / liquidity_sol.max(1.0);

        match volume_to_liquidity_ratio {
            r if r < 0.1 => LiquidityStability::Stable,
            r if r < 0.5 => LiquidityStability::Variable,
            _ => LiquidityStability::Unstable,
        }
    }

    fn is_famous_token(symbol: &str, name: &str) -> bool {
        let famous_tokens = vec![
            "MOONCAT",
            "BONK",
            "WIF",
            "POPCAT",
            "BRETT",
            "PEPE",
            "DOGE",
            "SHIB",
            "FLOKI",
            "BABY",
            "MOCHI",
            "PNUT",
            "ACT",
            "GOAT",
            "FWOG",
            "CHILLGUY"
        ];

        famous_tokens
            .iter()
            .any(|&famous| {
                symbol.to_uppercase().contains(famous) || name.to_uppercase().contains(famous)
            })
    }

    /// Get trading configuration for this token profile
    pub fn get_trading_config(&self) -> TokenTradingConfig {
        let mut config = TokenTradingConfig::default();

        // Adjust for famous tokens (like MOONCAT)
        if self.is_famous {
            config.position_size_multiplier *= 1.3;
            config.confidence_requirement *= 0.9; // Lower confidence needed
            config.max_hold_time_hours *= 2; // Can hold longer
        }

        // Adjust for holder base size
        match self.holder_base_size {
            HolderBaseSize::VerySmall => {
                config.position_size_multiplier *= 0.6;
                config.confidence_requirement *= 1.3;
                config.max_hold_time_hours /= 2;
            }
            HolderBaseSize::Small => {
                config.position_size_multiplier *= 0.8;
                config.confidence_requirement *= 1.1;
            }
            HolderBaseSize::Large | HolderBaseSize::VeryLarge => {
                config.position_size_multiplier *= 1.2;
                config.confidence_requirement *= 0.9;
            }
            _ => {} // Medium - use defaults
        }

        // Adjust for volatility
        match self.volatility_level {
            VolatilityLevel::Low => {
                config.profit_targets = vec![0.5, 1.0, 2.0, 4.0]; // Lower targets for stable tokens
                config.dca_spacing_multiplier *= 1.5; // Wider DCA spacing
            }
            VolatilityLevel::High => {
                config.profit_targets = vec![1.0, 3.0, 6.0, 12.0]; // Higher targets for volatile tokens
                config.dca_spacing_multiplier *= 0.7; // Tighter DCA spacing
            }
            VolatilityLevel::Extreme => {
                config.profit_targets = vec![2.0, 5.0, 10.0, 20.0]; // Much higher targets
                config.dca_spacing_multiplier *= 0.5; // Very tight DCA spacing
                config.position_size_multiplier *= 0.7; // Smaller positions
            }
            _ => {} // Medium - use defaults
        }

        // Adjust for liquidity stability
        match self.liquidity_stability {
            LiquidityStability::Unstable => {
                config.position_size_multiplier *= 0.8;
                config.confidence_requirement *= 1.2;
                config.max_hold_time_hours /= 2;
            }
            LiquidityStability::Stable => {
                config.position_size_multiplier *= 1.1;
                config.max_hold_time_hours = ((config.max_hold_time_hours as f64) * 1.5) as u64;
            }
            _ => {} // Variable - use defaults
        }

        config
    }
}

#[derive(Debug, Clone)]
pub struct TokenTradingConfig {
    pub position_size_multiplier: f64,
    pub confidence_requirement: f64,
    pub profit_targets: Vec<f64>,
    pub dca_spacing_multiplier: f64,
    pub max_hold_time_hours: u64,
    pub max_dca_levels: u8,
}

impl Default for TokenTradingConfig {
    fn default() -> Self {
        Self {
            position_size_multiplier: 1.0,
            confidence_requirement: 0.6,
            profit_targets: vec![0.8, 2.0, 4.5, 9.0], // Default profit targets
            dca_spacing_multiplier: 1.0,
            max_hold_time_hours: 24,
            max_dca_levels: 5,
        }
    }
}
