/**
 * About Page Module
 * Displays app information, resources, community links, and support
 */

import { registerPage } from "../core/lifecycle.js";
import { $ } from "../core/dom.js";

// =============================================================================
// API Calls
// =============================================================================

async function fetchVersion() {
  try {
    const response = await fetch("/api/version");
    const data = await response.json();
    return data;
  } catch (error) {
    console.error("Failed to fetch version:", error);
    return null;
  }
}

// =============================================================================
// UI Updates
// =============================================================================

function updateVersionInfo(versionData) {
  const versionEl = $("#appVersion");
  const platformEl = $("#appPlatform");

  if (versionEl && versionData?.version) {
    versionEl.textContent = versionData.version;
  }

  if (platformEl && versionData?.platform) {
    platformEl.textContent = versionData.platform;
  }
}

// =============================================================================
// Lifecycle
// =============================================================================

export function createLifecycle() {
  return {
    async init() {
      // Fetch and display version info
      const versionData = await fetchVersion();
      if (versionData) {
        updateVersionInfo(versionData);
      }
    },

    activate() {
      // No pollers or active monitoring needed
    },

    deactivate() {
      // Nothing to clean up
    },

    dispose() {
      // Nothing to dispose
    },
  };
}

// Register the page
registerPage("about", createLifecycle());
