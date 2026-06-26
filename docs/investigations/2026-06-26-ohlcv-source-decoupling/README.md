# 2026-06-26 — OHLCV Source Decoupling

## Summary

Decoupled OHLCV's API source configuration from `[tokens.sources.*]` /
`[tokens.discovery.*]` and moved it into a new `[ohlcv.sources.*]` block.
The shared `ApiManager.geckoterminal` client now stays on if EITHER side
(discovery OR OHLCV) needs it. Endpoint URLs are config-driven; the
hardcoded `BASE_URL` constant in `apis/solana_tracker/mod.rs` has been
replaced with a per-instance `base_url` field.

## Problem

The 2026-06-26 10:02 run produced **264 OHLCV errors and 194 warnings in
12 minutes**, all of the form:

```
[OHLCV] [ERROR] API error: GeckoTerminal client disabled via configuration
[OHLCV] [ERROR] (endpoint=networks/solana/pools/.../ohlcv/minute)
[OHLCV] [WARNING] Backfill failed for mint=...: API error: GeckoTerminal client
```

Root cause was in `src/apis/manager.rs:48-49`:

```rust
let geckoterminal_enabled =
    geckoterminal_cfg.enabled        // [tokens.sources.geckoterminal].enabled = true
    && discovery_enabled              // [tokens.discovery].enabled = true
    && discovery_cfg.geckoterminal.enabled;  // [tokens.discovery.geckoterminal].enabled = FALSE
```

The user's `config.toml` had `[tokens.discovery.geckoterminal].enabled = false`
because they didn't want discovery pulling from GeckoTerminal. But the shared
GeckoTerminal client is ALSO used by the OHLCV fetcher, which doesn't depend
on discovery at all — turning off discovery silently disabled OHLCV fetches.

A secondary issue: `SolanaTrackerClient::BASE_URL` was hardcoded in code with
no config override, and `SolanaTrackerSourceConfig` lived under
`[tokens.sources.solana_tracker]` despite SolanaTracker being OHLCV-only.

## Files Changed

| File | Change |
|---|---|
| `src/config/schemas/ohlcv.rs` | Added `OhlcvSourcesConfig`, `OhlcvGeckoConfig`, `OhlcvSolanaTrackerConfig` structs + `sources` field on `OhlcvConfig` |
| `src/config/schemas/tokens.rs` | Removed `SolanaTrackerSourceConfig` struct and `solana_tracker` field from `TokenSourcesConfig`; added `endpoint` field to `SourceApiConfig` (default empty string) |
| `src/apis/geckoterminal/mod.rs` | Added `base_url: String` field + `with_base_url` constructor; `new()` now delegates with the default URL |
| `src/apis/geckoterminal/endpoints.rs` | Replaced `GECKOTERMINAL_BASE_URL` literals (13 sites) with `self.base_url`; dropped now-unused import |
| `src/apis/solana_tracker/mod.rs` | Added `base_url: String` field + `with_base_url` constructor; replaced `BASE_URL` literals with `self.base_url` |
| `src/apis/manager.rs` | If-any gate: `geckoterminal_enabled = sources_enabled && (discovery_needs_gt || ohlcv_needs_gt)`; endpoint derived from whichever side consumes the client; SolanaTracker settings now read from `cfg.ohlcv.sources.solana_tracker` |
| `src/ohlcvs/monitor.rs` | Rate-limit calc reads `cfg.ohlcv.sources.geckoterminal.*` instead of `cfg.tokens.sources.geckoterminal.*` |
| `src/webserver/routes/config/types.rs` | Added `("ohlcv", &["sources.solana_tracker.api_key"])` to `SENSITIVE_FIELDS` |
| `docs/architecture/ohlcvs.md` | Updated §13 Configuration with full field list + new `ohlcv.sources` subsection |

## Backward Compatibility

### Breaking changes for existing `config.toml`

If a user had `[tokens.sources.solana_tracker]` populated in their config,
those values are now ignored — the loader silently drops unknown sections
(verified by `config::load_config()` reading only known fields). The user
must move the values to `[ohlcv.sources.solana_tracker]` to keep
SolanaTracker working as an OHLCV fallback.

### Backward-compatible additions

- `[tokens.sources.geckoterminal]` gains an optional `endpoint` field
  (defaults to empty = use code default). No-op for existing configs.
- `[ohlcv.sources]` is entirely new — defaults to GeckoTerminal enabled,
  SolanaTracker disabled. Existing OHLCV behavior is preserved.

## Verification

- `cargo check --lib` — clean
- `cargo check --all-targets` — clean
- `cargo build` — clean
- Headless run with `[tokens.discovery.geckoterminal].enabled = false`:
  - **0 OHLCV errors** in a 70-second window
  - Previously: 264 errors / 12 min
  - `[OHLCV] [INFO] OHLCV monitor starting: rate_limit=30/min, delay=2100ms between tokens` confirms the new config path is read
  - Remaining `[WARNING] Rate limit exceeded` and `[WARNING] No healthy pools available` are correct, expected behavior under load (real 429s from GeckoTerminal, not the client-disabled error)

## Architectural Decisions

### Decision 1: If-any gate vs separate client instance

Considered creating a second `GeckoTerminalClient` instance owned by OHLCV
instead of sharing `ApiManager.geckoterminal`. Rejected because:
- The shared client already has correct rate limiting + stats tracking
- Two clients would split the rate-limit budget (30/min × 2) and trip 429s
- The if-any gate is a 4-line change vs ~80 LOC for a second client path

### Decision 2: Move or copy SolanaTracker config?

SolanaTracker is exclusively used by OHLCV (verified via grep — only
`apis/manager.rs` and `ohlcvs/*` reference it). Moved it entirely
rather than keeping a legacy alias — alias support would be confusing
and the field is undocumented for general users.

### Decision 3: Endpoint as empty string vs always required

`SourceApiConfig.endpoint` defaults to `""` (empty string) and the
client falls back to the hardcoded default. This means existing configs
keep working without changes — the endpoint field is opt-in for users
who want to override (e.g. for proxies or private instances).