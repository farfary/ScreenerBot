/// Pool price calculator
/// Calculates token prices from pool data

use crate::pools::types::{ PriceResult, PoolInfo };
use crate::pools::decoders::{ DecoderFactory, PoolDecodedResult };
use crate::pools::service::{ PoolService, PreparedPoolData };

/// Pool price calculator
pub struct PoolCalculator {
    decoder_factory: DecoderFactory,
}

impl PoolCalculator {
    pub fn new() -> Self {
        Self {
            decoder_factory: DecoderFactory::new(),
        }
    }

    /// Calculate price for a token from decoded pool data
    pub async fn calculate_price_from_decoded_pool(
        &self,
        service: &PoolService,
        pool_address: &str,
        program_id: &str,
        reserve_addresses: &[String],
        token_mint: &str
    ) -> Result<Option<PriceResult>, String> {
        // Get prepared pool data from service
        let prepared_data = service.prepare_pool_data(
            pool_address,
            program_id,
            reserve_addresses
        ).await?;

        // Get appropriate decoder
        let decoder = self.decoder_factory
            .get_decoder(program_id)
            .ok_or_else(|| format!("No decoder found for program ID: {}", program_id))?;

        // Decode pool data
        let decoded_result = decoder.decode_pool_data(&prepared_data)?;

        // Calculate price from decoded result
        self.calculate_price_from_decoded_result(&decoded_result, token_mint)
    }

    /// Calculate price from PoolDecodedResult
    fn calculate_price_from_decoded_result(
        &self,
        decoded: &PoolDecodedResult,
        token_mint: &str
    ) -> Result<Option<PriceResult>, String> {
        // Find which token in the pair matches the requested token
        let (token_reserves, sol_reserves, token_decimals, sol_decimals) = if
            decoded.token_a_mint == token_mint
        {
            // Token A is the target token, Token B should be SOL
            if decoded.token_b_mint == crate::pools::constants::SOL_MINT {
                (
                    decoded.token_a_reserve,
                    decoded.token_b_reserve,
                    decoded.token_a_decimals,
                    decoded.token_b_decimals,
                )
            } else {
                return Err(format!("Pool does not contain SOL pair for token {}", token_mint));
            }
        } else if decoded.token_b_mint == token_mint {
            // Token B is the target token, Token A should be SOL
            if decoded.token_a_mint == crate::pools::constants::SOL_MINT {
                (
                    decoded.token_b_reserve,
                    decoded.token_a_reserve,
                    decoded.token_b_decimals,
                    decoded.token_a_decimals,
                )
            } else {
                return Err(format!("Pool does not contain SOL pair for token {}", token_mint));
            }
        } else {
            return Err(format!("Token {} not found in pool", token_mint));
        };

        if token_reserves == 0 || sol_reserves == 0 {
            return Ok(None);
        }

        // Convert raw reserves to decimal values
        let token_reserve_decimal = (token_reserves as f64) / (10_f64).powi(token_decimals as i32);
        let sol_reserve_decimal = (sol_reserves as f64) / (10_f64).powi(sol_decimals as i32);

        if token_reserve_decimal > 0.0 && sol_reserve_decimal > 0.0 {
            let price_sol = sol_reserve_decimal / token_reserve_decimal;

            let result = PriceResult::new(
                token_mint.to_string(),
                price_sol,
                sol_reserve_decimal,
                token_reserve_decimal,
                decoded.pool_address.clone(),
                decoded.program_id.clone()
            );

            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Calculate price for a token from pool data
    pub async fn calculate_price(
        &self,
        pool: &PoolInfo,
        token: &str
    ) -> Result<Option<PriceResult>, String> {
        // Basic price calculation from pool reserves
        if pool.token_reserve > 0.0 && pool.sol_reserve > 0.0 {
            let price_sol = pool.sol_reserve / pool.token_reserve;

            let result = PriceResult::new(
                token.to_string(),
                price_sol,
                pool.sol_reserve,
                pool.token_reserve,
                pool.pool_address.clone(),
                pool.program_id.clone()
            );

            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Calculate prices for multiple tokens
    pub async fn batch_calculate(&self, pools: &[(PoolInfo, String)]) -> Vec<Option<PriceResult>> {
        let mut results = Vec::new();

        for (pool, token) in pools {
            match self.calculate_price(pool, token).await {
                Ok(result) => results.push(result),
                Err(_) => results.push(None),
            }
        }

        results
    }
}
