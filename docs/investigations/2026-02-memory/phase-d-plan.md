# Phase D — Hardening & Configurability

**Status: 📋 PLANNED**
**Priority: Medium** — ≤400 MB target already met; this phase hardens gains and fixes correctness risks
**Estimated Impact: Low memory savings, high reliability and code quality**

## Context

Phases A+B+C reduced RSS from 1,011 MB → 371 MB (62%), meeting the ≤400 MB target. Phase D is not about further optimization — it's about:

1. **Correctness**: Hardcoded values that could silently break user workflows
2. **Long-term stability**: Missing maintenance operations (WAL checkpoint)
3. **Documentation accuracy**: PLAN.md no longer reflects reality
4. **Verification**: Longer test to confirm stability beyond 10 minutes

## Problem Statement

### P1: Stale Token Cutoff Hardcoded (Correctness Risk)
The 7-day stale token cutoff (`assembly.rs:216`) is the most impactful parameter in the system:
- Controls 91% of token loading (172K → 15.6K)
- Buried as `7 * 24 * 60 * 60` magic number
- If a user tracks dormant tokens (>7 days without market data update), they silently disappear from the filter
- No way to adjust without recompilation
- **Must become a config parameter**

### P2: WAL Checkpoint Missing (Stability Risk)
`maintenance.rs` handles auto-vacuum but NOT WAL checkpoints:
- SQLite WAL files grow unbounded without periodic checkpoints
- 13 databases, all write-active, running 24/7
- `wal_checkpoint_interval_secs` exists in config but is never used
- Risk: WAL files growing to hundreds of MB over days/weeks

### P3: Vacuum Interval Hardcoded
`maintenance.rs:304` has `6 * 60 * 60` (6 hours) hardcoded:
- Config already has `vacuum_interval_secs` but it's not read
- Should wire config values to the maintenance task

### P4: PLAN.md Inaccurate
PLAN.md now claims Phase C implemented "TokenListEntry + incremental filtering" — this is FALSE.
What we actually did:
- Stale token SQL filter (WHERE clause, not a new struct)
- Bounded 2 more caches (API_RESPONSE_CACHE, FAILED_CACHE → moka)
- Database auto-vacuum maintenance module
- jemalloc tuning documentation

The plan should be corrected to reflect reality for anyone reading it.

### P5: No Long-Duration Test
10-minute test confirmed ≤400 MB target. But:
- No 1-hour+ test to verify stability
- No verification that maintenance task works across multiple vacuum cycles
- No data on memory behavior under sustained load
- rpc_stats.db (250 MB, auto_vacuum=0) will be migrated on next startup — untested

---

## Tasks

### D1: Make Stale Token Cutoff Configurable
**Priority: HIGH** — Correctness risk
- Add `stale_token_days` to `[filtering]` config section (default: 7)
- Read from config in `assembly.rs` instead of hardcoded `7 * 24 * 60 * 60`
- Document the parameter in config schema and AGENTS.md
- Consider: exclude tokens in open positions from staleness filter (always visible)

Files: `src/config/schemas/mod.rs`, `src/tokens/database/assembly.rs`, `src/tokens/database/async_api.rs`

### D2: Add WAL Checkpoint to Maintenance
**Priority: MEDIUM** — Long-term stability
- Add `run_wal_checkpoint()` function to `maintenance.rs`
- Uses `PRAGMA wal_checkpoint(TRUNCATE)` — resets WAL file
- Run on configurable interval (default: 1 hour, from config `wal_checkpoint_interval_secs`)
- Add to the maintenance task loop (separate cycle from vacuum)

Files: `src/database/maintenance.rs`

### D3: Wire Config Values to Maintenance Task
**Priority: MEDIUM** — Currently hardcoded intervals
- Read `vacuum_interval_secs` from config (default: 21600 = 6h)
- Read `wal_checkpoint_interval_secs` from config (default: 3600 = 1h)
- Pass to `start_maintenance_task()` instead of hardcoded values
- Validate config: minimum intervals (vacuum: 1h, wal: 5min)

Files: `src/database/maintenance.rs`, `src/run.rs`

### D4: Fix PLAN.md Accuracy
**Priority: MEDIUM** — Documentation debt
- Correct Phase C description to match reality (stale filter, not TokenListEntry)
- Note TokenListEntry and incremental filtering as "NOT IMPLEMENTED — bypassed by stale filter"
- Update estimated savings to match actual measurements
- Add "Actual vs Planned" comparison section

Files: `docs/investigations/2026-02-memory/PLAN.md`

### D5: 1-Hour Stability Test
**Priority: MEDIUM** — Verification
- Release build, run for 60+ minutes
- Monitor RSS every 10 seconds (360+ samples)
- Verify maintenance task runs at least one vacuum cycle
- Verify rpc_stats.db gets migrated to INCREMENTAL
- Check WAL file sizes don't grow unboundedly
- Document results

### D6: Code Review & Documentation Update
**Priority: LOW** — Maintenance
- Review all Phase D changes
- Update AGENTS.md with new config parameters
- Update phase-d-summary.md with results
- Update Assistant-instructions if needed

---

## What We're NOT Doing (Deferred Indefinitely)

| Item | Why Deferred |
|------|-------------|
| TokenListEntry (~550 byte struct) | Only saves ~23 MB now (15.6K tokens). ROI too low. |
| Incremental/delta filtering | Peak already manageable (483 MB). Complex with low payoff. |
| SQL pre-filtering | ~14 MB savings. Not worth the complexity. |
| Memory pressure detection (Phase E) | Nice-to-have monitoring. No memory impact. |
| Telegram pressure notifications | Luxury feature. |
| `/api/system/memory` endpoint | Useful but not critical. Can add later. |
| screenerbot-manager CLI | Separate project scope. |

---

## Dependencies

```
D1 (stale cutoff config) — independent, can start immediately
D2 (WAL checkpoint) — independent, can start immediately  
D3 (wire config) — depends on D2 (WAL function must exist)
D4 (fix PLAN.md) — independent, can do anytime
D5 (stability test) — depends on D1+D2+D3 (test with all fixes)
D6 (review + docs) — depends on D1-D5
```

## Success Criteria

- [ ] Stale token cutoff is configurable via `filtering.stale_token_days`
- [ ] WAL checkpoint runs periodically (verified in logs)
- [ ] Maintenance intervals come from config, not hardcoded
- [ ] PLAN.md accurately describes what was implemented
- [ ] 1-hour test shows stable RSS ≤400 MB with no WAL growth
- [ ] All changes documented
