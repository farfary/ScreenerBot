use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use crate::rpc::RpcManager;
use crate::pool::decoders::utils;

const PUMP_FUN_PROGRAM: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

pub struct PumpFunAmmDecoder {
    rpc_manager: RpcManager,
    program_id: Pubkey,
}

impl PumpFunAmmDecoder {
    pub fn new(rpc_manager: RpcManager) -> Self {
        Self {
            rpc_manager,
            program_id: Pubkey::from_str(PUMP_FUN_PROGRAM).unwrap(),
        }
    }

    /// Get real-time price from Pump.fun bonding curve
    pub async fn get_real_time_price(&self, pool_address: &str) -> Result<f64> {
        println!("🔍 DEBUG: Getting real-time price for pool {}", pool_address);

        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let account_data = self.rpc_manager.get_account_data(&pool_pubkey).await?;

        if account_data.len() < 99 {
            return Err(
                anyhow::anyhow!("Pool account data too small: {} bytes", account_data.len())
            );
        }

        // Extract mint addresses from the pool data
        let base_mint = utils::bytes_to_pubkey(&account_data[35..67]);
        let quote_mint = utils::bytes_to_pubkey(&account_data[67..99]);

        println!("🔍 DEBUG: base_mint: {}", base_mint);
        println!("🔍 DEBUG: quote_mint: {}", quote_mint);

        // For Pump.fun, we need to access the bonding curve accounts
        // The bonding curve holds the actual reserves
        let (bonding_curve, _) = Pubkey::find_program_address(
            &[b"bonding-curve", base_mint.as_ref()],
            &self.program_id
        );

        println!("🔍 DEBUG: bonding_curve: {}", bonding_curve);

        // Get the bonding curve account data
        match self.rpc_manager.get_account_data(&bonding_curve).await {
            Ok(curve_data) => {
                if curve_data.len() >= 72 {
                    // Try to parse bonding curve data
                    let virtual_token_reserves = utils::bytes_to_u64(&curve_data[8..16]);
                    let virtual_sol_reserves = utils::bytes_to_u64(&curve_data[16..24]);
                    let real_token_reserves = utils::bytes_to_u64(&curve_data[24..32]);
                    let real_sol_reserves = utils::bytes_to_u64(&curve_data[32..40]);

                    println!("🔍 DEBUG: virtual_token_reserves: {}", virtual_token_reserves);
                    println!("🔍 DEBUG: virtual_sol_reserves: {}", virtual_sol_reserves);
                    println!("🔍 DEBUG: real_token_reserves: {}", real_token_reserves);
                    println!("🔍 DEBUG: real_sol_reserves: {}", real_sol_reserves);

                    // Use real reserves if available, otherwise virtual
                    let token_reserves = if real_token_reserves > 0 {
                        real_token_reserves
                    } else {
                        virtual_token_reserves
                    };
                    let sol_reserves = if real_sol_reserves > 0 {
                        real_sol_reserves
                    } else {
                        virtual_sol_reserves
                    };

                    if token_reserves > 0 && sol_reserves > 0 {
                        // Calculate price as SOL per token
                        let price =
                            (sol_reserves as f64) /
                            1_000_000_000.0 /
                            ((token_reserves as f64) / 1_000_000.0);
                        println!("🎯 SUCCESS: Calculated price from bonding curve: {}", price);
                        return Ok(price);
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Failed to get bonding curve data: {}", e);
            }
        }

        // Fallback: try to get associated token accounts
        let (associated_bonding_curve, _) = Pubkey::find_program_address(
            &[b"associated-bonding-curve", base_mint.as_ref()],
            &self.program_id
        );

        println!("🔍 DEBUG: associated_bonding_curve: {}", associated_bonding_curve);

        let token_balance = self
            .get_token_account_balance(&associated_bonding_curve).await
            .unwrap_or(0);

        if token_balance > 0 {
            // Simple price calculation fallback
            let price = 0.0001; // Default minimal price
            println!("🔍 DEBUG: Using fallback price: {}", price);
            return Ok(price);
        }

        println!("⚠️  Cannot calculate price - no reserves found");
        Ok(0.0)
    }

    /// Get token account balance
    async fn get_token_account_balance(&self, account: &Pubkey) -> Result<u64> {
        println!("🔍 DEBUG: Getting token account balance for {}", account);

        match self.rpc_manager.get_account_data(account).await {
            Ok(data) => {
                if data.len() >= 72 {
                    // Parse SPL token account data
                    let amount = utils::bytes_to_u64(&data[64..72]);
                    println!("🔍 DEBUG: Found balance: {}", amount);
                    Ok(amount)
                } else {
                    println!("⚠️  Account data too small: {} bytes", data.len());
                    Ok(0)
                }
            }
            Err(e) => {
                println!("⚠️  Failed to get token account balance for {}: {}", account, e);
                Ok(0)
            }
        }
    }

    /// Decode pool information
    pub async fn decode_pool_info(&self, pool_address: &str) -> Result<(String, String, f64)> {
        println!("🔍 DEBUG: Decoding Pump.fun AMM pool {}", pool_address);

        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let account_data = self.rpc_manager.get_account_data(&pool_pubkey).await?;

        println!("🔍 DEBUG: Account data length: {} bytes", account_data.len());

        if account_data.len() < 99 {
            return Err(
                anyhow::anyhow!("Pool account data too small: {} bytes", account_data.len())
            );
        }

        // Debug first 64 bytes
        let first_64: Vec<String> = account_data[0..64]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        println!("🔍 DEBUG: First 64 bytes: {}", first_64.join(" "));

        // Extract mint addresses
        let base_mint = utils::bytes_to_pubkey(&account_data[35..67]);
        let quote_mint = utils::bytes_to_pubkey(&account_data[67..99]);

        println!("🔍 DEBUG: base_mint: {}", base_mint);
        println!("🔍 DEBUG: quote_mint: {}", quote_mint);

        // Get real-time price
        let price = self.get_real_time_price(pool_address).await.unwrap_or(0.0);

        println!(
            "🔍 DEBUG: Successfully decoded pool info for {} with price: {}",
            pool_address,
            price
        );

        Ok((base_mint.to_string(), quote_mint.to_string(), price))
    }

    /// Decode pool reserves
    pub async fn decode_pool_reserves(&self, pool_address: &str) -> Result<(u64, u64)> {
        println!("🔍 DEBUG: Decoding pool reserves for {}", pool_address);

        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let account_data = self.rpc_manager.get_account_data(&pool_pubkey).await?;

        if account_data.len() < 99 {
            return Err(anyhow::anyhow!("Pool account data too small"));
        }

        let base_mint = utils::bytes_to_pubkey(&account_data[35..67]);

        // Get bonding curve account
        let (bonding_curve, _) = Pubkey::find_program_address(
            &[b"bonding-curve", base_mint.as_ref()],
            &self.program_id
        );

        match self.rpc_manager.get_account_data(&bonding_curve).await {
            Ok(curve_data) => {
                if curve_data.len() >= 40 {
                    let real_token_reserves = utils::bytes_to_u64(&curve_data[24..32]);
                    let real_sol_reserves = utils::bytes_to_u64(&curve_data[32..40]);

                    if real_token_reserves > 0 && real_sol_reserves > 0 {
                        return Ok((real_token_reserves, real_sol_reserves));
                    }

                    // Fallback to virtual reserves
                    let virtual_token_reserves = utils::bytes_to_u64(&curve_data[8..16]);
                    let virtual_sol_reserves = utils::bytes_to_u64(&curve_data[16..24]);

                    return Ok((virtual_token_reserves, virtual_sol_reserves));
                }
            }
            Err(_) => {
                // Fallback to zero reserves
                return Ok((0, 0));
            }
        }

        Ok((0, 0))
    }

    /// Get base mint address
    pub async fn get_base_mint(&self, pool_address: &str) -> Result<Pubkey> {
        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let account_data = self.rpc_manager.get_account_data(&pool_pubkey).await?;

        if account_data.len() < 67 {
            return Err(anyhow::anyhow!("Pool account data too small"));
        }

        Ok(utils::bytes_to_pubkey(&account_data[35..67]))
    }

    /// Get quote mint address
    pub async fn get_quote_mint(&self, pool_address: &str) -> Result<Pubkey> {
        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let account_data = self.rpc_manager.get_account_data(&pool_pubkey).await?;

        if account_data.len() < 99 {
            return Err(anyhow::anyhow!("Pool account data too small"));
        }

        Ok(utils::bytes_to_pubkey(&account_data[67..99]))
    }
}
