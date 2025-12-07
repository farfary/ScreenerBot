// Status Bar - Fetches and displays version info
// Non-module script for immediate execution

(function () {
  "use strict";

  const CACHE_KEY = "screenerbot_version_info";
  const CACHE_DURATION_MS = 24 * 60 * 60 * 1000; // 24 hours

  function getCachedVersionInfo() {
    try {
      const cached = localStorage.getItem(CACHE_KEY);
      if (cached) {
        const { version, timestamp } = JSON.parse(cached);
        if (Date.now() - timestamp < CACHE_DURATION_MS) {
          return { version };
        }
      }
    } catch {
      // Ignore cache errors
    }
    return null;
  }

  function setCachedVersionInfo(version) {
    try {
      localStorage.setItem(
        CACHE_KEY,
        JSON.stringify({
          version,
          timestamp: Date.now(),
        })
      );
    } catch {
      // Ignore cache errors
    }
  }

  function updateVersionDisplay(version) {
    const el = document.getElementById("statusBarVersion");
    if (el) {
      el.textContent = `v${version}`;
    }
  }

  async function fetchVersionInfo() {
    try {
      const response = await fetch("/api/version");
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const data = await response.json();
      const version = data.version || data.data?.version;
      return { version };
    } catch (err) {
      console.warn("Failed to fetch version:", err);
      return null;
    }
  }

  async function initStatusBar() {
    // Try cache first for instant display
    const cached = getCachedVersionInfo();
    if (cached) {
      updateVersionDisplay(cached.version);
    }

    // Fetch fresh version (updates cache)
    const info = await fetchVersionInfo();
    if (info && info.version) {
      updateVersionDisplay(info.version);
      setCachedVersionInfo(info.version);
    } else if (!cached) {
      // Fallback if no cache and fetch failed
      updateVersionDisplay("—");
    }
  }

  // Initialize when DOM is ready
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initStatusBar);
  } else {
    initStatusBar();
  }
})();
