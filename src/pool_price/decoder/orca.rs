use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use super::super::types::*;
use crate::pool_price::types::pool_log;

/// Parse Orca Whirlpool pool data from raw account bytes
pub fn parse_orca_whirlpool_data(data: &[u8]) -> Result<OrcaWhirlpoolData> {
    pool_log("INFO", &format!("Parsing Orca Whirlpool pool data: {} bytes", data.len()));

    if data.len() < 653 {
        pool_log(
            "ERROR",
            &format!("Whirlpool data too short: {} bytes (expected at least 653)", data.len())
        );
        return Err(anyhow::anyhow!("Orca Whirlpool pool data too short: {} bytes", data.len()));
    }

    let mut offset = 8; // Skip discriminator

    // Parse whirlpool configuration and basic parameters
    let whirlpools_config = Pubkey::new_from_array(
        data[offset..offset + 32].try_into()?
    ).to_string();
    offset += 32;

    let whirlpool_bump = data[offset];
    offset += 1;

    let tick_spacing = u16::from_le_bytes(data[offset..offset + 2].try_into()?);
    offset += 4; // Skip tick_spacing + feeTierIndexSeed

    let fee_rate = u16::from_le_bytes(data[offset..offset + 2].try_into()?);
    offset += 2;

    let protocol_fee_rate = u16::from_le_bytes(data[offset..offset + 2].try_into()?);
    offset += 2;

    // Parse liquidity and price data
    let liquidity = u128::from_le_bytes(data[offset..offset + 16].try_into()?);
    offset += 16;

    let sqrt_price = u128::from_le_bytes(data[offset..offset + 16].try_into()?);
    offset += 16;

    let tick_current_index = i32::from_le_bytes(data[offset..offset + 4].try_into()?);
    offset += 4;

    let protocol_fee_owed_a = u64::from_le_bytes(data[offset..offset + 8].try_into()?);
    offset += 8;

    let protocol_fee_owed_b = u64::from_le_bytes(data[offset..offset + 8].try_into()?);
    offset += 8;

    // Parse token mints and vaults
    let token_mint_a = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    let token_vault_a = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    let fee_growth_global_a = u128::from_le_bytes(data[offset..offset + 16].try_into()?);
    offset += 16;

    let token_mint_b = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    let token_vault_b = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    let fee_growth_global_b = u128::from_le_bytes(data[offset..offset + 16].try_into()?);

    // Check if token B is SOL
    const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
    let is_sol_pair = token_mint_b == SOL_MINT;

    pool_log(
        "SUCCESS",
        &format!(
            "Parsed Whirlpool: tokenA={}, tokenB={} {}, liquidity={}",
            token_mint_a,
            token_mint_b,
            if is_sol_pair {
                "(SOL)"
            } else {
                ""
            },
            liquidity
        )
    );

    Ok(OrcaWhirlpoolData {
        whirlpools_config,
        token_mint_a,
        token_mint_b,
        token_vault_a,
        token_vault_b,
        fee_rate,
        protocol_fee_rate,
        liquidity,
        sqrt_price,
        tick_current_index,
        tick_spacing,
        protocol_fee_owed_a,
        protocol_fee_owed_b,
        fee_growth_global_a,
        fee_growth_global_b,
        whirlpool_bump,
    })
}
