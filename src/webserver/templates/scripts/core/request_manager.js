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
const MAX_CONNECTION_RETRIES = 5;
const CONNECTION_RETRY_DELAY_MS = 1000;

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
   * Check if error is a connection failure (server not ready)
   */
  _isConnectionError(error) {
    // Network errors when server is down
    if (error instanceof TypeError && error.message.includes("Failed to fetch")) {
      return true;
    }
    // ERR_CONNECTION_REFUSED, ERR_CONNECTION_RESET, etc.
    if (error.name === "TypeError" || error.message?.includes("NetworkError")) {
      return true;
    }
    return false;
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
   *   - retryOnConnectionError: boolean (default true for boot resilience)
   * @returns {Promise<any>} - Parsed JSON response
   */
  async fetch(url, options = {}) {
    const {
      timeout = DEFAULT_TIMEOUT,
      priority = "normal",
      skipDedup = false,
      skipQueue = false,
      retryOnConnectionError = true,
      signal: externalSignal = null,
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

    // Create the fetch promise with connection retry logic
    const fetchPromise = new Promise((resolve, reject) => {
      const executeWithRetry = async (retryCount = 0) => {
        // Create abort controller for timeout
        const controller = new AbortController();
        let externalAbortHandler = null;
        let didTimeout = false;

        if (externalSignal) {
          if (externalSignal.aborted) {
            controller.abort();
            const abortError = new Error("Request aborted");
            abortError.name = "AbortError";
            reject(abortError);
            return;
          }
          externalAbortHandler = () => controller.abort();
          externalSignal.addEventListener("abort", externalAbortHandler, { once: true });
        }

        const timeoutId = setTimeout(() => {
          didTimeout = true;
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
          if (externalSignal && externalAbortHandler) {
            externalSignal.removeEventListener("abort", externalAbortHandler);
          }

          // Check if this is a connection error and we should retry
          if (
            retryOnConnectionError &&
            this._isConnectionError(error) &&
            retryCount < MAX_CONNECTION_RETRIES
          ) {
            this.activeCount--;
            // Wait before retry (progressive delay)
            const retryDelay = CONNECTION_RETRY_DELAY_MS * (retryCount + 1);
            console.debug(
              `[RequestManager] Connection failed for ${endpoint}, retry ${retryCount + 1}/${MAX_CONNECTION_RETRIES} in ${retryDelay}ms`
            );
            await new Promise((r) => setTimeout(r, retryDelay));
            return executeWithRetry(retryCount + 1);
          }

          if (error.name === "AbortError") {
            if (didTimeout) {
              this._recordFailure(endpoint);
              const timeoutError = new Error(`Request timeout after ${timeout}ms`);
              timeoutError.name = "TimeoutError";
              timeoutError.endpoint = endpoint;
              reject(timeoutError);
            } else {
              reject(error);
            }
          } else {
            this._recordFailure(endpoint);
            reject(error);
          }
        } finally {
          if (externalSignal && externalAbortHandler) {
            externalSignal.removeEventListener("abort", externalAbortHandler);
          }
          clearTimeout(timeoutId);
          this.activeCount--;
          this.inFlight.delete(key);

          // Process next queued request
          this._processQueue();
        }
      };

      // If under concurrency limit and not skipping queue, execute immediately
      if (this.activeCount < MAX_CONCURRENT_REQUESTS || skipQueue) {
        executeWithRetry(0);
      } else {
        // Queue the request
        this.queue.push({
          execute: () => executeWithRetry(0),
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

export function createScopedFetcher(ctx, { latestOnly = false } = {}) {
  if (!ctx || typeof ctx.createAbortController !== "function") {
    throw new Error("createScopedFetcher requires a lifecycle context");
  }

  let lastController = null;

  return (url, options = {}) => {
    if (latestOnly && lastController) {
      try {
        lastController.abort();
      } catch (error) {
        console.warn("[RequestManager] Failed to abort previous request", error);
      }
    }

    const controller = ctx.createAbortController();
    lastController = controller;

    return requestManager.fetch(url, {
      ...options,
      signal: controller.signal,
    });
  };
}

export { requestManager, RequestManager };
