# Investigation: "00" Token Symbol — Scam Factory Analysis

**Date:** 2026-02-21  
**Status:** ✅ CONFIRMED — Not a bug, real on-chain scam factory  
**Impact:** Dashboard shows scam tokens at top of "All Tokens" sorted by liquidity

## Summary

1,033+ tokens in our database have the symbol **"00"**. Deep investigation confirmed these are **real on-chain scam tokens** created by a single scam factory entity, not a bug in ScreenerBot's metadata handling.

## Evidence

### 1. On-Chain Metadata Verification

The symbol "00" is literally stored in Metaplex Token Metadata on Arweave/Irys:

```json
// https://arweave.net/QfwVttGp2LTTLwWN3ybGVaJsG0Ud0r0LF0p85rIGEoQ
{
  "name": "USDT",
  "symbol": "00",
  "image": "https://gateway.irys.xyz/n0ze1dkTZMsimaPfMxs5P0VleIo1Z1WOsEfrsogkC2o"
}
```

All checked metadata URIs returned `"symbol": "00"` — this is permanent, immutable on-chain data set by the token creator.

### 2. Scam Factory Pattern

| Attribute | Value |
|-----------|-------|
| **Shared freeze authority** | `9N2kn1C8sYM3PrTJ4DY5q7R4uaLXVkrc8C23JR1e6pWW` |
| **Shared update authority** | `4wTRxzhv8HZZPW6YgrPcrZEwtDTC4RvKjKzZVHbzGAxL` |
| **Supply pattern** | 99,999,999,999 tokens (100B with 6 decimals) |
| **Metadata** | Immutable (mutable=False) |
| **Token count** | 1,033 in our DB alone |

13 of 20 sampled "00" tokens share the **exact same** freeze authority — one entity mass-creating scam tokens.

### 3. RugCheck Risk Profile

| Risk | Level |
|------|-------|
| Freeze authority enabled | 🔴 Danger |
| LP unlocked | 🔴 Danger |
| Top holder owns 76% | 🔴 Danger |
| Top 10 hold >70% | 🔴 Danger |
| Single holder high ownership | 🔴 Danger |
| Low liquidity (real) | 🔴 Danger |
| Risk score | 68K-71K (normalised ~75 = high risk) |

### 4. Fake Liquidity ($40M+ Reserve, $0 Volume)

The most deceptive aspect: GeckoTerminal reports these pools as having **$40M+ reserves**.

**Example pool:** `2PU8ftjVSwhdxNUxnerefGcWM5mRDzyrWDf7UB9g4ham` (Orca Whirlpool)

| Metric | Value |
|--------|-------|
| Reserve (GeckoTerminal) | $40,407,473 |
| FDV | $1,022,479,568 |
| Volume 24h | **$0.00** |
| Transactions 24h | **0 buys, 0 sells** |
| Created | Dec 11, 2025 (2+ months, dead) |
| DexScreener | **NOT FOUND** (correctly filtered) |

**How fake liquidity works:**
1. Scammer creates token with 100B supply
2. Deposits tokens + small real USDC into Orca Whirlpool at extreme price ratio
3. Pool reports "reserve" based on token price × supply (circular valuation)
4. GeckoTerminal ingests this as real data
5. Our bot picks up the inflated liquidity via GeckoTerminal
6. Tokens rank #1 in "All Tokens" sorted by liquidity

### 5. DexScreener vs GeckoTerminal

- **DexScreener:** Returns NOT FOUND for all 5 tested "00" tokens → correctly filters scams
- **GeckoTerminal:** Confirms `symbol: "00"` with inflated market data → does NOT filter scams
- **RugCheck:** Correctly flags all risks (freeze, LP, holders)

### 6. Scam Name Impersonation

The factory creates tokens impersonating legitimate projects:
- "USDT" (fake Tether)
- "World Liberty Financial USD" (fake Trump/DeFi project)
- "United States Crypto Reserve" (fake government)
- "Scoutly AI", "NOVA AI" (AI hype bait)
- "Samuel the Baby Turtle", "CowAlon" (meme bait)
- "SHITCOIN" (at least honest)

### 7. pump.fun "terminal00" Tokens (Separate Pattern)

Some "00" tokens end in "pump" (pump.fun tokens):
- **No** freeze authority (pump.fun standard)
- Supply: 1B (pump.fun standard)
- Likely experiments/vanity tokens, not the same scam factory

## Database Statistics

| Metric | Value |
|--------|-------|
| Total "00" tokens | 1,033 |
| Unique names for "00" | 287+ (plus many duplicates) |
| Empty symbol tokens | 13,066 |
| NULL symbol tokens | 15,909 |
| Total unique symbols | 82,724 |
| "UNKNOWN" fallback | 302 |

## ScreenerBot Code Analysis

### Metadata is Correct
- Our metadata fetcher correctly stores on-chain data
- Default fallback is `"UNKNOWN"`, not `"00"`
- No code path could produce "00" as an artifact

### Filtering Gaps Found

| Gap | Status | Impact |
|-----|--------|--------|
| Zero volume + high liquidity detection | ❌ Missing | Fake liquidity tokens pass filters |
| Numeric-only symbol validation | ❌ Missing | "00" symbols not flagged |
| Scam wallet clustering | ❌ Missing | Same factory not detected |
| Freeze authority filter | ✅ Exists | But may be disabled in config |
| Volume threshold | ✅ Exists | But bypassed when threshold ≤ 0 |

## Conclusion

**NOT A BUG.** ScreenerBot correctly reads and displays on-chain metadata. The "00" symbol is intentionally set by a mass scam token factory using Metaplex metadata. The tokens appear prominent because:

1. GeckoTerminal provides inflated liquidity data from fake Orca pools
2. Dashboard "All Tokens" sorts by liquidity descending
3. Filtering doesn't currently detect the fake-liquidity pattern (high liquidity + zero volume)

## Recommendations (Future Phases)

1. **Fake liquidity filter:** Reject tokens where `liquidity > $X` but `volume_24h < $Y`
2. **Numeric symbol filter:** Flag tokens with purely numeric symbols ≤2 chars
3. **Scam wallet database:** Track known scam freeze/update authorities
4. **Volume sanity check:** Require minimum ratio of volume-to-liquidity
5. **DexScreener cross-validation:** If DexScreener doesn't list a token but GeckoTerminal shows high liquidity, flag it

## Technical References

- Scam freeze authority: `9N2kn1C8sYM3PrTJ4DY5q7R4uaLXVkrc8C23JR1e6pWW`
- Scam update authority: `4wTRxzhv8HZZPW6YgrPcrZEwtDTC4RvKjKzZVHbzGAxL`
- Example metadata: `https://arweave.net/QfwVttGp2LTTLwWN3ybGVaJsG0Ud0r0LF0p85rIGEoQ`
- Example pool: `2PU8ftjVSwhdxNUxnerefGcWM5mRDzyrWDf7UB9g4ham`
