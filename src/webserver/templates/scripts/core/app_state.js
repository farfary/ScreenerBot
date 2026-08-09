// State Manager - Server-Side Storage Only
//
// All state is stored on the backend in data/ui_state.json
// No localStorage is used - this ensures persistence across app restarts
// (Dynamic ports make localStorage unreliable)

// In-memory cache (populated on init, updated on save)
let stateCache = null;
let initPromise = null;
let pendingSaves = new Map();
let saveTimeout = null;

// Initialize - load all state from server (call once on app startup)
export async function init() {
  if (initPromise) return initPromise;

  initPromise = (async () => {
    try {
      const response = await fetch("/api/ui-state/all");
      if (response.ok) {
        stateCache = await response.json();
      } else {
        console.warn("[AppState] Failed to load state from server, using empty state");
        stateCache = {};
      }
    } catch (e) {
      console.warn("[AppState] Failed to load state from server:", e);
      stateCache = {};
    }
    return stateCache;
  })();

  return initPromise;
}

// Ensure initialized before operations
async function ensureInit() {
  if (stateCache === null) {
    await init();
  }
}

// Flush pending saves to server (debounced batch save)
export async function flushPendingSaves({ keepalive = false } = {}) {
  if (pendingSaves.size === 0) return;

  if (saveTimeout) {
    clearTimeout(saveTimeout);
    saveTimeout = null;
  }
  const snapshot = new Map(pendingSaves);
  const entries = Object.fromEntries(snapshot);

  try {
    const response = await fetch("/api/ui-state/batch-save", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ entries }),
      keepalive,
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    snapshot.forEach((value, key) => {
      if (pendingSaves.get(key) === value) pendingSaves.delete(key);
    });
  } catch (e) {
    console.warn("[AppState] Failed to batch save state:", e);
    if (!keepalive && pendingSaves.size > 0) {
      saveTimeout = setTimeout(flushPendingSaves, 1000);
    }
  }
}

// Schedule debounced save
function scheduleSave(key, value) {
  pendingSaves.set(key, value);
  if (saveTimeout) clearTimeout(saveTimeout);
  saveTimeout = setTimeout(flushPendingSaves, 200);
}

// Save a value (async but returns immediately, batches saves)
export function save(key, value) {
  if (stateCache === null) {
    console.warn("[AppState] save() called before init(), queueing save for:", key);
  }

  // Update local cache immediately
  if (stateCache) {
    stateCache[key] = value;
  }

  // Schedule server save (debounced)
  scheduleSave(key, value);
}

// Load a value (synchronous from cache - MUST call init() first)
export function load(key, defaultValue = null) {
  if (stateCache === null) {
    console.warn("[AppState] load() called before init() completed for key:", key);
    return defaultValue;
  }

  const value = stateCache[key];
  return value !== undefined ? value : defaultValue;
}

// Load a value asynchronously (ensures init is complete)
export async function loadAsync(key, defaultValue = null) {
  await ensureInit();
  return load(key, defaultValue);
}

// Remove a key
export function remove(key) {
  if (stateCache) {
    delete stateCache[key];
  }

  // Remove from pending saves if queued
  pendingSaves.delete(key);

  // Send remove request to server
  fetch("/api/ui-state/remove", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ key }),
  }).catch((e) => console.warn("[AppState] Failed to remove state:", key, e));
}

// Clear all state
export async function clearAll() {
  stateCache = {};
  pendingSaves.clear();
  if (saveTimeout) {
    clearTimeout(saveTimeout);
    saveTimeout = null;
  }

  try {
    await fetch("/api/ui-state/clear", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });
  } catch (e) {
    console.warn("[AppState] Failed to clear state:", e);
  }
}

// Check if initialized
export function isInitialized() {
  return stateCache !== null;
}

// Get all state (for debugging)
export function getAll() {
  return stateCache ? { ...stateCache } : null;
}

window.addEventListener("pagehide", () => {
  void flushPendingSaves({ keepalive: true });
});
