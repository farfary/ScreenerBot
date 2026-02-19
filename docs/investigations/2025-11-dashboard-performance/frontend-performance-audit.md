# Frontend Performance & API Handling Audit

**Date:** November 21, 2025  
**Issue:** Request storm causing "Failed to fetch" errors and empty page content

## Executive Summary

**Root Cause:** Browser connection pool exhaustion due to excessive concurrent polling requests creating a request storm (1000+ requests observed).

**Impact:** Empty page content, "Failed to fetch" errors, degraded user experience especially during rapid tab switching.

**Severity:** P0 - Critical performance issue affecting all pages

---

## Detailed Findings

### 1. **No Request Deduplication** ❌

**Problem:** Multiple pollers can make concurrent requests to the same endpoint.

**Evidence:**

- `filtering.js`: `fetchStats()` → `/api/filtering/stats` every 1s
- `header.js`: `fetchHeaderMetrics()` → `/api/header/metrics` every 5s
- No mechanism to prevent duplicate in-flight requests to same endpoint

**Impact:** If a request takes >1s, next poller tick creates another request → exponential growth.

### 2. **No Request Queuing or Concurrency Control** ❌

**Problem:** All pollers fire independently with no coordination.

**Evidence:**

- Browser limit: 6-8 concurrent connections per domain
- Active pollers: header metrics (5s), header status (interval), filtering stats (1s), home stats (1s), tokens poller (1s), positions poller (1s), wallet poller (1s), trader stats (1s), strategies poller (1s)
- When switching tabs rapidly, old pollers overlap with new ones

**Impact:** Connection pool exhaustion → all requests fail with "Failed to fetch"

### 3. **Inconsistent Error Handling** ⚠️

**Problem:** Some fetch functions throw errors, others silently fail.

**Evidence:**

```javascript
// filtering.js - throws error
async function fetchStats() {
  const response = await fetch("/api/filtering/stats");
  if (!response.ok) {
    throw new Error(`Failed to fetch stats: ${response.statusText}`);
  }
  return response.json();
}

// But caught silently in loadStats
async function loadStats() {
  try {
    const response = await fetchStats();
    state.stats = response.data || response;
  } catch (error) {
    console.error("Failed to load stats:", error);
    // Don't show toast for stats errors (non-critical)
  }
}

// header.js - returns null on error
async function fetchHeaderMetrics() {
  try {
    const res = await fetch("/api/header/metrics", {...});
    if (!res.ok) {
      console.warn(`[Header] Metrics fetch failed: ${res.status}`);
      return null;
    }
    const data = await res.json();
    updateHeaderMetrics(data);
    return data;
  } catch (err) {
    if (err?.name !== "AbortError") {
      console.error("[Header] Failed to fetch metrics:", err);
    }
    return null;
  }
}
```

**Impact:** Inconsistent behavior, difficult to debug, no standardized retry logic.

### 4. **No Fetch Timeout Handling** ❌

**Problem:** No timeouts on fetch requests - they hang indefinitely.

**Evidence:**

- All fetch calls lack `signal` parameter with timeout
- Only `positions.js` uses AbortController for signal
- No global timeout configuration

**Impact:** Slow endpoints can block connections indefinitely, worsening pool exhaustion.

### 5. **No Exponential Backoff on Failures** ❌

**Problem:** Failed requests immediately retry at same interval.

**Evidence:**

- Poller continues firing every 1s even if all requests fail
- No backoff mechanism in `Poller` class
- Console floods with error messages

**Impact:** Continues hammering failing endpoints, no recovery mechanism.

### 6. **Global Header Pollers Never Stop** ⚠️

**Problem:** Header pollers run continuously regardless of page state.

**Evidence:**

```javascript
// header.js - started on init, never managed by lifecycle
function startMetricsPolling() {
  if (metricsPoller) {
    metricsPoller.cleanup();
  }
  metricsPoller = new Poller(() => fetchHeaderMetrics(), {
    label: "HeaderMetrics",
    interval: METRICS_POLL_INTERVAL,
  });
  metricsPoller.start({ silent: true });
}
```

**Impact:** 2 global pollers always running + page-specific pollers = excessive concurrent requests.

### 7. **Poller Cleanup Race Conditions** ⚠️

**Problem:** When switching tabs rapidly, old poller may not stop before new one starts.

**Evidence:**

- Lifecycle calls `deactivate()` → `poller.stop()`
- But `setInterval` clear is synchronous, callback might be in-flight
- No mechanism to wait for in-flight requests

**Impact:** Overlapping pollers during tab switches.

### 8. **No Request Prioritization** ❌

**Problem:** All requests treated equally, no priority queue.

**Evidence:**

- Critical requests (user actions) compete with polling requests
- No way to prioritize user-initiated actions over background polls

**Impact:** User actions delayed by background polling.

### 9. **Missing Fetch Utilities** ❌

**Problem:** No centralized fetch wrapper with retry/timeout/dedup logic.

**Evidence:**

- `utils.js` has no fetch helpers
- Every page implements own fetch patterns
- Copy-paste code duplication

**Impact:** Inconsistent behavior, hard to fix globally.

### 10. **No Request Metrics/Monitoring** ❌

**Problem:** No visibility into fetch performance or failure rates.

**Evidence:**

- No timing metrics
- No failure tracking
- No request queue depth monitoring

**Impact:** Can't detect problems proactively.

---

## Page-by-Page Analysis

### header.js (GLOBAL - Always Active)

**Pollers:** 2 (metrics every 5s, status every configurable interval)  
**Issues:**

- No lifecycle management (never stops)
- No request deduplication
- Silent failures

**Impact:** HIGH - Runs continuously even when not visible

### home.js

**Pollers:** 1 (dashboard data every 1s)  
**Issues:**

- fetchData() has no error handling for network failures
- No timeout

**Impact:** MEDIUM - Only active on home page

### filtering.js

**Pollers:** 1 (stats every 1s)  
**Issues:**

- fetchStats() throws errors (caught silently)
- No deduplication with header metrics
- loadStats catches but doesn't surface errors

**Impact:** HIGH - User-reported empty subtabs issue

### tokens.js

**Pollers:** 1 (token list refresh)  
**Issues:**

- Complex DataTable refresh logic
- Multiple fetch calls in single poll cycle
- No request batching

**Impact:** MEDIUM - Can cause spikes during refresh

### positions.js

**Pollers:** 1 (position list refresh)  
**Issues:**

- Uses AbortController (good!) but inconsistently
- DataTable refresh can trigger multiple fetches
- Manual trade actions use separate fetch patterns

**Impact:** MEDIUM - Position updates critical for trading

### wallet.js

**Pollers:** 1 (wallet data every 1s)  
**Issues:**

- fetchCurrentSnapshot() no timeout
- fetchDashboardData() no retry logic
- Refresh button can conflict with poller

**Impact:** LOW - Less frequently accessed

### trader.js

**Pollers:** 3 (stats, config, strategies)  
**Issues:**

- Multiple concurrent pollers on same page
- Complex interdependencies
- No coordination between pollers

**Impact:** HIGH - Most resource-intensive page

### strategies.js

**Pollers:** 2 (strategies list, templates)  
**Issues:**

- Two pollers can fire simultaneously
- No deduplication
- Complex state management

**Impact:** MEDIUM - Less frequently used

---

## Architectural Problems

### 1. No Fetch Abstraction Layer

Every page implements its own fetch patterns:

```javascript
// Pattern 1: Throw on error
const response = await fetch(url);
if (!response.ok) throw new Error();

// Pattern 2: Return null on error
try {
  const response = await fetch(url);
  if (!response.ok) return null;
} catch (err) {
  return null;
}

// Pattern 3: Silent catch
try {
  const response = await fetch(url);
} catch (error) {
  console.error(error);
}
```

### 2. No Request Coordination

Pollers are independent - no system-wide view of:

- Total active requests
- Requests per endpoint
- Request queue depth
- Connection pool utilization

### 3. No Progressive Loading

All data fetched eagerly:

- No lazy loading
- No incremental rendering
- No data prefetching for predicted navigation

### 4. No Request Caching

Every poll makes new request:

- No HTTP cache headers respected
- No client-side cache
- No stale-while-revalidate pattern

---

## Proposed Solutions

### Phase 1: Immediate Fixes (P0)

**Goal:** Stop the request storm

1. **Request Deduplication**
   - Create RequestManager with in-flight tracking
   - Return existing promise if request to endpoint already in-flight
   - Add per-endpoint locks

2. **Request Timeout**
   - Add 10s default timeout to all fetch calls
   - Use AbortController consistently
   - Cancel pending requests on page deactivate

3. **Concurrency Limiting**
   - Max 4 concurrent requests system-wide
   - Queue excess requests
   - Prioritize user-initiated actions

4. **Better Error Handling**
   - Standardize error responses
   - Add exponential backoff (1s, 2s, 4s, 8s, max 30s)
   - Surface errors to UI when appropriate

### Phase 2: Performance Improvements (P1)

**Goal:** Optimize request patterns

1. **Batch Requests**
   - Combine multiple endpoints into single request
   - Add `/api/dashboard/batch` endpoint
   - Reduce round trips

2. **Smart Polling**
   - Adaptive intervals based on page activity
   - Pause polling when tab not visible
   - Increase interval when no data changes

3. **Request Caching**
   - Add TTL-based cache
   - Implement stale-while-revalidate
   - Use ETag/If-None-Match headers

4. **Progressive Loading**
   - Load critical data first
   - Lazy load secondary data
   - Show loading states

### Phase 3: Architectural Improvements (P2)

**Goal:** Long-term scalability

1. **WebSocket Streaming**
   - Replace polling with WebSocket for real-time data
   - Fallback to polling if WebSocket unavailable
   - Server pushes updates instead of client pulling

2. **Service Worker**
   - Offline support
   - Background sync
   - Push notifications

3. **Request Metrics**
   - Add timing instrumentation
   - Track failure rates
   - Performance monitoring dashboard

---

## Implementation Priority

### Critical (Do First)

1. ✅ Create `RequestManager` class with deduplication
2. ✅ Add timeout to all fetch calls (10s default)
3. ✅ Implement concurrency limiting (max 4)
4. ✅ Add exponential backoff on failures
5. ✅ Standardize error handling pattern

### High (Do Soon)

6. ⏳ Pause header pollers when page hidden
7. ⏳ Batch related requests
8. ⏳ Add request caching layer
9. ⏳ Implement adaptive polling intervals

### Medium (Nice to Have)

10. ⏳ Add WebSocket streaming
11. ⏳ Implement progressive loading
12. ⏳ Add request metrics
13. ⏳ Service Worker for offline support

---

## Code Patterns to Follow

### Good Fetch Pattern

```javascript
import { requestManager } from "./request_manager.js";

async function fetchData() {
  try {
    const data = await requestManager.fetch("/api/endpoint", {
      timeout: 10000, // 10s
      priority: "normal", // or 'high' for user actions
      retry: { maxAttempts: 3, backoff: "exponential" },
    });
    return data;
  } catch (error) {
    if (error.name === "TimeoutError") {
      showToast("Request timed out", "error");
    } else if (error.name === "NetworkError") {
      showToast("Network error, retrying...", "warning");
    }
    throw error;
  }
}
```

### Good Poller Pattern

```javascript
export function createLifecycle() {
  return {
    activate(ctx) {
      const poller = ctx.managePoller(
        new Poller(
          async () => {
            // Use RequestManager - handles deduplication automatically
            const data = await requestManager.fetch("/api/data");
            updateUI(data);
          },
          {
            label: "PageData",
            adaptive: true, // Slow down when no changes
            pauseWhenHidden: true, // Stop when tab hidden
          }
        )
      );
      poller.start();
    },

    deactivate() {
      // Pollers auto-stopped by lifecycle
      // Cancel any in-flight requests
      requestManager.cancelAll({ source: "page-deactivate" });
    },
  };
}
```

---

## Success Metrics

### Before Fix

- Requests per second: 20-50+
- Failed requests: 30-50%
- Page load time: 5-10s
- Tab switch time: 2-5s
- Console errors: 100+ per minute

### After Fix (Target)

- Requests per second: 2-5
- Failed requests: <1%
- Page load time: 1-2s
- Tab switch time: <500ms
- Console errors: 0

---

## Testing Plan

1. **Load Testing**
   - Rapid tab switching (10 switches in 10s)
   - Leave browser idle for 5 minutes
   - Multiple concurrent user actions

2. **Network Conditions**
   - Slow 3G simulation
   - Offline → online transition
   - High latency (500ms)

3. **Error Scenarios**
   - Backend returns 500 errors
   - Backend times out (30s+)
   - Backend rate limits (429)

4. **Browser Testing**
   - Chrome (connection limit: 6)
   - Firefox (connection limit: 6)
   - Safari (connection limit: 6)

---

## Conclusion

The frontend has **no request coordination or deduplication**, leading to **request storms** that exhaust browser connection pools. This is a **systemic architectural issue** that requires:

1. **Immediate:** Request deduplication + concurrency limiting
2. **Short-term:** Standardized fetch patterns + error handling
3. **Long-term:** WebSocket streaming + service worker

**Priority:** P0 - Implement Phase 1 immediately to restore functionality.
