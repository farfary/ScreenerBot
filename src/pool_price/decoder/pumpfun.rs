use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use super::super::types::*;
use crate::logger::{ log, LogTag };

/// Parse Pump.fun AMM pool data
pub fn parse_pumpfun_amm_data(data: &[u8]) -> Result<PumpfunAmmData> {
    log(LogTag::Pool, "DEBUG", &format!("Parsing Pump.fun AMM pool data, length: {}", data.len()));

    let mut offset = 8; // Skip discriminator

    // pool_bump (u8) - 1 byte
    if offset >= data.len() {
        return Err(anyhow::anyhow!("Data too short for pool_bump"));
    }
    let pool_bump = data[offset];
    offset += 1;

    // index (u16) - 2 bytes
    if offset + 2 > data.len() {
        return Err(anyhow::anyhow!("Data too short for index"));
    }
    let index = u16::from_le_bytes(data[offset..offset + 2].try_into()?);
    offset += 2;

    // creator (Pubkey) - 32 bytes
    if offset + 32 > data.len() {
        return Err(anyhow::anyhow!("Data too short for creator"));
    }
    let creator = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    // base_mint (Pubkey) - 32 bytes
    if offset + 32 > data.len() {
        return Err(anyhow::anyhow!("Data too short for base_mint"));
    }
    let base_mint = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    // quote_mint (Pubkey) - 32 bytes
    if offset + 32 > data.len() {
        return Err(anyhow::anyhow!("Data too short for quote_mint"));
    }
    let quote_mint = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    // lp_mint (Pubkey) - 32 bytes
    if offset + 32 > data.len() {
        return Err(anyhow::anyhow!("Data too short for lp_mint"));
    }
    let lp_mint = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();
    offset += 32;

    // pool_base_token_account (Pubkey) - 32 bytes
    if offset + 32 > data.len() {
        return Err(anyhow::anyhow!("Data too short for pool_base_token_account"));
    }
    let pool_base_token_account = Pubkey::new_from_array(
        data[offset..offset + 32].try_into()?
    ).to_string();
    offset += 32;

    // pool_quote_token_account (Pubkey) - 32 bytes
    if offset + 32 > data.len() {
        return Err(anyhow::anyhow!("Data too short for pool_quote_token_account"));
    }
    let pool_quote_token_account = Pubkey::new_from_array(
        data[offset..offset + 32].try_into()?
    ).to_string();
    offset += 32;

    // lp_supply (u64) - 8 bytes
    if offset + 8 > data.len() {
        return Err(anyhow::anyhow!("Data too short for lp_supply"));
    }
    let lp_supply = u64::from_le_bytes(data[offset..offset + 8].try_into()?);
    offset += 8;

    // coin_creator (Pubkey) - 32 bytes
    if offset + 32 > data.len() {
        return Err(anyhow::anyhow!("Data too short for coin_creator"));
    }
    let coin_creator = Pubkey::new_from_array(data[offset..offset + 32].try_into()?).to_string();

    log(
        LogTag::Pool,
        "DEBUG",
        &format!(
            "Parsed Pump.fun AMM data: pool_bump={}, index={}, creator={}, base_mint={}, quote_mint={}, lp_supply={}",
            pool_bump,
            index,
            creator,
            base_mint,
            quote_mint,
            lp_supply
        )
    );

    Ok(PumpfunAmmData {
        pool_bump,
        index,
        creator,
        base_mint,
        quote_mint,
        lp_mint,
        pool_base_token_account,
        pool_quote_token_account,
        lp_supply,
        coin_creator,
    })
}
