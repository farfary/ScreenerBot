// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Tools Page Module
 * Provides utility tools for wallet management, token operations, and trading
 */

import { registerPage } from "../core/lifecycle.js";
import { $, $$, on, off } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import * as Hints from "../core/hints.js";
import { enhanceAllSelects } from "../ui/custom_select.js";
import { gateControl, renderSetupGate, setupRequired } from "../ui/setup_gate.js";

// Import tool modules
import {
  renderWalletCleanupTool,
  renderBurnTokensTool,
  renderWalletConsolidationTool,
  renderAirdropCheckerTool,
  renderWalletGeneratorTool,
} from "./tools/wallet_tools.js";
import {
  renderCreateTokenTool,
  renderTokenWatchTool,
  renderTokenAnalyzerTool,
} from "./tools/token_tools.js";
import { cleanupTradeWatcher, renderTradeWatcherTool } from "./tools/trading_tools.js";
import {
  renderBuyMultiWalletsTool,
  renderSellMultiWalletsTool,
  stopMultiBuyPolling,
  resetMultiBuyUI,
  stopMultiSellPolling,
  resetMultiSellUI,
} from "./tools/multi_wallet_tools.js";

// =============================================================================
// Constants
// =============================================================================

const TOOLS_STATE_KEY = "tools.page";
const DEFAULT_TOOL = "wallet-cleanup";

/**
 * Feature status values from the API
 */
const FEATURE_STATUS = {
  AVAILABLE: "available",
  COMING_SOON: "coming_soon",
  BETA: "beta",
  DISABLED: "disabled",
};

/**
 * Maps tool IDs (from HTML data-tool) to feature API keys
 */
const TOOL_TO_FEATURE_MAP = {
  "wallet-cleanup": "wallet_cleanup",
  "burn-tokens": "burn_tokens",
  "token-analyzer": "token_analyzer",
  "create-token": "create_token",
  "trade-watcher": "trade_watcher",
  "token-watch": "holder_watch",
  "buy-multi-wallets": "multi_buy",
  "sell-multi-wallets": "multi_sell",
  "wallet-consolidation": "wallet_consolidation",
  "airdrop-checker": "airdrop_checker",
  "wallet-generator": "wallet_generator",
};

/**
 * Status display configuration
 */
const STATUS_CONFIG = {
  [FEATURE_STATUS.COMING_SOON]: {
    cssClass: "coming-soon",
    dataStatus: "coming",
  },
  [FEATURE_STATUS.BETA]: {
    cssClass: "beta",
    dataStatus: "beta",
  },
  [FEATURE_STATUS.DISABLED]: {
    cssClass: "disabled",
    dataStatus: "disabled",
  },
};

// Message key of the status dot tooltip, by nav item `data-status`.
const STATUS_TOOLTIP_LABELS = Object.freeze({
  ready: "tools-status-ready",
  coming: "tools-status-coming",
  beta: "tools-status-beta",
  disabled: "tools-status-disabled",
});

// Message key of the badge of a non-available tool, by `data-status`.
const STATUS_BADGE_LABELS = Object.freeze({
  coming: "tools-status-badge-coming",
  beta: "tools-status-badge-beta",
  disabled: "common-state-disabled",
});

// Message key of each tool's title and header description, by tool id.
const TOOL_TITLE_LABELS = Object.freeze({
  "wallet-cleanup": "tools-tool-wallet-cleanup-title",
  "burn-tokens": "tools-tool-burn-tokens-title",
  "token-analyzer": "tools-tool-token-analyzer-title",
  "create-token": "tools-tool-create-token-title",
  "token-watch": "tools-tool-token-watch-title",
  "trade-watcher": "tools-tool-trade-watcher-title",
  "buy-multi-wallets": "tools-tool-buy-multi-wallets-title",
  "sell-multi-wallets": "tools-tool-sell-multi-wallets-title",
  "wallet-consolidation": "tools-tool-wallet-consolidation-title",
  "airdrop-checker": "tools-tool-airdrop-checker-title",
  "wallet-generator": "tools-tool-wallet-generator-title",
});
const TOOL_DESCRIPTION_LABELS = Object.freeze({
  "wallet-cleanup": "tools-tool-wallet-cleanup-description",
  "burn-tokens": "tools-tool-burn-tokens-description",
  "token-analyzer": "tools-tool-token-analyzer-description",
  "create-token": "tools-tool-create-token-description",
  "token-watch": "tools-tool-token-watch-description",
  "trade-watcher": "tools-tool-trade-watcher-description",
  "buy-multi-wallets": "tools-tool-buy-multi-wallets-description",
  "sell-multi-wallets": "tools-tool-sell-multi-wallets-description",
  "wallet-consolidation": "tools-tool-wallet-consolidation-description",
  "airdrop-checker": "tools-tool-airdrop-checker-description",
  "wallet-generator": "tools-tool-wallet-generator-description",
});

/**
 * Tool definitions with metadata and content generators. `wallet` marks a tool that
 * reads or signs with the user's wallet, which Explore Mode does not have.
 */
const TOOL_DEFINITIONS = {
  "wallet-cleanup": {
    id: "wallet-cleanup",
    icon: "icon-trash-2",
    category: "wallet",
    wallet: true,
    render: renderWalletCleanupTool,
  },
  "burn-tokens": {
    id: "burn-tokens",
    icon: "icon-flame",
    category: "wallet",
    wallet: true,
    render: renderBurnTokensTool,
  },
  "token-analyzer": {
    id: "token-analyzer",
    icon: "icon-search",
    category: "token",
    render: renderTokenAnalyzerTool,
  },
  "create-token": {
    id: "create-token",
    icon: "icon-circle-plus",
    category: "token",
    wallet: true,
    render: renderCreateTokenTool,
  },
  "token-watch": {
    id: "token-watch",
    icon: "icon-eye",
    category: "single-token",
    render: renderTokenWatchTool,
  },
  "trade-watcher": {
    id: "trade-watcher",
    icon: "icon-activity",
    category: "single-token",
    render: renderTradeWatcherTool,
  },
  "buy-multi-wallets": {
    id: "buy-multi-wallets",
    icon: "icon-shopping-cart",
    category: "single-token",
    wallet: true,
    render: renderBuyMultiWalletsTool,
  },
  "sell-multi-wallets": {
    id: "sell-multi-wallets",
    icon: "icon-package",
    category: "single-token",
    wallet: true,
    render: renderSellMultiWalletsTool,
  },
  "wallet-consolidation": {
    id: "wallet-consolidation",
    icon: "icon-git-merge",
    category: "utilities",
    wallet: true,
    render: renderWalletConsolidationTool,
  },
  "airdrop-checker": {
    id: "airdrop-checker",
    icon: "icon-gift",
    category: "more",
    wallet: true,
    render: renderAirdropCheckerTool,
  },
  "wallet-generator": {
    id: "wallet-generator",
    icon: "icon-key",
    category: "more",
    render: renderWalletGeneratorTool,
  },
};

// =============================================================================
// State
// =============================================================================

let currentTool = null;
let toolClickHandler = null;
let featureStatus = {}; // Stores feature status from API

// =============================================================================
// Feature Status Functions
// =============================================================================

/**
 * Fetch feature status from the API
 * @returns {Promise<Object>} Feature status by tool key
 */
async function fetchFeatureStatus() {
  try {
    const response = await fetch("/api/features");
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const data = await response.json();
    return data.tools || {};
  } catch (error) {
    console.warn("Failed to fetch feature status, defaulting to available:", error);
    // Default to all available if API fails
    return {};
  }
}

/**
 * Get the feature status for a tool
 * @param {string} toolId - The tool ID (e.g., "wallet-cleanup")
 * @returns {string} The status ("available", "coming_soon", "beta", "disabled")
 */
function getToolFeatureStatus(toolId) {
  const featureKey = TOOL_TO_FEATURE_MAP[toolId];
  if (!featureKey || !featureStatus[featureKey]) {
    return FEATURE_STATUS.AVAILABLE;
  }
  return featureStatus[featureKey];
}

/**
 * Set the status dot tooltip of a navigation item from its `data-status`
 */
function applyStatusTooltip(navItem) {
  const indicator = navItem.querySelector(".nav-item-status");
  if (indicator) {
    indicator.dataset.tooltip = I18n.label(STATUS_TOOLTIP_LABELS, navItem.dataset.status);
  }
}

/**
 * Apply feature status to all tool navigation items
 */
function applyFeatureStatusToUI() {
  const navItems = $$(".nav-item[data-tool]");

  navItems.forEach((navItem) => {
    const toolId = navItem.dataset.tool;
    const status = getToolFeatureStatus(toolId);

    // Remove any existing status badges
    const existingBadge = navItem.querySelector(".status-badge");
    if (existingBadge) {
      existingBadge.remove();
    }

    // If available, ensure clean state
    if (status === FEATURE_STATUS.AVAILABLE) {
      navItem.dataset.status = "ready";
      navItem.classList.remove("feature-disabled", "feature-beta", "feature-coming-soon");
      applyStatusTooltip(navItem);
      return;
    }

    // Get status configuration
    const config = STATUS_CONFIG[status];
    if (!config) return;

    // Apply data-status attribute
    navItem.dataset.status = config.dataStatus;

    // Add appropriate class
    navItem.classList.remove("feature-disabled", "feature-beta", "feature-coming-soon");
    if (status === FEATURE_STATUS.DISABLED) {
      navItem.classList.add("feature-disabled");
    } else if (status === FEATURE_STATUS.BETA) {
      navItem.classList.add("feature-beta");
    } else if (status === FEATURE_STATUS.COMING_SOON) {
      navItem.classList.add("feature-coming-soon");
    }

    applyStatusTooltip(navItem);

    // Add status badge for non-available tools
    if (status !== FEATURE_STATUS.AVAILABLE) {
      const badge = document.createElement("span");
      badge.className = `status-badge ${config.cssClass}`;
      badge.textContent = I18n.label(STATUS_BADGE_LABELS, config.dataStatus);
      navItem.appendChild(badge);
    }
  });
}

// =============================================================================
// Tool Renderers
// =============================================================================

// =============================================================================
// Tool Navigation
// =============================================================================

function selectTool(toolId, { historyMode = "push" } = {}) {
  const definition = TOOL_DEFINITIONS[toolId];
  if (!definition) {
    console.warn(`Unknown tool: ${toolId}`);
    return;
  }

  currentTool = toolId;

  // Update sidebar active state - support both old and new class names
  const navItems = $$(".nav-item, .tool-item");
  navItems.forEach((item) => {
    if (item.dataset.tool === toolId) {
      item.classList.add("active");
    } else {
      item.classList.remove("active");
    }
  });

  // Update header
  const iconEl = $("#tool-icon");
  const titleEl = $("#tool-title");
  const descEl = $("#tool-description");

  if (iconEl) iconEl.innerHTML = `<i class="${definition.icon}"></i>`;
  // The "select a tool" prompt is only true while nothing is selected.
  const hintFooter = $(".sidebar-footer");
  if (hintFooter) hintFooter.hidden = true;
  if (titleEl) titleEl.textContent = I18n.label(TOOL_TITLE_LABELS, toolId);
  if (descEl) descEl.textContent = I18n.label(TOOL_DESCRIPTION_LABELS, toolId);

  // Render tool content
  const contentEl = $("#tools-content");
  const actionsEl = $("#tool-actions");

  if (contentEl && actionsEl && definition.render) {
    contentEl.innerHTML = "";
    actionsEl.innerHTML = "";
    definition.render(contentEl, actionsEl);

    if (definition.wallet && setupRequired()) {
      // The actions stay visible, disabled with the reason; the panel states it and
      // links to setup.
      actionsEl.querySelectorAll("button").forEach(gateControl);
      renderSetupGate(contentEl, I18n.t("tools-setup-gate-title"));
    } else {
      // Enhance any native select elements with custom styling
      enhanceAllSelects(contentEl);
    }
  }

  // Save state
  saveToolState(toolId);
  if (window.location.hash !== `#${toolId}`) {
    window.history[historyMode === "push" ? "pushState" : "replaceState"](
      { page: "tools", subtab: toolId },
      "",
      `#${toolId}`
    );
  }
}

function saveToolState(toolId) {
  AppState.save(TOOLS_STATE_KEY, toolId);
}

function loadToolState() {
  const hashTool = window.location.hash.slice(1);
  if (TOOL_DEFINITIONS[hashTool]) return hashTool;
  const savedTool = AppState.load(TOOLS_STATE_KEY, DEFAULT_TOOL);
  return TOOL_DEFINITIONS[savedTool] ? savedTool : DEFAULT_TOOL;
}

// =============================================================================
// Lifecycle
// =============================================================================

function createLifecycle() {
  let popstateHandler = null;
  return {
    init() {
      // Hints and feature flags enhance the already-painted tool shell. Neither
      // is allowed to delay the selected tool during a main-tab transition.
      void Hints.init();
      void fetchFeatureStatus().then((status) => {
        featureStatus = status;
        applyFeatureStatusToUI();
      });

      $$(".nav-item[data-tool]").forEach(applyStatusTooltip);

      // Set up tool navigation click handler
      toolClickHandler = (event) => {
        const toolItem = event.target.closest(".nav-item, .tool-item");
        if (toolItem && toolItem.dataset.tool) {
          const toolId = toolItem.dataset.tool;
          const status = toolItem.dataset.status;

          // Handle non-available tools
          if (status === "coming") {
            Utils.showToast(I18n.t("tools-toast-coming-soon"), "info");
            return;
          }
          if (status === "disabled") {
            Utils.showToast(I18n.t("tools-toast-disabled"), "warning");
            return;
          }

          selectTool(toolId);
        }
      };

      const nav = $("#tools-nav");
      if (nav) {
        on(nav, "click", toolClickHandler);
      }

      // Set up help button handler
      const helpBtn = $("#tool-help-btn");
      if (helpBtn) {
        helpBtn.dataset.tooltip = I18n.attr("tools-help-button", "aria-label");
        on(helpBtn, "click", showToolHelp);
      }

      // Load saved state or default
      const savedTool = loadToolState();
      selectTool(savedTool, { historyMode: "replace" });

      popstateHandler = () => {
        const hashTool = window.location.hash.slice(1);
        if (TOOL_DEFINITIONS[hashTool] && hashTool !== currentTool) {
          selectTool(hashTool, { historyMode: "replace" });
        }
      };
      on(window, "popstate", popstateHandler);
    },

    activate() {
      // Re-apply async feature data after a cached page is reattached.
      applyFeatureStatusToUI();
      // Refresh current tool if needed
      if (currentTool) {
        const definition = TOOL_DEFINITIONS[currentTool];
        if (definition && definition.onActivate) {
          definition.onActivate();
        }
      }
    },

    deactivate() {
      // Pause any active operations
    },

    dispose() {
      // Clean up event listeners
      const nav = $("#tools-nav");
      if (nav && toolClickHandler) {
        off(nav, "click", toolClickHandler);
      }
      toolClickHandler = null;
      if (popstateHandler) off(window, "popstate", popstateHandler);
      popstateHandler = null;
      currentTool = null;
      featureStatus = {}; // Reset feature status

      // Clean up Multi-Buy resources
      stopMultiBuyPolling();
      resetMultiBuyUI();

      // Clean up Multi-Sell resources
      stopMultiSellPolling();
      resetMultiSellUI();

      // Clean up Trade Watcher resources
      cleanupTradeWatcher();
    },
  };
}

/**
 * Show help/documentation for current tool using hint popover
 */
function showToolHelp() {
  if (!currentTool) return;

  // Map tool IDs to hint paths
  const hintPathMap = {
    "wallet-cleanup": "tools.walletCleanup",
    "burn-tokens": "tools.burnTokens",
    "wallet-generator": "tools.walletGenerator",
    "buy-multi-wallets": "tools.multiBuy",
    "sell-multi-wallets": "tools.multiSell",
    "wallet-consolidation": "tools.walletConsolidation",
  };

  const hintPath = hintPathMap[currentTool];
  if (!hintPath) {
    // Fallback for tools without hints yet
    Utils.showToast(I18n.label(TOOL_DESCRIPTION_LABELS, currentTool), "info");
    return;
  }

  const hint = Hints.getHint(hintPath);
  if (!hint) {
    Utils.showToast(I18n.t("tools-help-unavailable"), "info");
    return;
  }

  // Find or create a trigger element for the popover
  const helpBtn = $("#tool-help-btn");
  if (helpBtn) {
    // Simulate a click on the hint trigger by creating a temporary one
    import("../ui/hint_popover.js").then(({ HintPopover }) => {
      const popover = new HintPopover(hint, helpBtn);
      popover.show();
    });
  }
}

// Register the page
registerPage("tools", createLifecycle());

export { createLifecycle };
