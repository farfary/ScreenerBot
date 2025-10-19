# Tokens Module

This module centralizes token discovery, metadata aggregation, persistence, and caching for ScreenerBot. It replaces the legacy scattered token helpers with a single entry point that other subsystems consume via `crate::tokens::*` re-exports.

## Layout

- `mod.rs` – public surface that re-exports the key APIs.
- `types.rs` – canonical structs such as `Token`, `TokenMetadata`, and enums for sources and priorities.
- `store.rs` – in-memory cache of full `Token` objects backed by SQLite persistence.
- `storage/` – database wrapper, schema, and CRUD helpers.
- `cache/` – TTL-driven cache manager for API payloads (currently Rugcheck).
- `provider/` – high-level facade that coordinates API fetches, cache lookups, and DB writes.
- `discovery.rs` – pulls candidate mints from configured upstream sources.
- `service.rs` – orchestrator that wires the provider and background jobs for the services layer.
- `decimals.rs` – decimal lookup utilities with single-flight protection and RPC fallback.
- `blacklist.rs` – in-memory blacklist hydrated from persistent storage.
- `events.rs` – lightweight publish/subscribe hooks for token events.
- `security/` – pluggable security provider abstraction.

## Data Flow Overview

1. **Discovery** gathers mint candidates based on configuration toggles and emits `TokenEvent::TokenDiscovered` events.
2. **Provider** fetches data per mint. It wraps:
   - API calls via `ApiManager` with optional caching.
   - Persistence helpers that upsert metadata or Rugcheck payloads.
   - Store initialization (`store::initialize_with_database` + hydration).
3. **Store** maintains the in-memory source of truth for `Token` structs. Background tasks (e.g., decimals enforcement) are intended to call `store::set_decimals` or future `store::upsert_token` updates after fetches.
4. **Downstream consumers** access token data through re-exported helpers (`store::get_token`, `get_cached_decimals`, etc.) or subscribe to token events.

## Background Tasks

`TokensOrchestrator` (spawned by `TokensService`) manages two periodic jobs:

- **Discovery Loop** (20s cadence) – collects mints from configured APIs and dispatches fetches through the provider.
- **Decimals Loop** (5s cadence) – attempts to ensure mint decimals exist by delegating to `decimals::ensure`, which uses cache → DB → Rugcheck → on-chain RPC fallbacks.

Both loops respect the shared shutdown signal and are instrumented via `TaskMonitor` so service metrics can sample their workload.

## Persistence and Caching

- The module stores normalized metadata in `data/tokens.db` (managed via `storage::Database`). The schema currently tracks token identity fields, blacklist reasons, and Rugcheck payloads.
- `cache::CacheManager` holds JSON-serialized API responses with source-specific TTLs loaded from `config.tokens.sources.*.cache_ttl_seconds`.
- The in-memory `store` keeps cloned `Token` structs keyed by mint. It is the surface consumed by filtering, pools, trader, and wallet code.

## Extending the Module

- Add new database columns via `storage/schema.rs` and matching upsert/query helpers.
- Extend `provider::Fetcher` when onboarding additional data sources (DexScreener, GeckoTerminal, etc.), making sure to update the store and cache layers.
- Use `tokens::events::emit` to surface significant changes so other subsystems can react without direct coupling.
- Keep configuration centralized; fetch toggles and TTLs should originate from `config.tokens.*`.

## Current Gaps / Follow-ups

- `store::upsert_token` is defined but not yet wired into the fetch flow, so newly fetched tokens are not materialized in the in-memory store until a restart.
- `provider::fetch_complete_data` currently only integrates Rugcheck payloads; market data ingestion from DexScreener/GeckoTerminal is still pending.
- Decimal caching is seeded lazily; additional bootstrapping may be required so pool decoders reliably see decimals during startup.

These items should be addressed before relying on the tokens module as the sole authority for live trading decisions.
