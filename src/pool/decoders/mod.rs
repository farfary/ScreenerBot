pub mod pumpfun;
pub mod raydium_amm;
pub mod raydium_clmm;
pub mod orca_whirlpool;
pub mod universal;
pub mod utils;

use crate::pool::types::*;
use anyhow::Result;
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

pub use pumpfun::PumpFunDecoder;
pub use raydium_amm::RaydiumAmmDecoder;
pub use raydium_clmm::RaydiumClmmDecoder;
pub use orca_whirlpool::OrcaWhirlpoolDecoder;
pub use universal::UniversalPoolDecoder;

/// Trait for decoding pool data from different DEX protocols
#[async_trait]
pub trait PoolDecoder: Send + Sync {
    /// Get the program ID for this pool type
    fn program_id(&self) -> Pubkey;

    /// Check if the given account data matches this pool type
    fn can_decode(&self, account_data: &[u8]) -> bool;

    /// Decode pool information from account data
    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo>;

    /// Decode current reserves from account data
    async fn decode_pool_reserves(
        &self,
        pool_address: &str,
        account_data: &[u8],
        slot: u64
    ) -> Result<PoolReserve>;
}

/// Factory for creating pool decoders
pub struct DecoderFactory;

impl DecoderFactory {
    /// Create all available decoders
    pub fn create_all(rpc_manager: Arc<crate::rpc::RpcManager>) -> Vec<Box<dyn PoolDecoder>> {
        vec![
            Box::new(PumpFunDecoder::new(rpc_manager.clone())),
            Box::new(RaydiumAmmDecoder::new(rpc_manager.clone())),
            Box::new(RaydiumClmmDecoder::new(rpc_manager.clone())),
            Box::new(OrcaWhirlpoolDecoder::new(rpc_manager.clone()))
        ]
    }

    /// Create decoder for specific pool type
    pub fn create_for_type(
        pool_type: PoolType,
        rpc_manager: Arc<crate::rpc::RpcManager>
    ) -> Option<Box<dyn PoolDecoder>> {
        match pool_type {
            PoolType::PumpFunAmm => Some(Box::new(PumpFunDecoder::new(rpc_manager))),
            PoolType::RaydiumAmmV4 => Some(Box::new(RaydiumAmmDecoder::new(rpc_manager))),
            PoolType::RaydiumClmm => Some(Box::new(RaydiumClmmDecoder::new(rpc_manager))),
            PoolType::OrcaWhirlpool => Some(Box::new(OrcaWhirlpoolDecoder::new(rpc_manager))),
            _ => None,
        }
    }

    /// Find decoder that can handle the given account data
    pub fn find_decoder(
        account_data: &[u8],
        rpc_manager: Arc<crate::rpc::RpcManager>
    ) -> Option<Box<dyn PoolDecoder>> {
        let decoders = Self::create_all(rpc_manager);

        for decoder in decoders {
            if decoder.can_decode(account_data) {
                return Some(decoder);
            }
        }

        None
    }

    /// Find decoder by program ID
    pub fn find_decoder_by_program_id(
        program_id: &Pubkey,
        rpc_manager: Arc<crate::rpc::RpcManager>
    ) -> Option<Box<dyn PoolDecoder>> {
        let decoders = Self::create_all(rpc_manager);

        for decoder in decoders {
            if decoder.program_id() == *program_id {
                return Some(decoder);
            }
        }

        None
    }
}
