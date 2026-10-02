// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Trader Feature Status Module
 *
 * Manages feature flags and restrictions for trader tabs.
 * Handles feature status display (available, coming soon, beta, disabled).
 */

/**
 * Feature status values from the API
 */
export const FEATURE_STATUS = {
  AVAILABLE: "available",
  COMING_SOON: "coming_soon",
  BETA: "beta",
  DISABLED: "disabled",
};

/**
 * Maps tab IDs to feature API keys (trading features)
 */
export const TAB_TO_FEATURE_MAP = {
  roi: "roi_exit",
  "trailing-stop": "trailing_stop",
  "stop-loss": "stop_loss",
  "time-rules": "time_override",
  dca: "dca",
  "strategy-control": "strategies",
  // These tabs don't have feature flags - always available
  stats: null,
  "general-settings": null,
};

/**
 * Status display configuration for badges
 */
export const STATUS_CONFIG = {
  [FEATURE_STATUS.COMING_SOON]: { cssClass: "coming-soon" },
  [FEATURE_STATUS.BETA]: { cssClass: "beta" },
  [FEATURE_STATUS.DISABLED]: { cssClass: "disabled" },
};

/**
 * Badge label and the toast message of a restricted status. Beta features are
 * usable, so they carry no message.
 */
function statusText(status) {
  switch (status) {
    case FEATURE_STATUS.COMING_SOON:
      return {
        label: I18n.t("trader-feature-coming-soon"),
        message: I18n.attr("trader-feature-coming-soon", "message"),
      };
    case FEATURE_STATUS.BETA:
      return { label: I18n.t("trader-feature-beta"), message: null };
    case FEATURE_STATUS.DISABLED:
      return {
        label: I18n.t("trader-feature-disabled"),
        message: I18n.attr("trader-feature-disabled", "message"),
      };
    default:
      return null;
  }
}

/**
 * Fetch trading feature status from the API
 * @param {Object} requestManager - Request manager instance (not used, kept for future)
 * @returns {Promise<Object>} Feature status by feature key
 */
export async function fetchFeatureStatus(_requestManager) {
  try {
    const response = await fetch("/api/features");
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const data = await response.json();
    return data.trading || {};
  } catch (error) {
    console.warn("[Trader] Failed to fetch feature status, defaulting to available:", error);
    return {};
  }
}

/**
 * Get the feature status for a tab
 * @param {Object} tradingFeatures - Feature status object from API
 * @param {string} tabId - The tab ID (e.g., "roi", "stop-loss")
 * @returns {string} The status ("available", "coming_soon", "beta", "disabled")
 */
export function getTabFeatureStatus(tradingFeatures, tabId) {
  const featureKey = TAB_TO_FEATURE_MAP[tabId];
  // Tabs without feature mapping are always available
  if (!featureKey) {
    return FEATURE_STATUS.AVAILABLE;
  }
  if (!tradingFeatures[featureKey]) {
    return FEATURE_STATUS.AVAILABLE;
  }
  return tradingFeatures[featureKey];
}

/**
 * Check if a tab is usable (can be clicked/used)
 * @param {Object} tradingFeatures - Feature status object from API
 * @param {string} tabId - The tab ID
 * @returns {boolean} True if tab is available or beta
 */
export function isTabUsable(tradingFeatures, tabId) {
  const status = getTabFeatureStatus(tradingFeatures, tabId);
  return status === FEATURE_STATUS.AVAILABLE || status === FEATURE_STATUS.BETA;
}

/**
 * Apply feature status to all tab buttons
 * @param {Object} tradingFeatures - Feature status object from API
 * @param {Function} $$ - DOM selector function (select all)
 */
export function applyFeatureStatusToTabs(tradingFeatures, $$) {
  const tabButtons = $$(".sub-tab[data-tab-id]");

  tabButtons.forEach((button) => {
    const tabId = button.getAttribute("data-tab-id");
    const status = getTabFeatureStatus(tradingFeatures, tabId);

    // Remove any existing status badges
    const existingBadge = button.querySelector(".tab-status-badge");
    if (existingBadge) {
      existingBadge.remove();
    }

    // Remove existing status classes
    button.classList.remove("tab-feature-disabled", "tab-feature-beta", "tab-feature-coming-soon");
    button.removeAttribute("data-feature-status");

    // If available, no modifications needed
    if (status === FEATURE_STATUS.AVAILABLE) {
      return;
    }

    // Get status configuration
    const config = STATUS_CONFIG[status];
    if (!config) return;

    // Add data attribute for styling
    button.setAttribute("data-feature-status", status);

    // Add appropriate class
    if (status === FEATURE_STATUS.DISABLED) {
      button.classList.add("tab-feature-disabled");
    } else if (status === FEATURE_STATUS.BETA) {
      button.classList.add("tab-feature-beta");
    } else if (status === FEATURE_STATUS.COMING_SOON) {
      button.classList.add("tab-feature-coming-soon");
    }

    // Every restricted status, beta included, gets a badge.
    const badge = document.createElement("span");
    badge.className = `tab-status-badge ${config.cssClass}`;
    badge.textContent = statusText(status).label;
    button.appendChild(badge);
  });
}

/**
 * Handle click on a feature-restricted tab
 * @param {Object} tradingFeatures - Feature status object from API
 * @param {string} tabId - The tab ID being clicked
 * @param {Object} Utils - Utils module with showToast
 * @returns {boolean} True if tab switch should proceed
 */
export function handleFeatureRestrictedTab(tradingFeatures, tabId, Utils) {
  const status = getTabFeatureStatus(tradingFeatures, tabId);

  // Available and beta tabs can proceed
  if (status === FEATURE_STATUS.AVAILABLE || status === FEATURE_STATUS.BETA) {
    return true;
  }

  // Show toast for restricted tabs
  const text = statusText(status);
  if (text && text.message) {
    Utils.showToast({
      type: "warning",
      title: text.label,
      message: text.message,
    });
  }

  return false;
}
