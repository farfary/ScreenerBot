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
        const { version, buildNumber, timestamp } = JSON.parse(cached);
        if (Date.now() - timestamp < CACHE_DURATION_MS) {
          return { version, buildNumber };
        }
      }
    } catch {
      // Ignore cache errors
    }
    return null;
  }

  function setCachedVersionInfo(version, buildNumber) {
    try {
      localStorage.setItem(
        CACHE_KEY,
        JSON.stringify({
          version,
          buildNumber,
          timestamp: Date.now(),
        })
      );
    } catch {
      // Ignore cache errors
    }
  }

  function updateVersionDisplay(version, buildNumber) {
    const el = document.getElementById("statusBarVersion");
    if (el) {
      if (buildNumber && buildNumber !== "0") {
        el.textContent = `v${version} (build ${buildNumber})`;
      } else {
        el.textContent = `v${version}`;
      }
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
      const buildNumber = data.build_number || data.data?.build_number;
      return { version, buildNumber };
    } catch (err) {
      console.warn("Failed to fetch version:", err);
      return null;
    }
  }

  async function initStatusBar() {
    // Try cache first for instant display
    const cached = getCachedVersionInfo();
    if (cached) {
      updateVersionDisplay(cached.version, cached.buildNumber);
    }

    // Fetch fresh version (updates cache)
    const info = await fetchVersionInfo();
    if (info && info.version) {
      updateVersionDisplay(info.version, info.buildNumber);
      setCachedVersionInfo(info.version, info.buildNumber);
    } else if (!cached) {
      // Fallback if no cache and fetch failed
      updateVersionDisplay("—", null);
    }
  }

  // Initialize when DOM is ready
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initStatusBar);
  } else {
    initStatusBar();
  }
})();
