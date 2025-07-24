use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use super::super::types::*;
use crate::logger::{ log, LogTag };

/// Check if a mint is SOL or WSOL
fn is_sol_mint(mint: &str) -> bool {
    const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
    mint == SOL_MINT
}

/// Parse Orca Whirlpool pool data from raw account bytes
pub fn parse_orca_whirlpool_data(data: &[u8]) -> Result<OrcaWhirlpoolData> {
    log(LogTag::Pool, "DEBUG", &format!("Orca Whirlpool pool data length: {} bytes", data.len()));

    if data.len() < 653 {
        log(
            LogTag::Pool,
            "ERROR",
            &format!(
                "Orca Whirlpool pool data too short: {} bytes (expected at least 653)",
                data.len()
            )
        );
        return Err(anyhow::anyhow!("Orca Whirlpool pool data too short: {} bytes", data.len()));
    }

    // Based on the provided JSON structure, parse the Whirlpool data
    let mut offset = 8; // Skip 8-byte discriminator

    // whirlpoolsConfig (32 bytes) - Skip for now, but we could store it
    let whirlpools_config = Pubkey::new_from_array(
        data[offset..offset + 32].try_into()?
    ).to_string();
    offset += 32;

    // whirlpoolBump (1 byte)
    let whirlpool_bump = data[offset];
    offset += 1;

    // tickSpacing (u16) - 2 bytes
    let tick_spacing = u16::from_le_bytes(data[offset..offset + 2].try_into()?);
    offset += 2;

    // feeTierIndexSeed (2 bytes) - skip
    offset += 2;

    // feeRate (u16) - 2 bytes
    let fee_rate = u16::from_le_bytes(data[offset..offset + 2].try_into()?);
    offset += 2;

    // protocolFeeRate (u16) - 2 bytes
    let protocol_fee_rate = u16::from_le_bytes(data[offset..offset + 2].try_into()?);
    offset += 2;

    // liquidity (u128) - 16 bytes
    let liquidity = u128::from_le_bytes(data[offset..offset + 16].try_into()?);
    offset += 16;

    // sqrtPrice (u128) - 16 bytes
    let sqrt_price = u128::from_le_bytes(data[offset..offset + 16].try_into()?);
    offset += 16;

    // tickCurrentIndex (i32) - 4 bytes
    let tick_current_index = i32::from_le_bytes(data[offset..offset + 4].try_into()?);
    offset += 4;

    // protocolFeeOwedA (u64) - 8 bytes
    let protocol_fee_owed_a = u64::from_le_bytes(data[offset..offset + 8].try_into()?);
    offset += 8;

    // protocolFeeOwedB (u64) - 8 bytes
    let protocol_fee_owed_b = u64::from_le_bytes(data[offset..offset + 8].try_into()?);
    offset += 8;

    // tokenMintA (32 bytes)
    let token_mint_a = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    // tokenVaultA (32 bytes)
    let token_vault_a = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    // feeGrowthGlobalA (u128) - 16 bytes
    let fee_growth_global_a = u128::from_le_bytes(data[offset..offset + 16].try_into()?);
    offset += 16;

    // tokenMintB (32 bytes)
    let token_mint_b = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    // tokenVaultB (32 bytes)
    let token_vault_b = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    // feeGrowthGlobalB (u128) - 16 bytes
    let fee_growth_global_b = u128::from_le_bytes(data[offset..offset + 16].try_into()?);

    log(
        LogTag::Pool,
        "SUCCESS",
        &format!(
            "Parsed Orca Whirlpool: tokenA={}, tokenB={} ({}), liquidity={}, sqrt_price={}, tick_spacing={}, fee_rate={}",
            &token_mint_a,
            &token_mint_b,
            if is_sol_mint(&token_mint_b) {
                "✅SOL"
            } else {
                "❌NOT_SOL"
            },
            liquidity,
            sqrt_price,
            tick_spacing,
            fee_rate
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
