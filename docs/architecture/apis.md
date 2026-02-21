# APIs Module — Architecture

> ScreenerBot External API Clients — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [API Manager](#3-api-manager)
4. [Client Implementations](#4-client-implementations)
5. [Rate Limiting](#5-rate-limiting)
6. [Caching Strategy](#6-caching-strategy)
7. [Module Connections](#7-module-connections)

---

## 1. Overview

The APIs module manages all external (non-RPC) API clients: DexScreener, GeckoTerminal, Rugcheck, Jupiter, CoinGecko, and DefiLlama. Each client has independent rate limiting, error handling, and response parsing.

**Key characteristics:**
- Singleton `ApiManager` aggregating all clients
- Per-client and per-endpoint rate limiting
- Typed response parsing (serde)
- Fallback chains (e.g., price from DexScreener → GeckoTerminal → CoinGecko)
- moka caches for frequently accessed data

**41 files, ~10,422 lines**

---

## 2. File Structure

```
src/apis/
├── mod.rs              # Module declarations, ApiManager
├── manager.rs          # ApiManager singleton
├── dexscreener/        # DexScreener client (10 endpoints)
│   ├── client.rs
│   ├── types.rs
│   └── rate_limiter.rs # Per-endpoint rate limiting
├── geckoterminal/      # GeckoTerminal client (12 endpoints)
│   ├── client.rs
│   └── types.rs
├── rugcheck/           # Rugcheck client (4 endpoints)
│   ├── client.rs
│   └── types.rs
├── jupiter/            # Jupiter client (token list, prices)
│   ├── client.rs
│   └── types.rs
├── coingecko/          # CoinGecko client (price, market data)
│   ├── client.rs
│   └── types.rs
├── defillama/          # DefiLlama client (TVL, protocol data)
│   ├── client.rs
│   └── types.rs
└── helpers.rs          # Shared HTTP utilities
```

---

## 3. API Manager

```rust
pub struct ApiManager {
    pub dexscreener: DexScreenerClient,
    pub geckoterminal: GeckoTerminalClient,
    pub rugcheck: RugcheckClient,
    pub jupiter: JupiterClient,
    pub coingecko: CoinGeckoClient,
    pub defillama: DefiLlamaClient,
}
```

Global singleton accessed via:
```rust
pub fn api_manager() -> &'static ApiManager
```

---

## 4. Client Implementations

### DexScreener

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `get_token_profiles()` | GET `/token-profiles/latest/v1` | Latest token profiles |
| `get_token_boosts()` | GET `/token-boosts/latest/v1` | Boosted tokens |
| `get_pair(chain, address)` | GET `/latest/dex/pairs/{chain}/{address}` | Single pair data |
| `get_pairs_by_token(mint)` | GET `/latest/dex/tokens/{mint}` | All pairs for token |
| `search_pairs(query)` | GET `/latest/dex/search?q={query}` | Search pairs |
| `get_orders(mint)` | GET `/orders/v1/solana/{mint}` | Token orders |
| `get_token_info(mint)` | GET `/token-profiles/v1/solana/{mint}` | Token metadata |

**Rate limits:** 300 req/min global, per-endpoint limits for burst-prone endpoints.

### GeckoTerminal

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `get_trending_pools(network)` | Trending pools | Market discovery |
| `get_new_pools(network)` | New pools | Early detection |
| `get_pool(network, address)` | Pool details | Price/volume data |
| `get_pools_multi(addresses)` | Multiple pools | Batch lookup |
| `get_token(network, address)` | Token info | Metadata |
| `get_token_pools(address)` | Token's pools | Pool discovery |
| `get_ohlcv(pool, timeframe)` | OHLCV candles | Chart data |
| `get_trades(pool)` | Recent trades | Trade history |
| `search(query)` | Search | Token/pool search |
| `get_top_pools(network)` | Top pools | Volume leaders |
| `get_network_info(network)` | Network info | Chain metadata |
| `get_token_info(address)` | Extended token info | Full metadata |

**Rate limit:** 30 req/min.

### Rugcheck

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `get_token_report(mint)` | GET `/v1/tokens/{mint}/report` | Risk assessment |
| `get_token_report_summary(mint)` | GET `/v1/tokens/{mint}/report/summary` | Quick risk score |
| `get_token_locks(mint)` | GET `/v1/tokens/{mint}/locks` | LP lock status |
| `get_recent_scams()` | GET `/v1/stats/recent-scams` | Known scam list |

**Rate limit:** 60 req/min.

### Jupiter

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `get_token_list()` | GET `/tokens` | All listed tokens |
| `get_price(mints)` | GET `/price?ids=...` | Token prices (batch) |

**Rate limit:** Unlimited (free tier) or configurable API key tier.

### CoinGecko

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `get_sol_price()` | SOL price in USD | Portfolio valuation |
| `get_token_price(ids)` | Token prices | Cross-reference |

### DefiLlama

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `get_protocols()` | Protocol data | TVL comparison |
| `get_token_prices(addresses)` | Token prices | Another price source |

---

## 5. Rate Limiting

### DexScreener (most complex)

Per-endpoint rate limiters using GCRA:

```rust
pub struct DexScreenerRateLimiter {
    global: Governor,              // 300/min
    search: Governor,              // 20/min (search is expensive)
    token: Governor,               // 60/min
    pair: Governor,                // 120/min
}
```

### Other Clients

Simple per-client rate limiters:

| Client | Rate Limit | Implementation |
|--------|-----------|----------------|
| GeckoTerminal | 30/min | Governor |
| Rugcheck | 60/min | Governor |
| Jupiter | Unlimited / tier-based | Optional Governor |
| CoinGecko | 10-50/min (plan dependent) | Governor |
| DefiLlama | 50/min | Governor |

---

## 6. Stats & Tracking

The APIs module includes `ApiStatsTracker` for monitoring API usage and performance per client. No response caching is done within the APIs module itself — caching is handled by caller modules (tokens, filtering, pools) using their own moka caches.

---

## 7. Module Connections

```
apis/
├── config/         ← API keys, rate limit settings
├── rpc/            ← NOT used (apis handles HTTP only)
├── errors/         ← Error types
└── tokens/cache    ← Shares data with token cache
```

| Caller | Client | Purpose |
|--------|--------|---------|
| tokens | DexScreener, GeckoTerminal, Jupiter | Metadata enrichment |
| filtering | Rugcheck, DexScreener, GeckoTerminal | Risk assessment |
| pools | DexScreener, GeckoTerminal | Pool discovery, prices |
| ohlcvs | GeckoTerminal | OHLCV candle data |
| positions | DexScreener | Current price for P&L |
| webserver | All | Dashboard data display |
| trader | DexScreener | Pre-trade price checks |
