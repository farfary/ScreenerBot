// Polling Manager - Global polling interval coordination
import * as AppState from "./app_state.js";

const _state = {
  interval: null,
  listeners: [],
};

const DEFAULT_INTERVAL = 1000;

export function init() {
  // Apply a safe default synchronously so early callers never read null, then
  // adopt the persisted value once AppState's cache is ready. AppState.init()
  // is an async fetch, so reading load() synchronously at module load would
  // race ahead of it (returning the default and pinning it forever). loadAsync
  // awaits init, so the saved interval is honored on every startup.
  if (_state.interval === null) {
    _state.interval = DEFAULT_INTERVAL;
  }
  AppState.loadAsync("pollingInterval", DEFAULT_INTERVAL)
    .then((value) => {
      const ms = Number(value);
      if (Number.isFinite(ms) && ms > 0 && ms !== _state.interval) {
        const oldInterval = _state.interval;
        _state.interval = ms;
        // Reschedule any pollers already started with the default — they only
        // reread the interval when notified.
        _state.listeners.forEach((callback) => {
          try {
            callback(ms, oldInterval);
          } catch (err) {
            console.error("[PollingManager] Listener callback failed:", err);
          }
        });
      }
    })
    .catch(() => {
      /* keep the default; AppState already logged the failure */
    });
}

export function getInterval() {
  if (_state.interval === null) {
    init();
  }
  return _state.interval;
}

export function setInterval(ms) {
  const oldInterval = _state.interval;
  _state.interval = ms;
  AppState.save("pollingInterval", ms);
  console.log("[PollingManager] Interval changed from", oldInterval, "ms to", ms, "ms");

  // Notify all listeners
  _state.listeners.forEach((callback) => {
    try {
      callback(ms, oldInterval);
    } catch (err) {
      console.error("[PollingManager] Listener callback failed:", err);
    }
  });
}

export function onChange(callback) {
  if (typeof callback === "function") {
    _state.listeners.push(callback);
  }
  return callback;
}

export function removeListener(callback) {
  const index = _state.listeners.indexOf(callback);
  if (index > -1) {
    _state.listeners.splice(index, 1);
  }
}

// Poller class - per-page polling lifecycle
export class Poller {
  /**
   * @param {Function} onPoll
   * @param {Object} [options]
   * @param {string} [options.label]
   * @param {number} [options.intervalMs] - fixed cadence in ms. Without it the poller
   *   follows the user's global dashboard interval (1s by default), which is almost never
   *   what a heavy or slow-moving endpoint wants. This option used to be ignored: every
   *   call site that passed one (`intervalMs`, and a misspelt `interval`) silently ran at
   *   the global rate instead — the token-details chart's careful 3s/10s/15s backoff was
   *   really refetching candles every second, and the position dialog refetched its whole
   *   detail payload plus full candle history on every 1s tick.
   * @param {Function} [options.getInterval] - dynamic cadence; wins over `intervalMs`.
   *   Only re-read when the poller is (re)started.
   */
  constructor(onPoll, options = {}) {
    this.label = options.label || "Poller";
    this.onPoll = onPoll;
    this.getInterval = options.getInterval;
    this.intervalMs = options.intervalMs;
    this.pauseWhenHidden = options.pauseWhenHidden !== false; // Default true
    this.adaptive = options.adaptive || false;

    this.timerId = null;
    this.listener = null;
    this.active = false;
    this.consecutiveFailures = 0;
    this.lastSuccessTime = null;
    this.isPaused = false;

    if (typeof onPoll !== "function") {
      throw new Error(`[Poller:${this.label}] onPoll callback is required`);
    }
  }

  _logPrefix() {
    return `[Poller:${this.label}]`;
  }

  _computeInterval() {
    if (typeof this.getInterval === "function") {
      try {
        const value = Number(this.getInterval());
        if (Number.isFinite(value) && value > 0) {
          return value;
        }
      } catch (err) {
        console.warn(`${this._logPrefix()} getInterval failed, falling back`, err);
      }
    }

    if (Number.isFinite(this.intervalMs) && this.intervalMs > 0) {
      return this.intervalMs;
    }

    try {
      const value = Number(getInterval());
      if (Number.isFinite(value) && value > 0) {
        return value;
      }
    } catch (err) {
      console.warn(`${this._logPrefix()} PollingManager.getInterval failed, using default`, err);
    }

    return 1000;
  }

  _schedule() {
    const interval = this._computeInterval();
    this.timerId = globalThis.setInterval(() => {
      // Skip if paused (e.g., tab hidden)
      if (this.isPaused) {
        return;
      }

      // Skip while the backend is unreachable — the connectivity watcher shows
      // the global overlay and will fire `screenerbot:reconnected` on recovery;
      // there's no point firing doomed fetches that just log console errors.
      if (window.__SB_CONNECTIVITY__ && window.__SB_CONNECTIVITY__.isBackendOnline() === false) {
        return;
      }

      try {
        const result = this.onPoll();
        if (result && typeof result.then === "function") {
          Promise.resolve(result)
            .then(() => {
              // Success - reset failure counter
              this.consecutiveFailures = 0;
              this.lastSuccessTime = Date.now();
            })
            .catch((error) => {
              // Failure - increment counter
              this.consecutiveFailures++;
              console.error(
                `${this._logPrefix()} Poll callback rejected (${this.consecutiveFailures} consecutive failures)`,
                error
              );

              // Apply exponential backoff if multiple failures
              if (this.consecutiveFailures >= 3) {
                const backoffDelay = Math.min(
                  1000 * Math.pow(2, this.consecutiveFailures - 3),
                  30000
                );
                console.warn(
                  `${this._logPrefix()} Applying backoff: ${backoffDelay}ms (${this.consecutiveFailures} failures)`
                );
              }
            });
        } else {
          // Synchronous success
          this.consecutiveFailures = 0;
          this.lastSuccessTime = Date.now();
        }
      } catch (error) {
        this.consecutiveFailures++;
        console.error(
          `${this._logPrefix()} Poll callback threw (${this.consecutiveFailures} consecutive failures)`,
          error
        );
      }
    }, interval);

    // Track interval with Router for cleanup (legacy compatibility)
    if (window.Router && typeof window.Router.trackInterval === "function") {
      window.Router.trackInterval(this.timerId);
    }

    this.active = true;
    return interval;
  }

  _ensureListener() {
    // A fixed-cadence poller does not follow the global interval, so it must not be
    // restarted (and re-logged) every time the user changes that setting.
    if (this.listener || (Number.isFinite(this.intervalMs) && this.intervalMs > 0)) {
      return;
    }

    this.listener = onChange(() => {
      if (!this.active) {
        return;
      }
      const interval = this.start({ silent: true });
      console.log(`${this._logPrefix()} Polling interval changed → ${interval} ms`);
    });
  }

  start(options = {}) {
    this.stop({ silent: true });
    const interval = this._schedule();
    this._ensureListener();

    if (!options.silent) {
      console.log(`${this._logPrefix()} Started polling every ${interval} ms`);
    }

    return interval;
  }

  stop(options = {}) {
    if (!this.timerId) {
      this.active = false;
      return;
    }

    globalThis.clearInterval(this.timerId);
    this.timerId = null;
    this.active = false;

    if (!options.silent) {
      console.log(`${this._logPrefix()} Stopped polling`);
    }
  }

  restart() {
    const interval = this.start({ silent: true });
    console.log(`${this._logPrefix()} Restarted polling (${interval} ms)`);
    return interval;
  }

  cleanup() {
    this.stop();
    if (this.listener) {
      removeListener(this.listener);
    }
    this.listener = null;
  }

  pause() {
    if (this.isPaused) return;
    this.isPaused = true;
  }

  resume() {
    if (!this.isPaused) return;
    this.isPaused = false;
  }

  isActive() {
    return this.active;
  }

  isPausedState() {
    return this.isPaused;
  }

  getFailureCount() {
    return this.consecutiveFailures;
  }
}

init();
