//! PumpFun v2 “CPMM” pool decoder
//! Program id: 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P
//!
//! ✅  Extracts the on-chain “real” reserves that form the price.
//!
//! Account layout (after Anchor’s 8-byte discriminator):
//!   0..  8  virtual_token_reserves  (u64)   – EMA / TWAP helper, ignore for spot
//!   8.. 16  virtual_sol_reserves    (u64)
//!  16.. 24  real_token_reserves     (u64)   – we use this
//!  24.. 32  real_sol_reserves       (u64)   – we use this
//!  32.. 40  token_total_supply      (u64)   – LP supply, not used for price
//!  40      complete                (u8 )    – 0 = trading, 1 = frozen/complete
//!  41.. 73  creator                 (Pubkey)
//!
//! The pool **does not store the token-mint pubkey** on account; you must
//! know it from context.  We therefore return `Pubkey::default()` for the
//! base-mint and `So111…` for SOL so callers can still compute price.

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use solana_sdk::account::Account;

/// Returns `(real_token_reserves, real_sol_reserves, base_mint, quote_mint)`
pub fn decode_pumpfun2_pool(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account,
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    
    if acct.data.len() < 73 {
        return Err(anyhow!(
            "PumpFun2 account {} too short: {} B (< 73)",
            pool_pk,
            acct.data.len()
        ));
    }

    // skip Anchor discriminator (8 bytes)
    let data = &acct.data[8..];

    let real_token_reserves = u64::from_le_bytes(data[16..24].try_into()?);
    let real_sol_reserves   = u64::from_le_bytes(data[24..32].try_into()?);

    // The pool doesn’t store its token-mint; return default / wrapped-SOL.
    let base_mint  = Pubkey::default();
    let quote_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")
        .expect("hard-coded So111… pubkey");

    Ok((real_token_reserves, real_sol_reserves, base_mint, quote_mint))
}
