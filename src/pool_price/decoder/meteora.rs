use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use super::super::types::*;
use crate::logger::{ log, LogTag };

/// Parse Meteora DLMM pool data from raw account bytes
pub fn parse_meteora_dlmm_data(data: &[u8]) -> Result<MeteoraPoolData> {
    if data.len() < 800 {
        return Err(anyhow::anyhow!("Meteora DLMM pool data too short"));
    }

    // Based on the provided Meteora DLMM layout, let's be more careful with offsets
    // The structure is quite complex, so we'll parse it step by step

    let mut offset = 0;

    // Skip discriminator (8 bytes typically)
    offset += 8;

    // StaticParameters struct - let's calculate size more carefully
    // baseFactor(u16) + filterPeriod(u16) + decayPeriod(u16) + reductionFactor(u16) +
    // variableFeeControl(u32) + maxVolatilityAccumulator(u32) + minBinId(i32) + maxBinId(i32) +
    // protocolShare(u16) + baseFeePowerFactor(u8) + padding([u8;5])
    // = 2+2+2+2+4+4+4+4+2+1+5 = 32 bytes
    offset += 32;

    // VariableParameters struct
    // volatilityAccumulator(u32) + volatilityReference(u32) + indexReference(i32) +
    // padding([u8;4]) + lastUpdateTimestamp(i64) + padding1([u8;8])
    // = 4+4+4+4+8+8 = 32 bytes
    offset += 32;

    // bumpSeed([u8;1]) + binStepSeed([u8;2]) + pairType(u8) = 4 bytes
    offset += 4;

    // activeId (i32) - 4 bytes
    let active_id_bytes: [u8; 4] = data[offset..offset + 4]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to read activeId"))?;
    let active_id = i32::from_le_bytes(active_id_bytes);
    offset += 4;

    // binStep (u16) - 2 bytes
    let bin_step_bytes: [u8; 2] = data[offset..offset + 2]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to read binStep"))?;
    let bin_step = u16::from_le_bytes(bin_step_bytes);
    offset += 2;

    // status (u8) - 1 byte
    let status = data[offset];
    offset += 1;

    // requireBaseFactorSeed(u8) + baseFactorSeed([u8;2]) + activationType(u8) + creatorPoolOnOffControl(u8) = 5 bytes
    offset += 5;

    // tokenXMint (32 bytes)
    if offset + 32 > data.len() {
        return Err(anyhow::anyhow!("Data too short for tokenXMint at offset {}", offset));
    }
    let token_x_mint = Pubkey::new_from_array(
        data[offset..offset + 32]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read tokenXMint"))?
    ).to_string();
    offset += 32;

    // tokenYMint (32 bytes)
    if offset + 32 > data.len() {
        return Err(anyhow::anyhow!("Data too short for tokenYMint at offset {}", offset));
    }
    let token_y_mint = Pubkey::new_from_array(
        data[offset..offset + 32]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read tokenYMint"))?
    ).to_string();
    offset += 32;

    // reserveX (32 bytes)
    if offset + 32 > data.len() {
        return Err(anyhow::anyhow!("Data too short for reserveX at offset {}", offset));
    }
    let reserve_x = Pubkey::new_from_array(
        data[offset..offset + 32]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read reserveX"))?
    ).to_string();
    offset += 32;

    // reserveY (32 bytes)
    if offset + 32 > data.len() {
        return Err(anyhow::anyhow!("Data too short for reserveY at offset {}", offset));
    }
    let reserve_y = Pubkey::new_from_array(
        data[offset..offset + 32]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read reserveY"))?
    ).to_string();

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
    if data.len() < 500 {
        return Err(anyhow::anyhow!("Meteora DAMM v2 pool data too short: {} bytes", data.len()));
    }

    debug_log("DEBUG", &format!("Parsing DAMM v2 pool data: {} bytes", data.len()));

    // Based on our analysis of actual pool data, the correct field positions are:

    // token_a_mint at offset 168 (32 bytes)
    let token_a_mint = Pubkey::new_from_array(
        data[168..168 + 32]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read token_a_mint at offset 168"))?
    ).to_string();

    // token_b_mint at offset 200 (32 bytes)
    let token_b_mint = Pubkey::new_from_array(
        data[200..200 + 32]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read token_b_mint at offset 200"))?
    ).to_string();

    // token_a_vault at offset 232 (32 bytes)
    let token_a_vault = Pubkey::new_from_array(
        data[232..232 + 32]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read token_a_vault at offset 232"))?
    ).to_string();

    // token_b_vault at offset 264 (32 bytes)
    let token_b_vault = Pubkey::new_from_array(
        data[264..264 + 32]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to read token_b_vault at offset 264"))?
    ).to_string();

    // liquidity at offset 360 (16 bytes as u128)
    let liquidity_bytes: [u8; 16] = data[360..360 + 16]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to read liquidity at offset 360"))?;
    let liquidity = u128::from_le_bytes(liquidity_bytes);

    // sqrt_price at offset 456 (16 bytes as u128)
    let sqrt_price_bytes: [u8; 16] = data[456..456 + 16]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to read sqrt_price at offset 456"))?;
    let sqrt_price = u128::from_le_bytes(sqrt_price_bytes);

    // pool_status - let's check a few possible locations based on the JSON structure
    // The JSON shows activation_type and pool_status fields, let's try around offset 470-480
    let pool_status = if data.len() > 480 {
        data[480] // Try this position first
    } else {
        0 // Default value if we can't read it
    };

    log(
        LogTag::Pool,
        "DEBUG",
        &format!(
            "DAMM v2 parsed - token_a: {}, token_b: {}, token_a_vault: {}, token_b_vault: {}, liquidity: {}, sqrt_price: {}, status: {}",
            token_a_mint,
            token_b_mint,
            token_a_vault,
            token_b_vault,
            liquidity,
            sqrt_price,
            pool_status
        )
    );

    // Validate that we have valid pubkeys (not null keys)
    const NULL_PUBKEY: &str = "11111111111111111111111111111111"; // 32 bytes of zeros as base58
    if
        token_a_mint == NULL_PUBKEY ||
        token_b_mint == NULL_PUBKEY ||
        token_a_vault == NULL_PUBKEY ||
        token_b_vault == NULL_PUBKEY
    {
        return Err(anyhow::anyhow!("Invalid null pubkeys found in DAMM v2 pool data"));
    }

    log(
        LogTag::Pool,
        "SUCCESS",
        &format!(
            "Successfully parsed DAMM v2 pool: token_a={}, token_b={}, liquidity={}, sqrt_price={}",
            token_a_mint,
            token_b_mint,
            liquidity,
            sqrt_price
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
