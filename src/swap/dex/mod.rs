/// DEX implementations for different protocols
/// 
/// This module contains implementations for various DEX protocols:
/// - Jupiter: Solana's premier DEX aggregator
/// - Raydium: Popular AMM on Solana
/// - GMGN: Trading platform with advanced features

pub mod jupiter;
pub mod raydium;
pub mod gmgn;

pub use jupiter::JupiterSwap;
pub use raydium::RaydiumSwap;
pub use gmgn::GmgnSwap;

use crate::swap::types::*;

/// Factory function to create DEX instances based on configuration
pub fn create_dex_instances(config: &SwapConfig) -> DexInstances {
    let jupiter = if config.jupiter.enabled {
        Some(JupiterSwap::new(config.jupiter.clone()))
    } else {
        None
    };

    let raydium = if config.raydium.enabled {
        Some(RaydiumSwap::new(config.raydium.clone()))
    } else {
        None
    };

    let gmgn = if config.gmgn.enabled {
        Some(GmgnSwap::new(config.gmgn.clone()))
    } else {
        None
    };

    DexInstances {
        jupiter,
        raydium,
        gmgn,
    }
}

/// Container for all DEX instances
pub struct DexInstances {
    pub jupiter: Option<JupiterSwap>,
    pub raydium: Option<RaydiumSwap>,
    pub gmgn: Option<GmgnSwap>,
}

impl DexInstances {
    /// Get the names of all available DEXes
    pub fn get_available_dex_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        
        if self.jupiter.is_some() {
            names.push("jupiter");
        }
        
        if self.raydium.is_some() {
            names.push("raydium");
        }
        
        if self.gmgn.is_some() {
            names.push("gmgn");
        }
        
        names
    }
    
    /// Check if a specific DEX is available
    pub fn has_dex(&self, name: &str) -> bool {
        match name.to_lowercase().as_str() {
            "jupiter" => self.jupiter.is_some(),
            "raydium" => self.raydium.is_some(),
            "gmgn" => self.gmgn.is_some(),
            _ => false,
        }
    }
}
