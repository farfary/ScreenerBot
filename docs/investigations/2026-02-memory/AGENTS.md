# ScreenerBot Memory Optimization — Agent Strategy Guide

> **Issue**: Memory/RAM growing to gigabytes during 24/7 operation
> **Date Started**: 2026-02-18
> **Status**: Investigation complete (v12), ready for implementation
> **Plan Document**: [PLAN.md](./PLAN.md) (~5,590 lines)
> **Public Copy**: Also in public repo at `ScreenerBot/docs/investigations/2026-02-memory/`

---

## Investigation Summary

### What We Found
- Bot RSS starts at **804 MB** and grows unboundedly to 2+ GB
- 7 root causes identified (SQLite page caches #1 at ~580MB)
- 14 SQLite databases with misconfigured cache_size and mmap_size
- 171K tokens in filtering snapshot consuming ~240MB
- 3 true memory leaks + 2 slow leaks + unbounded caches
- macOS system allocator fragmentation (~200MB waste)

### Architecture Designed (10 Components)
1. Right-Sized SQLite Configuration (~580MB → ~84MB)
2. Incremental Filtering (eliminate 238MB reload every 3 min)
3. TokenListEntry lightweight struct (240MB → ~103MB in snapshot)
4. Bounded Caches via moka (stop all unbounded growth)
5. Automatic Maintenance Service (self-maintaining databases)
6. jemalloc Allocator (reduce fragmentation ~100-200MB)
7. Memory Pressure Response (OOM prevention)
8. SQLite Configuration Standardization (shared function)
9. SQL Pre-Filtering (optional, further reduces token load)
10. User & Automatic Tuning (3-layer cascade: auto/profile/manual)

### Expected Results
- RSS at startup: 804 MB → 150-250 MB
- RSS after 24h: 2+ GB → 200-350 MB
- Memory growth: Unbounded → Fully bounded
- Maintenance: Manual → Automatic

---

## Agent Strategy for Implementation

### Phase A — Foundation (LOW RISK, biggest impact)
**Agent type**: `backend` or `general-purpose`
**Approach**: Single agent per step, sequential

| Step | Agent | Scope | Notes |
|------|-------|-------|-------|
| A1. Create database/common.rs | backend | 2 new files | DbConnectionConfig + configure_sqlite_connection() |
| A2. Migrate 13 SQLite pools | backend | 13 files | Mechanical: replace PRAGMAs with with_init() |
| A3. Right-size pool max_size | backend | Same 13 files | Can combine with A2 in same commit |
| A4. Add jemalloc | backend | 2 files | Cargo.toml + main.rs, 3 lines |
| A5. Wire cleanup_stats() | backend | 1 file | rpc_stats_service.rs |
| A6-A7. Config sections | backend | 4 files | Must use config_struct! macro |
| A8. auto_vacuum PRAGMA | backend | 1 file | Add to shared function |
| A9. VERIFY | task | Build + run | `cargo build --release`, measure RSS |

**Key constraints**:
- All config must use `config_struct!` macro (src/config/macros.rs)
- New modules registered in src/lib.rs (lines 3-42)
- with_init() pattern from r2d2_sqlite 0.31
- Pool presets: Hot (tokens, transactions), Standard (events, positions), Cold (strategies, ai)

### Phase B — Bounded Caches (LOW-MEDIUM RISK)
**Agent type**: `backend` per cache migration
**Approach**: Can parallelize independent cache migrations

| Step | Agent | Scope | Notes |
|------|-------|-------|-------|
| B1. Add moka dep | backend | Cargo.toml | `moka = { version = "0.12", features = ["sync"] }` |
| B2-B4. Big caches → moka | backend | 3 files | DECIMALS, TOKEN_2022, SIGNATURES |
| B5-B6. Leak fixes | backend | 3 files | POSITION_LOCKS, ACTIVE_ACTIONS |
| B8-B15. Remaining caches | backend | 8 files | Can split across agents |

**Key constraints**:
- moka .get() returns cloned value (not reference) — use Arc<T> for large values
- moka max_capacity fixed at creation — NO runtime resize
- Must call run_pending_tasks() periodically for timely eviction
- Profile values from PerformanceConfig resolve_profile()

### Phase C1 — TokenListEntry (MEDIUM RISK, simpler than originally planned)
**Agent type**: `backend` (single agent, careful work)
**Approach**: Sequential, verify after each file

| Step | Agent | Scope | Notes |
|------|-------|-------|-------|
| C1. Add TokenListEntry to types.rs | backend | 1 file | ~45 fields, ~550 bytes |
| C2. Update engine.rs | backend | 1 file | Convert Token → TokenListEntry after evaluation |
| C3. Update store.rs | backend | 1 file | collect/filter/sort on &TokenListEntry |
| C4. Update API types | backend | 2 files | TokenListResponse, tokens list.rs |
| C5. Update Telegram | backend | 1 file | callbacks.rs query_tokens() consumer |

**CRITICAL**: Filter sources (dexscreener.rs, geckoterminal.rs, etc.) do NOT change!
They use transient full Token from DB during evaluation. Only snapshot output changes.

### Phase C2 — Incremental Filtering (MEDIUM-HIGH RISK)
**Agent type**: `backend` (experienced, careful)
**Approach**: Sequential with heavy testing

### Phase D — Maintenance Service (MEDIUM RISK)
**Agent type**: `backend`
**Approach**: Create service first, add tasks incrementally

### Phase E — Observability (LOW-MEDIUM RISK)
**Agent type**: `front` for dashboard, `backend` for API endpoints

---

## Research Documents in This Folder

### Core Research (read first)
- `PLAN.md` — THE master document (5,228 lines, v11)
- `RESEARCH_INDEX.md` — Index of all research topics
- `RESEARCH_COMPLETION_REPORT.md` — Final research status
- `RESEARCH_SOURCES.md` — All sources consulted

### By Topic

**Arc/Atomic Reference Counting**:
- `00_ARC_RESEARCH_START_HERE.md` → Start here
- `ARC_ARCHITECTURE_DETAILS.md`, `ARC_COMPREHENSIVE_RESEARCH.md`
- `ARC_CACHE_ALIGNMENT_RESEARCH.md`, `ARC_DOCUMENTATION_ANALYSIS.md`
- `ARC_PRACTICAL_EXAMPLES.rs`, `ARC_SOURCE_DOCUMENTS.md`
- `FINAL_ARC_RESEARCH_SUMMARY.md`

**Async/SQLite**:
- `ASYNC_RUSQLITE_RESEARCH.md`, `ASYNC_RUSQLITE_ARCHITECTURE.md`
- `ASYNC_RUSQLITE_QUICK_REFERENCE.md`, `ASYNC_RESEARCH_SUMMARY.md`
- `RUSQLITE_COMPREHENSIVE_RESEARCH.md`

**Epoch-Based Memory Reclamation**:
- `EPOCH_RESEARCH_START_HERE.md` → Start here
- `EPOCH_BASED_MEMORY_RECLAMATION_DEEP_DIVE.md`
- `START_HERE_EPOCH_MASTERFILE.md`, `MASTERFILE_SUMMARY.md`
- `DEEP_DIVE_QUICK_REFERENCE.txt`

**DashMap/Concurrent Data Structures**:
- `DASHMAP_RESEARCH.md`, `DASHMAP_MEMORY_RESEARCH.md`
- `dashmap_README.md`, `dashmap_lib.rs`
- `HASHBROWN_MEMORY_ANALYSIS.md`
- `CROSSBEAM_UTILS_DOCUMENTATION.md`
- `FLURRY_DOCUMENTATION_EXTRACT.md`, `flurry_docs.html`
- `EVMAP_RESEARCH_SUMMARY.md`

**Moka Cache**:
- `MOKA_MEMORY_RESEARCH.md`, `MOKA_BENCHMARKS.md`
- `MOKA_VS_MINIMOKA_COMPREHENSIVE_RESEARCH.md`
- `MOKA_VS_MINI_MOKA_RESEARCH.md`
- `moka_api_summary.md`, `moka_docs.html`, `moka_high_value_issues.md`

**Other**:
- `frequency_sketch.rs` — TinyLFU frequency sketch implementation
- `deque.rs` — Double-ended queue implementation
- `implementation_guide.md` — General implementation guidance
- Various `.json` files — GitHub search results for research

---

## Doc Archiving Strategy

**Rule**: When completing a major investigation or issue, archive all related research documents:

1. Create folder: `docs/{YYYYMMDD}-issue-{short-description}/`
   - Date = when investigation started
   - Description = kebab-case issue name (e.g., `memory`, `security-audit`, `telegram-refactor`)

2. Move ALL research files into the folder

3. Include these standard files:
   - `PLAN.md` — Full implementation plan (copy from session state)
   - `AGENTS.md` — Agent strategy guide for implementation

4. Commit with message: `docs: archive {issue} research and plan`

This keeps the repo root clean while preserving all research for future reference.
