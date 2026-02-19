# Logging Level Audit & Fixes - October 25, 2025

## Summary

Comprehensive audit and correction of logging levels in the pool decoder and calculation modules. All raw data and per-token diagnostic logs have been correctly categorized according to the logging standards:

- **INFO**: User-facing events, state changes, system readiness (not changed)
- **DEBUG**: Processing details, RPC operations, fetcher/calculator steps, internal flow
- **VERBOSE**: Raw data dumps, per-token calculations, detailed diagnostics (new level usage)

## Changes Made

### 1. **pumpfun_legacy.rs** - Line 216

**Change**: `logger::info()` → `logger::debug()`
**Reason**: Price calculation result display is a diagnostic detail, not user-facing
**Log**: "PumpFun bonding curve price calculated: pool={}, token={}, price={:.15} SOL/token"

### 2. **raydium_legacy_amm.rs** - Line 320

**Change**: `logger::info()` → `logger::debug()`
**Reason**: Vault adjustment is internal processing detail
**Log**: "Adjusted vaults: coin_vault={} pc_vault={}"

### 3. **raydium_legacy_amm.rs** - Line 358

**Change**: `logger::info()` → `logger::debug()`
**Reason**: Pool data reserve extraction is diagnostic processing
**Log**: "Found pool data reserves at offsets {} and {}: {} and {}"

### 4. **pumpfun_amm.rs** - Line 216

**Change**: `logger::debug()` → `logger::verbose()`
**Reason**: Raw reserve amounts are per-token raw data requiring `--verbose` flag
**Log**: "Raw reserves - Token: {}, SOL: {}"

### 5. **pumpfun_amm.rs** - Line 270

**Change**: `logger::debug()` → `logger::verbose()`
**Reason**: Detailed price calculation breakdown (reserves, decimals, adjusted values) is raw diagnostic data
**Log**: "PumpFun price calculation: - SOL Reserve: {} (decimals: {}, adjusted: {:.12})..."
**Content**:

- SOL Reserve with decimals and adjusted value
- Token Reserve with decimals and adjusted value
- Calculated price per token

### 6. **pumpfun_amm.rs** - Line 328

**Change**: `logger::debug()` → `logger::verbose()`
**Reason**: Per-vault balance amounts are raw account data
**Log**: "Vault {} balance: {}"

### 7. **fluxbeam_amm.rs** - Line 116

**Change**: `logger::debug()` → `logger::verbose()`
**Reason**: Raw vault balances are per-token diagnostic data
**Log**: "FluxBeam vault balances: SOL={}, token={}"

### 8. **fluxbeam_amm.rs** - Line 172

**Change**: `logger::debug()` → `logger::verbose()`
**Reason**: Detailed price calculation is raw diagnostic data
**Log**: "FluxBeam price calculation: {:.12} SOL per token (sol_reserves={:.6}, token_reserves={:.6})"

### 9. **raydium_clmm.rs** - Line 109

**Change**: `logger::debug()` → `logger::verbose()`
**Reason**: Raw vault balances are per-token diagnostic data
**Log**: "CLMM vault balances: SOL={}, token={}, is_token_0={}"

### 10. **raydium_clmm.rs** - Line 184

**Change**: `logger::debug()` → `logger::verbose()`
**Reason**: Detailed price calculation is raw diagnostic data
**Log**: "CLMM price calculation: {:.12} SOL per token (sol_reserves={:.6}, token_reserves={:.6})"

### 11. **raydium_cpmm.rs** - Line 342

**Change**: `logger::debug()` → `logger::verbose()`
**Reason**: Detailed price calculation breakdown (reserves, decimals, adjusted values) is raw diagnostic data
**Log**: "Raydium CPMM Price Calculation for {}:\n... SOL Reserve: {} ({:.9} adjusted, {} decimals)..."
**Content**:

- SOL Reserve with decimals and adjusted value
- Token Reserve with decimals and adjusted value
- Calculated price per token

## Files Modified

1. `src/pools/decoders/pumpfun_legacy.rs` (1 change)
2. `src/pools/decoders/raydium_legacy_amm.rs` (2 changes)
3. `src/pools/decoders/pumpfun_amm.rs` (3 changes)
4. `src/pools/decoders/fluxbeam_amm.rs` (2 changes)
5. `src/pools/decoders/raydium_clmm.rs` (2 changes)
6. `src/pools/decoders/raydium_cpmm.rs` (1 change)

**Total**: 11 logging level corrections

## Logging Level Standards Applied

### DEBUG (Processing Details)

- Vault adjustments and corrections
- Pool data extraction steps
- Account state transitions
- Filter/validation steps within calculations

Examples:

- "Adjusted vaults: coin_vault={} pc_vault={}"
- "Found pool data reserves at offsets {} and {}: {} and {}"
- "PumpFun Legacy: Processing for base={} quote={}"

### VERBOSE (Raw Data)

- Raw reserve amounts (lamports/wei units)
- Per-token vault balances
- Decimal adjustment details
- Complete price calculation breakdowns with intermediate values
- Account-level diagnostic dumps

Examples:

- "Raw reserves - Token: {}, SOL: {}" (raw lamports)
- "Vault {} balance: {}" (raw token amount)
- "PumpFun price calculation: SOL Reserve: {} (decimals: {}, adjusted: {:.12})"
- "CLMM price calculation: {:.12} SOL per token (sol_reserves={:.6}, token_reserves={:.6})"

## Visibility Changes

These logs are now only visible when explicitly requested:

### Before (DEBUG level - default visibility):

```bash
cargo run --bin screenerbot -- --run
# Shows all DEBUG logs by default - NOISY
```

### After (VERBOSE level - explicit flag):

```bash
cargo run --bin screenerbot -- --run --verbose-pool-decoder
# Shows raw data only when needed for deep diagnostics
```

## Testing Performed

✅ **Compilation**: `cargo check --lib` - All changes compile without errors
✅ **Logging Standards Compliance**: All logs follow the three-tier structure (INFO/DEBUG/VERBOSE)
✅ **Decoder Coverage**: All 11 decoder files reviewed for consistency

## Impact

- **Noise Reduction**: Default logging is now cleaner without raw per-token diagnostics
- **Debuggability**: Raw data still available via `--verbose` flag for deep investigation
- **Performance**: No performance impact - only logging level metadata changes
- **Compatibility**: No API changes, fully backward compatible

## How to Use

### View normal operations (INFO + DEBUG):

```bash
cargo run --bin screenerbot -- --run --debug-pool-decoder
```

### Deep dive with raw data (INFO + DEBUG + VERBOSE):

```bash
cargo run --bin screenerbot -- --run --verbose-pool-decoder
```

### All modules verbose:

```bash
cargo run --bin screenerbot -- --run --verbose
```

## Future Considerations

1. Other modules (tokens, filtering, swaps) may have similar per-token raw data that should be VERBOSE
2. Consider adding `--verbose-<module>` flags for other subsystems
3. Raw RPC response bodies in `rpc.rs` might benefit from VERBOSE categorization
4. Transaction event logs might need similar audit
