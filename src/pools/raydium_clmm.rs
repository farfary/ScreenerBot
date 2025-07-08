//! Raydium CLMM v1  (program pAMMBa… & CAMMCz…)
use anyhow::{ anyhow, Result };
use num_format::{ Locale, ToFormattedString };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::account::Account;

/// Returns (reserve_coin, reserve_pc, coin_mint, pc_mint)
pub fn decode_raydium_clmm(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    if acct.data.len() < 211 {
        return Err(anyhow!("Pump.fun account only {} B (<211)", acct.data.len()));
    }
    if acct.data.len() < 216 {
        println!("⚠️  CLMM account too short");
        return Ok((0, 0, Pubkey::default(), Pubkey::default()));
    }

    // Extract mint addresses from pool account
    let coin_mint = Pubkey::new_from_array(acct.data[72..104].try_into()?);
    let pc_mint = Pubkey::new_from_array(acct.data[104..136].try_into()?);

    let coin = u64::from_le_bytes(acct.data[200..208].try_into()?);
    let pc = u64::from_le_bytes(acct.data[208..216].try_into()?);

    println!(
        "✅ Raydium CLMM  → Coin: {} | PC: {}",
        coin.to_formatted_string(&Locale::en),
        pc.to_formatted_string(&Locale::en)
    );
    Ok((coin, pc, coin_mint, pc_mint))
}

/// Batch-friendly version: decode from already-fetched account
pub fn decode_raydium_clmm_from_account(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    // Same logic as decode_raydium_clmm, but account is already provided
    decode_raydium_clmm(rpc, pool_pk, acct)
}
