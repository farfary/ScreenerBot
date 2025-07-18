# Pool Decoders Module

This module provides a comprehensive system for decoding and analyzing different types of Solana liquidity pools. It supports multiple pool types including Raydium CLMM, Meteora DLMM, and Whirlpools, allowing for price calculation and pool analysis directly from on-chain data.

## Overview

The decoders module is designed to:

- Decode raw pool account data from different DEX programs
- Calculate accurate prices from pool reserves and mathematical models
- Provide unified interfaces for different pool types
- Fetch live data from Solana RPC endpoints
- Analyze pool health and liquidity metrics

## Supported Pool Types

### 1. Raydium Concentrated Liquidity (CLMM)

- **Program ID**: `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK`
- **Features**: Concentrated liquidity with tick-based pricing
- **Price Calculation**: Uses sqrt_price_x64 format with tick current information

### 2. Meteora Dynamic Liquidity Market Maker (DLMM)

- **Program ID**: `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo`
- **Features**: Dynamic bin-based liquidity with volatility adjustments
- **Price Calculation**: Uses active bin ID and bin step for price determination

### 3. Orca Whirlpools

- **Program ID**: `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`
- **Features**: Concentrated liquidity similar to Uniswap V3
- **Price Calculation**: Uses sqrt_price and tick information

### 4. Pump.fun AMM

- **Program ID**: `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA`
- **Features**: Simple constant product AMM for meme tokens
- **Price Calculation**: Uses traditional x\*y=k formula with reserves

## Module Structure

```
src/pairs/decoders/
├── mod.rs              # Main module with decoder registry
├── types.rs            # Common types and error definitions
├── raydium_clmm.rs     # Raydium CLMM decoder implementation
├── meteora_dlmm.rs     # Meteora DLMM decoder implementation
├── whirlpool.rs        # Whirlpool decoder implementation
└── pump_fun_amm.rs     # Pump.fun AMM decoder implementation

src/pairs/
├── pool_fetcher.rs     # RPC-based pool data fetching
└── analyzer.rs         # Pool analysis and comparison tools
```

## Key Components

### DecoderRegistry

Central registry that manages all pool decoders and routes decoding requests to the appropriate decoder based on program ID.

```rust
let registry = DecoderRegistry::new();
let pool_info = registry.decode_pool(&program_id, &account_data)?;
let price = registry.calculate_price(&program_id, &pool_info)?;
```

### PoolDataFetcher

Fetches live pool data from Solana RPC endpoints and handles the complete data pipeline from raw account data to decoded pool information.

```rust
let fetcher = PoolDataFetcher::new(rpc_manager);
let pool_info = fetcher.fetch_pool_data(&pool_address).await?;
let price_info = fetcher.get_price_info(&pool_info)?;
```

### PoolAnalyzer

High-level analysis tools for comparing pools, calculating metrics, and health scores.

```rust
let analyzer = PoolAnalyzer::new(rpc_manager);
let analysis = analyzer.analyze_pool("pool_address").await?;
let comparisons = analyzer.compare_pools(pool_addresses).await?;
```

## Usage Examples

### Basic Pool Analysis

```rust
use screenerbot::pairs::{PoolAnalyzer, PoolDataFetcher};
use screenerbot::rpc::RpcManager;

// Initialize RPC manager
let rpc_manager = Arc::new(RpcManager::new(rpc_url, fallbacks, config)?);

// Create analyzer
let analyzer = PoolAnalyzer::new(rpc_manager);

// Analyze a specific pool
let analysis = analyzer.analyze_pool("8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj").await?;

println!("Pool Type: {:?}", analysis.pool_info.pool_type);
println!("Price: {:.6}", analysis.price_info.price);
println!("TVL: {:.2}", analysis.tvl);
println!("Health Score: {:.1}/100", analysis.health_score);
```

### Direct Pool Decoding

```rust
use screenerbot::pairs::decoders::{DecoderRegistry, program_ids};

// Get pool account data (from RPC or other source)
let account_data = rpc_client.get_account(&pool_address)?;

// Decode the pool
let registry = DecoderRegistry::new();
let pool_info = registry.decode_pool(&account_data.owner, &account_data.data)?;

// Calculate price
let price = registry.calculate_price(&account_data.owner, &pool_info)?;
```

### Pool Comparison

```rust
let pool_addresses = vec![
    "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj",
    "2QdhepnKRTLjjSqPL1PtKNwqrUkoLee5Gqs8bvZhRdMv",
];

let comparisons = analyzer.compare_pools(pool_addresses).await?;

for comparison in comparisons {
    println!("{} - Type: {:?}, TVL: {:.2}",
             comparison.address,
             comparison.pool_type,
             comparison.tvl);
}
```

## Demo Binary

A complete demo binary is provided to showcase the functionality:

```bash
# Analyze a single pool
cargo run --bin demo_pools analyze 8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj

# Compare multiple pools
cargo run --bin demo_pools compare ADDR1,ADDR2,ADDR3

# List supported pool types
cargo run --bin demo_pools supported
```

## Data Structures

### PoolInfo

Core data structure containing all decoded pool information:

```rust
pub struct PoolInfo {
    pub pool_address: Pubkey,
    pub program_id: Pubkey,
    pub pool_type: PoolType,
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    pub reserve_0: u64,
    pub reserve_1: u64,
    pub decimals_0: u8,
    pub decimals_1: u8,
    pub liquidity: Option<u128>,
    pub sqrt_price: Option<u128>,
    pub current_tick: Option<i32>,
    pub fee_rate: Option<u16>,
    pub status: PoolStatus,
    pub metadata: PoolMetadata,
}
```

### PriceInfo

Price calculation results with additional metadata:

```rust
pub struct PriceInfo {
    pub price: f64,
    pub inverted_price: f64,
    pub token_0_symbol: Option<String>,
    pub token_1_symbol: Option<String>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h_usd: Option<f64>,
}
```

## Price Calculation Methods

### 1. Sqrt Price (Raydium CLMM, Whirlpools)

- Uses Q64.64 fixed-point arithmetic
- Formula: `price = (sqrt_price / 2^64)^2`
- Adjusted for token decimals

### 2. Bin-based (Meteora DLMM)

- Uses active bin ID and bin step
- Formula: `price = (1 + bin_step/10000)^active_bin_id`
- Accounts for dynamic fee structure

### 3. Reserve-based (Pump.fun AMM, Fallback)

- Traditional AMM pricing
- Formula: `price = reserve_1 / reserve_0`
- Adjusted for decimals

## Error Handling

The module provides comprehensive error handling for:

- Invalid account data formats
- Missing required fields
- Zero liquidity conditions
- Network/RPC failures
- Unsupported pool types

## Performance Considerations

- **Batching**: Pool fetcher supports batch requests for multiple pools
- **Caching**: Token decimals and metadata can be cached
- **Fallback**: Multiple RPC endpoints for redundancy
- **Efficiency**: Minimal data parsing for better performance

## Future Enhancements

1. **Additional Pool Types**: Support for more DEX protocols
2. **Historical Data**: Price history and volume tracking
3. **Advanced Analytics**: Impermanent loss calculations, yield farming metrics
4. **Real-time Updates**: WebSocket-based live price feeds
5. **Cross-chain Support**: Multi-chain pool analysis

## Dependencies

The module relies on:

- `solana-sdk`: For Solana account and pubkey types
- `solana-client`: For RPC client functionality
- `anyhow`: For error handling
- `async-trait`: For async trait implementations
- `serde`: For serialization support

## Contributing

When adding support for new pool types:

1. Create a new decoder file in `src/pairs/decoders/`
2. Implement the `PoolDecoder` trait
3. Add the decoder to the registry in `mod.rs`
4. Update the `PoolType` enum in `types.rs`
5. Add tests and documentation

## Testing

```bash
# Run all tests
cargo test

# Test specific decoder
cargo test raydium_clmm

# Test with real pool data
cargo run --bin demo_pools supported
```
