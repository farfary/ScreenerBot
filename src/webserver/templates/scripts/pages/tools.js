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
import {
  renderTradeWatcherTool,
} from "./tools/trading_tools.js";
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
    label: "Coming Soon",
    cssClass: "coming-soon",
    dataStatus: "coming",
    tooltip: "Coming soon",
  },
  [FEATURE_STATUS.BETA]: {
    label: "Beta",
    cssClass: "beta",
    dataStatus: "beta",
    tooltip: "Beta - may have bugs",
  },
  [FEATURE_STATUS.DISABLED]: {
    label: "Disabled",
    cssClass: "disabled",
    dataStatus: "disabled",
    tooltip: "Currently disabled",
  },
};

/**
 * Tool definitions with metadata and content generators
 */
const TOOL_DEFINITIONS = {
  "wallet-cleanup": {
    id: "wallet-cleanup",
    title: "Wallet Cleanup",
    description: "Close empty Associated Token Accounts to reclaim SOL",
    icon: "icon-trash-2",
    category: "wallet",
    render: renderWalletCleanupTool,
  },
  "burn-tokens": {
    id: "burn-tokens",
    title: "Burn Tokens",
    description: "Permanently destroy tokens from your wallet",
    icon: "icon-flame",
    category: "wallet",
    render: renderBurnTokensTool,
  },
  "token-analyzer": {
    id: "token-analyzer",
    title: "Token Analyzer",
    description: "Deep analysis of any Solana token with multi-dimensional insights",
    icon: "icon-search",
    category: "token",
    render: renderTokenAnalyzerTool,
  },
  "create-token": {
    id: "create-token",
    title: "Create Token",
    description: "Deploy a new SPL token on Solana",
    icon: "icon-circle-plus",
    category: "token",
    render: renderCreateTokenTool,
  },
  "token-watch": {
    id: "token-watch",
    title: "Holder Watch",
    description: "Track and monitor new token holders in real-time",
    icon: "icon-eye",
    category: "single-token",
    render: renderTokenWatchTool,
  },
  "trade-watcher": {
    id: "trade-watcher",
    title: "Trade Watcher",
    description: "Monitor token trades and trigger automatic buy/sell actions",
    icon: "icon-activity",
    category: "single-token",
    render: renderTradeWatcherTool,
  },
  "buy-multi-wallets": {
    id: "buy-multi-wallets",
    title: "Multi-Buy",
    description: "Execute coordinated buy orders across multiple wallets with randomized amounts",
    icon: "icon-shopping-cart",
    category: "single-token",
    render: renderBuyMultiWalletsTool,
  },
  "sell-multi-wallets": {
    id: "sell-multi-wallets",
    title: "Multi-Sell",
    description: "Execute coordinated sell orders across multiple wallets with SOL consolidation",
    icon: "icon-package",
    category: "single-token",
    render: renderSellMultiWalletsTool,
  },
  "wallet-consolidation": {
    id: "wallet-consolidation",
    title: "Wallet Consolidation",
    description: "Consolidate SOL and tokens from sub-wallets back to main wallet",
    icon: "icon-git-merge",
    category: "utilities",
    render: renderWalletConsolidationTool,
  },
  "airdrop-checker": {
    id: "airdrop-checker",
    title: "Airdrop Checker",
    description: "Check for pending airdrops and claimable rewards",
    icon: "icon-gift",
    category: "more",
    render: renderAirdropCheckerTool,
  },
  "wallet-generator": {
    id: "wallet-generator",
    title: "Wallet Generator",
    description: "Generate new Solana keypairs securely",
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
      const statusIndicator = navItem.querySelector(".nav-item-status");
      if (statusIndicator) {
        statusIndicator.dataset.tooltip = "Ready to use";
      }
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

    // Update status indicator tooltip
    const statusIndicator = navItem.querySelector(".nav-item-status");
    if (statusIndicator) {
      statusIndicator.dataset.tooltip = config.tooltip;
    }

    // Add status badge for non-available tools
    if (status !== FEATURE_STATUS.AVAILABLE) {
      const badge = document.createElement("span");
      badge.className = `status-badge ${config.cssClass}`;
      badge.textContent = config.label;
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
  if (titleEl) titleEl.textContent = definition.title;
  if (descEl) descEl.textContent = definition.description;

  // Render tool content
  const contentEl = $("#tools-content");
  const actionsEl = $("#tool-actions");

  if (contentEl && actionsEl && definition.render) {
    contentEl.innerHTML = "";
    actionsEl.innerHTML = "";
    definition.render(contentEl, actionsEl);

    // Enhance any native select elements with custom styling
    enhanceAllSelects(contentEl);
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
    async init() {
      // Initialize hints system
      await Hints.init();

      // Fetch feature status from API and apply to UI
      featureStatus = await fetchFeatureStatus();
      applyFeatureStatusToUI();

      // Set up tool navigation click handler
      toolClickHandler = (event) => {
        const toolItem = event.target.closest(".nav-item, .tool-item");
        if (toolItem && toolItem.dataset.tool) {
          const toolId = toolItem.dataset.tool;
          const status = toolItem.dataset.status;

          // Handle non-available tools
          if (status === "coming") {
            Utils.showToast("This tool is coming soon", "info");
            return;
          }
          if (status === "disabled") {
            Utils.showToast("This tool is currently disabled", "warning");
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
    const definition = TOOL_DEFINITIONS[currentTool];
    Utils.showToast(definition?.description || "Help not available", "info");
    return;
  }

  const hint = Hints.getHint(hintPath);
  if (!hint) {
    Utils.showToast("Help not available", "info");
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
