//! Generic multipool decoder: Pump.fun, Raydium AMM / CLMM, Orca Whirlpool,
//! Meteora DLMM, generic CPMM – plus helpers for fetching pools & price.

use crate::pool_cpmm::decode_cpmm;
use crate::pool_meteora_dlmm::decode_meteora_dlmm;
use crate::pool_orca_whirlpool::decode_orca_whirlpool;
use crate::pool_pumpfun::{ decode_pumpfun_pool };
use crate::pool_raydium_amm::decode_raydium_amm;
use crate::pool_raydium_clmm::decode_raydium_clmm;
use crate::pool_raydium_cpmm::decode_raydium_cpmm;
use crate::pool_pumpfun2::decode_pumpfun2_pool;
use crate::pool_raydium_launchpad::decode_raydium_launchpad;

use crate::utilitis::get_token_decimals;

use anyhow::{ bail, Result };

use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

/// Main multipool decode entrypoint.
/// Only fetch account once, pass account data to decoders.
/// Signature is **unchanged**.
pub fn decode_any_pool(rpc: &RpcClient, pool_pk: &Pubkey) -> Result<(u64, u64, Pubkey, Pubkey)> {
    let acct = rpc.get_account(pool_pk)?; // fetch once
    let owner = acct.owner.to_string();

    match owner.as_str() {
        // Pump.fun (Raydium-CLMM v1)
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => decode_pumpfun_pool(rpc, pool_pk, &acct),
        // PumpFun v2 CPMM
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => decode_pumpfun2_pool(rpc, pool_pk, &acct),
        // Raydium CLMM v2
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => decode_raydium_clmm(rpc, pool_pk, &acct),
        // Raydium AMM v4
        "RVKd61ztZW9g2VZgPZrFYuXJcZ1t7xvaUo1NkL6MZ5w" => decode_raydium_amm(rpc, pool_pk, &acct),
        // Raydium CPMM
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => decode_raydium_cpmm(rpc, pool_pk, &acct),
        // Orca Whirlpool
        "whirLb9FtDwZ2Bi4FXe65aaPaJqmCj7QSfUeCrpuHgx" => decode_orca_whirlpool(rpc, pool_pk, &acct),

    // Raydium Launchpad
    "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj" =>
        decode_raydium_launchpad(rpc, pool_pk, &acct),
        // Meteora DLMM & DYN2 alias
        // | "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo"
        // | "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG" =>
        //     decode_meteora_dlmm(rpc, pool_pk, &acct),
        _ => bail!("Unsupported program id {} for pool {}", owner, pool_pk),
    }
}

/// Convenience: returns `(base, quote, price)` with `price = quote / base` (f64).
///
/// *Decimals are **not** adjusted;* scale if the two tokens have different decimals.

pub fn decode_any_pool_price(rpc: &RpcClient, pool_pk: &Pubkey) -> Result<(u64, u64, f64)> {
    // now returns (base_amt, quote_amt, base_mint, quote_mint)
    let (base_amt, quote_amt, base_mint, quote_mint) = decode_any_pool(rpc, pool_pk)?;

    if base_amt == 0 {
        bail!("base reserve is zero – cannot calculate price");
    }

    let base_dec = get_token_decimals(rpc, &base_mint)? as i32;
    let quote_dec = get_token_decimals(rpc, &quote_mint)? as i32;

    // price of **one whole base token** expressed in quote tokens
    let price = ((quote_amt as f64) / (base_amt as f64)) * (10f64).powi(base_dec - quote_dec);

    Ok((base_amt, quote_amt, price))
}
