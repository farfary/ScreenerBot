use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use super::super::types::*;
use crate::pool_price::types::pool_log;

/// Parse Raydium CPMM pool data from raw account bytes
pub fn parse_raydium_cpmm_data(data: &[u8]) -> Result<RaydiumCpmmData> {
    pool_log("INFO", &format!("Parsing Raydium CPMM pool data: {} bytes", data.len()));

    if data.len() < 600 {
        pool_log("ERROR", &format!("Pool data too short: {} bytes", data.len()));
        return Err(anyhow::anyhow!("Pool data too short"));
    }

    // Parse the CPMM pool structure
    let mut offset = 40; // Skip discriminator and config
    offset += 32; // Skip pool_creator

    // Token vaults
    let token_0_vault = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;
    let token_1_vault = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;
    offset += 32; // Skip lp_mint

    // Token mints
    let token_0_mint = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;
    let token_1_mint = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 96; // Skip program keys and observation_key

    // Extract status (decimals should come from mint accounts, not pool data)
    let status = data[offset + 1];
    // Use default decimal values - actual decimals will be fetched from mint accounts
    let mint_0_decimals = 6; // Default SOL decimals
    let mint_1_decimals = 6; // Default token decimals

    pool_log(
        "SUCCESS",
        &format!("Parsed CPMM: token_0={}, token_1={}", token_0_mint, token_1_mint)
    );

    Ok(RaydiumCpmmData {
        token_0_mint,
        token_1_mint,
        token_0_vault,
        token_1_vault,
        mint_0_decimals,
        mint_1_decimals,
        status,
    })
}

/// Parse Raydium AMM pool data from raw account bytes
pub fn parse_raydium_amm_data(data: &[u8]) -> Result<RaydiumAmmData> {
    pool_log("INFO", &format!("Parsing Raydium AMM pool data: {} bytes", data.len()));

    if data.len() < 264 {
        pool_log("ERROR", &format!("AMM account too short: {} bytes", data.len()));
        return Err(anyhow::anyhow!("AMM account too short: {} bytes", data.len()));
    }

    // Extract mint addresses and vaults from pool account
    let base_mint = Pubkey::new_from_array(data[168..200].try_into()?);
    let quote_mint = Pubkey::new_from_array(data[216..248].try_into()?);
    let base_vault = Pubkey::new_from_array(data[200..232].try_into()?);
    let quote_vault = Pubkey::new_from_array(data[232..264].try_into()?);

    // Validate pubkeys
    const NULL_PUBKEY: &str = "11111111111111111111111111111111";
    const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

    if
        base_mint.to_string() == NULL_PUBKEY ||
        quote_mint.to_string() == NULL_PUBKEY ||
        base_vault.to_string() == NULL_PUBKEY ||
        quote_vault.to_string() == NULL_PUBKEY
    {
        pool_log("ERROR", "Invalid null pubkeys found in AMM pool data");
        return Err(anyhow::anyhow!("Invalid null pubkeys found in Raydium AMM pool data"));
    }

    if base_vault.to_string() == SOL_MINT || quote_vault.to_string() == SOL_MINT {
        pool_log("ERROR", "Invalid vault addresses (SOL mint used as vault)");
        return Err(anyhow::anyhow!("Invalid vault addresses in Raydium AMM pool data"));
    }

    pool_log("SUCCESS", &format!("Parsed AMM: base={}, quote={}", base_mint, quote_mint));

    Ok(RaydiumAmmData {
        base_mint: base_mint.to_string(),
        quote_mint: quote_mint.to_string(),
        base_vault: base_vault.to_string(),
        quote_vault: quote_vault.to_string(),
        base_decimals: 9, // Default, will be overridden
        quote_decimals: 9, // Default, will be overridden
    })
}

/// Parse Raydium LaunchLab pool data from raw account bytes
pub fn parse_raydium_launchlab_data(data: &[u8]) -> Result<RaydiumLaunchLabData> {
    pool_log("INFO", &format!("Parsing Raydium LaunchLab pool data: {} bytes", data.len()));

    if data.len() < 317 {
        pool_log(
            "ERROR",
            &format!("LaunchLab pool data too short: {} bytes (minimum: 317)", data.len())
        );
        return Err(anyhow::anyhow!("Raydium LaunchLab pool data too short: {} bytes", data.len()));
    }

    // Parse pool structure
    let mut offset = 8; // Skip epoch
    offset += 1; // Skip auth_bump
    let status = data[offset];
    offset += 1;

    // Parse real base and quote values
    let real_base = if data.len() > 37 {
        u64::from_le_bytes(data[29..37].try_into().unwrap_or([0; 8]))
    } else {
        0
    };

    let real_quote = if data.len() > 69 {
        u64::from_le_bytes(data[61..69].try_into().unwrap_or([0; 8]))
    } else {
        0
    };

    // Parse mints with fallback offsets
    let base_mint = if data.len() > 237 {
        if let Ok(bytes_array) = data[205..237].try_into() {
            Pubkey::new_from_array(bytes_array).to_string()
        } else {
            Pubkey::new_from_array(data[192..224].try_into()?).to_string()
        }
    } else {
        Pubkey::new_from_array(data[192..224].try_into()?).to_string()
    };

    let quote_mint = if data.len() > 269 {
        if let Ok(bytes_array) = data[237..269].try_into() {
            Pubkey::new_from_array(bytes_array).to_string()
        } else {
            Pubkey::new_from_array(data[224..256].try_into()?).to_string()
        }
    } else {
        Pubkey::new_from_array(data[224..256].try_into()?).to_string()
    };

    // Parse vaults
    let base_vault = Pubkey::new_from_array(data[256..288].try_into()?).to_string();
    let quote_vault = Pubkey::new_from_array(data[288..320].try_into()?).to_string();

    pool_log(
        "SUCCESS",
        &format!(
            "Parsed LaunchLab: base={}, quote={}, real_base={}, real_quote={}",
            base_mint,
            quote_mint,
            real_base,
            real_quote
        )
    );

    Ok(RaydiumLaunchLabData {
        base_mint,
        quote_mint,
        base_vault,
        quote_vault,
        base_decimals: 6, // Expected value
        quote_decimals: 9, // SOL decimals
        total_base_sell: 0,
        real_base,
        real_quote,
        status,
    })
}
