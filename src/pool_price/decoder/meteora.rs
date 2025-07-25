use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use super::super::types::*;
use crate::pool_price::types::pool_log;

/// Parse Meteora DLMM pool data from raw account bytes
pub fn parse_meteora_dlmm_data(data: &[u8]) -> Result<MeteoraPoolData> {
    pool_log("INFO", &format!("Parsing Meteora DLMM pool data: {} bytes", data.len()));

    if data.len() < 800 {
        pool_log("ERROR", &format!("DLMM pool data too short: {} bytes", data.len()));
        return Err(anyhow::anyhow!("Meteora DLMM pool data too short"));
    }

    let mut offset = 8; // Skip discriminator
    offset += 32; // Skip StaticParameters struct
    offset += 32; // Skip VariableParameters struct
    offset += 4; // Skip bumpSeed, binStepSeed, pairType

    // Parse activeId, binStep, status
    let active_id = i32::from_le_bytes(
        data[offset..offset + 4].try_into().map_err(|_| anyhow::anyhow!("Failed to read activeId"))?
    );
    offset += 4;

    let bin_step = u16::from_le_bytes(
        data[offset..offset + 2].try_into().map_err(|_| anyhow::anyhow!("Failed to read binStep"))?
    );
    offset += 2;

    let status = data[offset];
    offset += 6; // Skip status + other fields

    // Parse token mints and reserves
    let token_x_mint = Pubkey::new_from_array(
        data[offset..offset + 32]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read tokenXMint"))?
    ).to_string();
    offset += 32;

    let token_y_mint = Pubkey::new_from_array(
        data[offset..offset + 32]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read tokenYMint"))?
    ).to_string();
    offset += 32;

    let reserve_x = Pubkey::new_from_array(
        data[offset..offset + 32]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read reserveX"))?
    ).to_string();
    offset += 32;

    let reserve_y = Pubkey::new_from_array(
        data[offset..offset + 32]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read reserveY"))?
    ).to_string();

    pool_log(
        "SUCCESS",
        &format!(
            "Parsed DLMM: tokenX={}, tokenY={}, active_id={}",
            token_x_mint,
            token_y_mint,
            active_id
        )
    );

    Ok(MeteoraPoolData {
        token_x_mint,
        token_y_mint,
        reserve_x,
        reserve_y,
        active_id,
        bin_step,
        status,
    })
}

/// Parse Meteora DAMM v2 pool data from raw account bytes
pub fn parse_meteora_damm_v2_data(data: &[u8]) -> Result<MeteoraDammV2Data> {
    pool_log("INFO", &format!("Parsing Meteora DAMM v2 pool data: {} bytes", data.len()));

    if data.len() < 500 {
        pool_log("ERROR", &format!("DAMM v2 pool data too short: {} bytes", data.len()));
        return Err(anyhow::anyhow!("Meteora DAMM v2 pool data too short: {} bytes", data.len()));
    }

    // Parse token mints and vaults at known offsets
    let token_a_mint = Pubkey::new_from_array(
        data[168..200]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read token_a_mint at offset 168"))?
    ).to_string();

    let token_b_mint = Pubkey::new_from_array(
        data[200..232]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read token_b_mint at offset 200"))?
    ).to_string();

    let token_a_vault = Pubkey::new_from_array(
        data[232..264]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read token_a_vault at offset 232"))?
    ).to_string();

    let token_b_vault = Pubkey::new_from_array(
        data[264..296]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read token_b_vault at offset 264"))?
    ).to_string();

    // Parse liquidity and sqrt_price
    let liquidity = u128::from_le_bytes(
        data[360..376]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read liquidity at offset 360"))?
    );

    let sqrt_price = u128::from_le_bytes(
        data[456..472]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read sqrt_price at offset 456"))?
    );

    let pool_status = if data.len() > 480 { data[480] } else { 0 };

    // Validate pubkeys
    const NULL_PUBKEY: &str = "11111111111111111111111111111111";
    if
        token_a_mint == NULL_PUBKEY ||
        token_b_mint == NULL_PUBKEY ||
        token_a_vault == NULL_PUBKEY ||
        token_b_vault == NULL_PUBKEY
    {
        pool_log("ERROR", "Invalid null pubkeys found in DAMM v2 pool data");
        return Err(anyhow::anyhow!("Invalid null pubkeys found in DAMM v2 pool data"));
    }

    pool_log(
        "SUCCESS",
        &format!(
            "Parsed DAMM v2: token_a={}, token_b={}, liquidity={}",
            token_a_mint,
            token_b_mint,
            liquidity
        )
    );

    Ok(MeteoraDammV2Data {
        token_a_mint,
        token_b_mint,
        token_a_vault,
        token_b_vault,
        liquidity,
        sqrt_price,
        pool_status,
    })
}
