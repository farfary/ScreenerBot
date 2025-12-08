// Status Bar - Fetches and displays version info
// Non-module script for immediate execution

(function () {
  "use strict";

  // In-memory cache only (no localStorage needed for version info)
  let cachedVersion = null;

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
    // Use cached version if available (same session)
    if (cachedVersion) {
      updateVersionDisplay(cachedVersion);
      return;
    }

    // Fetch version from server
    const info = await fetchVersionInfo();
    if (info && info.version) {
      cachedVersion = info.version;
      updateVersionDisplay(info.version);
    } else {
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
