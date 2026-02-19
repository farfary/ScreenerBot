# Request Manager - Developer Quick Reference

## Basic Usage

```javascript
import { requestManager } from "../core/request_manager.js";

// Simple GET request
const data = await requestManager.fetch("/api/endpoint");

// POST request with priority
const result = await requestManager.fetch("/api/endpoint", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(payload),
  priority: "high", // Use "high" for user actions, "normal" for polling
});
```

## Migration Pattern

### Before (Direct fetch)

```javascript
async function fetchData() {
  const response = await fetch("/api/endpoint");
  if (!response.ok) {
    throw new Error(`Failed: ${response.statusText}`);
  }
  return response.json();
}
```

### After (RequestManager)

```javascript
async function fetchData() {
  return await requestManager.fetch("/api/endpoint", {
    priority: "normal",
  });
}
```

## Error Handling

RequestManager automatically handles:

- HTTP errors (throws with status code)
- Timeouts (throws `TimeoutError`)
- Network errors (throws original error)

```javascript
try {
  const data = await requestManager.fetch("/api/endpoint");
  // Success
} catch (error) {
  if (error.name === "TimeoutError") {
    console.error("Request timed out");
  } else if (error.status) {
    console.error(`HTTP ${error.status}: ${error.message}`);
  } else {
    console.error("Network error:", error);
  }
}
```

## Priority Levels

- **`"high"`**: User-initiated actions (clicks, form submits)
  - Processed first in queue
  - Use for: manual trades, config saves, user searches
- **`"normal"`**: Background polling, automatic updates
  - Standard priority
  - Use for: dashboard polling, metrics updates, token lists

## Advanced Options

```javascript
const data = await requestManager.fetch("/api/endpoint", {
  method: "GET",
  headers: { "X-Custom": "value" },
  priority: "high",
  timeout: 15000, // Custom timeout (default 10s)
  skipDedup: true, // Allow concurrent requests to same endpoint
  skipQueue: true, // Bypass queue (use sparingly)
});
```

## Debugging

```javascript
// Get current stats
const stats = requestManager.getStats();
console.log("In-flight:", stats.inFlight);
console.log("Active:", stats.activeCount);
console.log("Queued:", stats.queued);
console.log("Failed endpoints:", stats.failedEndpoints);

// Reset state (testing only)
requestManager.reset();
```

## Best Practices

1. **Always use priority**: Specify `"high"` or `"normal"`
2. **Let RequestManager handle errors**: Don't wrap in try-catch unless needed
3. **Trust the queue**: Don't use `skipQueue` unless absolutely necessary
4. **Monitor failures**: Check `getStats().failedEndpoints` for persistent issues
5. **Use default timeout**: Only increase for known slow endpoints

## Common Patterns

### Polling with Poller

```javascript
const poller = ctx.managePoller(
  new Poller(
    async () => {
      const data = await requestManager.fetch("/api/data");
      updateUI(data);
    },
    { label: "MyPoller", pauseWhenHidden: true }
  )
);
poller.start();
```

### User Action with High Priority

```javascript
async function handleSubmit() {
  try {
    const result = await requestManager.fetch("/api/action", {
      method: "POST",
      body: JSON.stringify(payload),
      priority: "high",
    });
    showToast("Success!", "success");
  } catch (error) {
    showToast(`Error: ${error.message}`, "error");
  }
}
```

### Multiple Requests (Batched)

```javascript
// RequestManager handles concurrency automatically
const [data1, data2, data3] = await Promise.all([
  requestManager.fetch("/api/endpoint1"),
  requestManager.fetch("/api/endpoint2"),
  requestManager.fetch("/api/endpoint3"),
]);
// Max 4 will run concurrently, rest queued
```

## Testing

```javascript
// In browser console:
window.__requestManager.getStats();

// Monitor requests during tab switching:
// 1. Open DevTools Network tab
// 2. Switch tabs rapidly
// 3. Verify max 4 concurrent connections
// 4. Verify no "Failed to fetch" errors
```
