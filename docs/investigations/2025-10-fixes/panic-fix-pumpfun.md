# PumpFun Legacy Decoder Panic Fix - October 25, 2025

## Problem

The bot was panicking with this error:

```
thread 'tokio-runtime-worker' panicked at src/pools/decoders/pumpfun_legacy.rs:173:38:
byte index 8 is out of bounds of ``
```

## Root Cause

**Location**: `src/pools/decoders/pumpfun_legacy.rs` line 173 (and line 342)

**Issue**: The code was attempting to slice strings without checking if they were empty:

```rust
&pair_info.token_mint[..8],  // PANIC if token_mint is empty string!
```

**Why Empty Strings?**: The `analyze_token_pair()` function returns a `TokenPairInfo` struct with empty strings for `token_mint`, `token_vault`, and `sol_vault` when the pool is NOT a valid SOL pair. This happens when:

1. Neither mint is SOL (e.g., TOKEN/USDC pair)
2. Both mints are SOL variants
3. One mint is a stablecoin (USDC, USDT)

The `TokenPairInfo::invalid()` constructor creates a struct with:

```rust
token_mint: String::new(),  // Empty string!
sol_mint: SOL_MINT.to_string(),
token_vault: String::new(),  // Empty string!
sol_vault: String::new(),  // Empty string!
is_sol_pair: false,  // This flag indicates it's invalid
```

**The Bug**: The decoder never checked `is_sol_pair` before attempting to slice the strings for logging.

## Solution Applied

### 1. Updated `decode_pump_fun_legacy_pool()` Function

**File**: `src/pools/decoders/pumpfun_legacy.rs` lines 130-195

**Changes**:

- Added `pool_account: &str` parameter to function signature for better error logging
- Added validation check AFTER calling `analyze_token_pair()`:
  ```rust
  if !pair_info.is_sol_pair {
      logger::error(
          LogTag::PoolDecoder,
          &format!(
              "PumpFun Legacy pool {} is NOT a valid SOL pair - mint1={}, mint2={}, vault1={}, vault2={}",
              pool_account, base_mint, quote_mint, vault1, vault2
          ),
      );
      return None;
  }
  ```
- Only attempts string slicing AFTER validation passes

### 2. Updated `extract_reserve_accounts()` Function

**File**: `src/pools/decoders/pumpfun_legacy.rs` lines 335-354

**Changes**:

- Added same validation check before using vault strings:
  ```rust
  if !pair_info.is_sol_pair {
      logger::error(
          LogTag::PoolDecoder,
          "PumpFun Legacy extract_reserve_accounts: pool is NOT a valid SOL pair, cannot extract vaults",
      );
      return None;
  }
  ```

### 3. Updated Caller in `decode_and_calculate()`

**File**: `src/pools/decoders/pumpfun_legacy.rs` line 54

**Changes**:

- Updated call to pass pool_account for logging:
  ```rust
  if let Some(pool_info) = Self::decode_pump_fun_legacy_pool(&pool_data.data, pool_account) {
  ```

## What Will Happen Now

When the bot encounters a PumpFun Legacy pool that is NOT a valid SOL pair, you will see this **ERROR** log:

```
[POOLDEC   ] [ERROR] PumpFun Legacy pool <POOL_ADDRESS> is NOT a valid SOL pair -
                     mint1=<FULL_MINT1_ADDRESS>,
                     mint2=<FULL_MINT2_ADDRESS>,
                     vault1=<FULL_VAULT1_ADDRESS>,
                     vault2=<FULL_VAULT2_ADDRESS>
```

This will show:

- **Exact pool address** that caused the issue
- **Both mint addresses** (full, not truncated)
- **Both vault addresses** (full, not truncated)
- **No panic** - graceful error handling with detailed logging

## Testing Instructions

1. Run the bot normally:

   ```bash
   cargo run --bin screenerbot
   ```

2. Watch for the ERROR log above - it will identify the problematic pool

3. The bot should **NOT panic** anymore - it will skip invalid pools gracefully

4. After identifying the pool, you can investigate:
   - Why DexScreener/discovery returned a non-SOL PumpFun pool
   - Whether the pool structure is correct
   - Whether we need to add support for non-SOL PumpFun pools

## Related Files

- `src/pools/decoders/pumpfun_legacy.rs` - Fixed decoder
- `src/pools/utils.rs` - Contains `analyze_token_pair()` and `TokenPairInfo`
- `src/pools/decoders/pumpfun_amm.rs` - Already had proper validation

## Verification Status

✅ Code compiles successfully (`cargo check --lib`)
✅ PumpFun AMM decoder already uses `validate_sol_pool()` - no issues
✅ Other decoders don't use `analyze_token_pair()` directly - no issues
✅ Two functions in `pumpfun_legacy.rs` fixed

## Notes

- The PumpFun AMM decoder doesn't have this issue because it uses `validate_sol_pool()` which returns a `Result` and properly handles validation before any string operations
- This pattern should be followed in all decoders: **validate before slicing**
- The `is_sol_pair` flag in `TokenPairInfo` is the canonical way to check validity
