/**
 * Request Manager - Centralized fetch coordination
 *
 * Features:
 * - Request deduplication (in-flight tracking)
 * - Automatic timeout handling (10s default)
 * - Concurrency limiting (max 4 concurrent)
 * - Priority queue (high priority for user actions)
 * - Exponential backoff on failures
 */

const DEFAULT_TIMEOUT = 10000; // 10 seconds
const MAX_CONCURRENT_REQUESTS = 4;

class RequestManager {
  constructor() {
    // Track in-flight requests by key (method:url)
    this.inFlight = new Map();

    // Active request count for concurrency control
    this.activeCount = 0;

    // Queue for excess requests
    this.queue = [];

    // Per-endpoint failure tracking for backoff
    this.failures = new Map();
  }

  /**
   * Create unique key for request deduplication
   */
  _createKey(url, options = {}) {
    const method = (options.method || "GET").toUpperCase();
    return `${method}:${url}`;
  }

  /**
   * Get backoff delay for endpoint based on consecutive failures
   */
  _getBackoffDelay(endpoint) {
    const failures = this.failures.get(endpoint) || 0;
    if (failures === 0) return 0;

    // Exponential backoff: 1s, 2s, 4s, 8s, max 30s
    const delay = Math.min(1000 * Math.pow(2, failures - 1), 30000);
    return delay;
  }

  /**
   * Record failure for endpoint
   */
  _recordFailure(endpoint) {
    const current = this.failures.get(endpoint) || 0;
    this.failures.set(endpoint, current + 1);
  }

  /**
   * Clear failure tracking for endpoint on success
   */
  _clearFailures(endpoint) {
    this.failures.delete(endpoint);
  }

  /**
   * Process next queued request if under concurrency limit
   */
  _processQueue() {
    if (this.activeCount >= MAX_CONCURRENT_REQUESTS || this.queue.length === 0) {
      return;
    }

    // Sort by priority (high = 1, normal = 0)
    this.queue.sort((a, b) => (b.priority || 0) - (a.priority || 0));

    const next = this.queue.shift();
    if (next) {
      next.execute();
    }
  }

  /**
   * Main fetch wrapper with all features
   *
   * @param {string} url - Request URL
   * @param {object} options - Fetch options plus:
   *   - timeout: number (ms, default 10000)
   *   - priority: 'high' | 'normal' (default 'normal')
   *   - skipDedup: boolean (default false)
   *   - skipQueue: boolean (default false)
   * @returns {Promise<any>} - Parsed JSON response
   */
  async fetch(url, options = {}) {
    const {
      timeout = DEFAULT_TIMEOUT,
      priority = "normal",
      skipDedup = false,
      skipQueue = false,
      ...fetchOptions
    } = options;

    const key = this._createKey(url, fetchOptions);
    const endpoint = new URL(url, window.location.origin).pathname;

    // Check backoff delay
    const backoffDelay = this._getBackoffDelay(endpoint);
    if (backoffDelay > 0) {
      await new Promise((resolve) => setTimeout(resolve, backoffDelay));
    }

    // Check for in-flight duplicate (unless skipped)
    if (!skipDedup && this.inFlight.has(key)) {
      return this.inFlight.get(key);
    }

    // Create the fetch promise
    const fetchPromise = new Promise((resolve, reject) => {
      const execute = async () => {
        // Create abort controller for timeout
        const controller = new AbortController();
        const timeoutId = setTimeout(() => {
          controller.abort();
        }, timeout);

        try {
          this.activeCount++;

          const response = await fetch(url, {
            ...fetchOptions,
            signal: controller.signal,
          });

          clearTimeout(timeoutId);

          if (!response.ok) {
            this._recordFailure(endpoint);
            const error = new Error(`HTTP ${response.status}: ${response.statusText}`);
            error.status = response.status;
            error.response = response;
            throw error;
          }

          // Success - clear failure tracking
          this._clearFailures(endpoint);

          // Parse JSON if content-type indicates it
          const contentType = response.headers.get("content-type");
          if (contentType && contentType.includes("application/json")) {
            const data = await response.json();
            resolve(data);
          } else {
            resolve(response);
          }
        } catch (error) {
          clearTimeout(timeoutId);

          // Handle timeout
          if (error.name === "AbortError") {
            this._recordFailure(endpoint);
            const timeoutError = new Error(`Request timeout after ${timeout}ms`);
            timeoutError.name = "TimeoutError";
            timeoutError.endpoint = endpoint;
            reject(timeoutError);
          } else {
            this._recordFailure(endpoint);
            reject(error);
          }
        } finally {
          this.activeCount--;
          this.inFlight.delete(key);

          // Process next queued request
          this._processQueue();
        }
      };

      // If under concurrency limit and not skipping queue, execute immediately
      if (this.activeCount < MAX_CONCURRENT_REQUESTS || skipQueue) {
        execute();
      } else {
        // Queue the request
        this.queue.push({
          execute,
          priority: priority === "high" ? 1 : 0,
          timestamp: Date.now(),
        });
      }
    });

    // Track in-flight request
    if (!skipDedup) {
      this.inFlight.set(key, fetchPromise);
    }

    return fetchPromise;
  }

  /**
   * Cancel all in-flight requests
   */
  cancelAll() {
    this.inFlight.clear();
    this.queue = [];
  }

  /**
   * Get current statistics
   */
  getStats() {
    return {
      inFlight: this.inFlight.size,
      activeCount: this.activeCount,
      queued: this.queue.length,
      failedEndpoints: Array.from(this.failures.entries()).map(([endpoint, count]) => ({
        endpoint,
        failures: count,
        backoffMs: this._getBackoffDelay(endpoint),
      })),
    };
  }

  /**
   * Reset all state (useful for testing)
   */
  reset() {
    this.cancelAll();
    this.failures.clear();
    this.activeCount = 0;
  }
}

// Global singleton instance
const requestManager = new RequestManager();

// Expose for debugging
if (typeof window !== "undefined") {
  window.__requestManager = requestManager;
}

export { requestManager, RequestManager };
