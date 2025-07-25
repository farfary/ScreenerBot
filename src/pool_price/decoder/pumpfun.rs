use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use super::super::types::*;
use crate::pool_price::types::pool_log;

/// Parse Pump.fun AMM pool data from raw account bytes
pub fn parse_pumpfun_amm_pool(data: &[u8]) -> Result<PoolData> {
    pool_log("INFO", &format!("Parsing Pump.fun AMM pool data: {} bytes", data.len()));

    let mut offset = 8; // Skip discriminator

    // Parse pool metadata
    let pool_bump = data[offset];
    offset += 1;

    let index = u16::from_le_bytes(data[offset..offset + 2].try_into()?);
    offset += 2;

    // Parse addresses
    let creator = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    let base_mint = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    let quote_mint = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    let lp_mint = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    let pool_base_token_account = Pubkey::new_from_array(
        data[offset..offset + 32].try_into()?
    ).to_string();
    offset += 32;

    let pool_quote_token_account = Pubkey::new_from_array(
        data[offset..offset + 32].try_into()?
    ).to_string();
    offset += 32;

    // Parse LP supply
    let lp_supply = u64::from_le_bytes(data[offset..offset + 8].try_into()?);
    offset += 8;

    let coin_creator = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();

    pool_log(
        "SUCCESS",
        &format!(
            "Parsed PumpFun: base={}, quote={}, lp_supply={}",
            base_mint,
            quote_mint,
            lp_supply
        )
    );

    Ok(PoolData {
        pool_type: PoolType::PumpfunAmm,
        token_a: TokenInfo {
            mint: base_mint.to_string(),
            decimals: 6, // Token decimals (assumed)
        },
        token_b: TokenInfo {
            mint: quote_mint.to_string(),
            decimals: 9, // SOL decimals
        },
        reserve_a: ReserveInfo {
            vault_address: pool_base_token_account.to_string(),
            balance: 0, // Not available in this structure
        },
        reserve_b: ReserveInfo {
            vault_address: pool_quote_token_account.to_string(),
            balance: 0, // Not available in this structure
        },
        specific_data: PoolSpecificData::PumpfunAmm {
            pool_bump,
            index,
            creator: creator.to_string(),
            lp_mint: lp_mint.to_string(),
            lp_supply,
            coin_creator: coin_creator.to_string(),
        },
    })
}
