use screenerbot::pool::decoders::*;
use screenerbot::pool::types::PoolType;
use screenerbot::rpc::RpcManager;
use screenerbot::config::RpcConfig;
use std::sync::Arc;

#[tokio::test]
async fn test_raydium_decoders() {
    // Create RPC manager
    let rpc_config = RpcConfig::default();
    let rpc_manager = Arc::new(
        RpcManager::new(
            "https://api.mainnet-beta.solana.com".to_string(),
            vec![],
            rpc_config
        ).unwrap()
    );

    // Test all decoders are available
    let decoders = DecoderFactory::create_all(rpc_manager.clone());

    // Should have 7 decoders: MeteoraDynamic, PumpFunAmm, and 5 Raydium decoders
    assert_eq!(decoders.len(), 7);

    // Test specific pool types
    let raydium_amm_v4 = DecoderFactory::create_for_type(
        PoolType::RaydiumAmmV4,
        rpc_manager.clone()
    );
    assert!(raydium_amm_v4.is_some());

    let raydium_amm_v5 = DecoderFactory::create_for_type(
        PoolType::RaydiumAmmV5,
        rpc_manager.clone()
    );
    assert!(raydium_amm_v5.is_some());

    let raydium_clmm = DecoderFactory::create_for_type(PoolType::RaydiumClmm, rpc_manager.clone());
    assert!(raydium_clmm.is_some());

    let raydium_cpmm = DecoderFactory::create_for_type(PoolType::RaydiumCpmm, rpc_manager.clone());
    assert!(raydium_cpmm.is_some());

    let raydium_stable_swap = DecoderFactory::create_for_type(
        PoolType::RaydiumStableSwap,
        rpc_manager.clone()
    );
    assert!(raydium_stable_swap.is_some());

    // Test program IDs
    let raydium_amm_v4_decoder = RaydiumAmmV4Decoder::new(rpc_manager.clone());
    assert_eq!(
        raydium_amm_v4_decoder.program_id().to_string(),
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
    );

    let raydium_clmm_decoder = RaydiumClmmDecoder::new(rpc_manager.clone());
    assert_eq!(
        raydium_clmm_decoder.program_id().to_string(),
        "CAMMCzo5YL8w4VFF8KVHrK22GGUQzXMVCaRz9qfUAEA"
    );

    let raydium_cpmm_decoder = RaydiumCpmmDecoder::new(rpc_manager.clone());
    assert_eq!(
        raydium_cpmm_decoder.program_id().to_string(),
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"
    );

    println!("✅ All Raydium decoders are working correctly!");
}

#[tokio::test]
async fn test_pool_types() {
    // Test PoolType display
    assert_eq!(PoolType::RaydiumAmmV4.to_string(), "RaydiumAmmV4");
    assert_eq!(PoolType::RaydiumAmmV5.to_string(), "RaydiumAmmV5");
    assert_eq!(PoolType::RaydiumClmm.to_string(), "RaydiumClmm");
    assert_eq!(PoolType::RaydiumCpmm.to_string(), "RaydiumCpmm");
    assert_eq!(PoolType::RaydiumStableSwap.to_string(), "RaydiumStableSwap");

    // Test PoolType from string
    assert_eq!(PoolType::from("RaydiumAmmV4"), PoolType::RaydiumAmmV4);
    assert_eq!(PoolType::from("RaydiumAmmV5"), PoolType::RaydiumAmmV5);
    assert_eq!(PoolType::from("RaydiumClmm"), PoolType::RaydiumClmm);
    assert_eq!(PoolType::from("RaydiumCpmm"), PoolType::RaydiumCpmm);
    assert_eq!(PoolType::from("RaydiumStableSwap"), PoolType::RaydiumStableSwap);

    println!("✅ All PoolType conversions are working correctly!");
}
