/**
 * Contextual Hints System
 *
 * Central registry of all hint definitions for the dashboard.
 * Hints are organized by page/feature and can be toggled globally
 * or dismissed individually.
 */

// Global hints enabled state (loaded from settings)
let hintsEnabled = true;

// Set of dismissed hint IDs (loaded from UI state)
let dismissedHints = new Set();

// Initialization promise
let initPromise = null;

/**
 * Initialize hints system - load settings and dismissed state
 */
export async function init() {
  if (initPromise) return initPromise;

  initPromise = (async () => {
    try {
      // Load GUI config for global toggle
      const configResponse = await fetch("/api/config/gui");
      if (configResponse.ok) {
        const result = await configResponse.json();
        const config = result.data?.data || result.data || result;
        hintsEnabled = config?.dashboard?.interface?.show_hints !== false;
      }

      // Load dismissed hints from UI state
      const stateResponse = await fetch("/api/ui-state/all");
      if (stateResponse.ok) {
        const state = await stateResponse.json();
        const dismissed = state["dismissed_hints"];
        if (Array.isArray(dismissed)) {
          dismissedHints = new Set(dismissed);
        }
      }
    } catch (e) {
      console.warn("[Hints] Failed to load hints state:", e);
    }
  })();

  return initPromise;
}

/**
 * Check if hints are globally enabled
 */
export function isEnabled() {
  return hintsEnabled;
}

/**
 * Set global hints enabled state
 */
export function setEnabled(enabled) {
  hintsEnabled = enabled;
  // Trigger re-render of visible hints
  document.dispatchEvent(new CustomEvent("hints:toggle", { detail: { enabled } }));
}

/**
 * Check if a specific hint has been dismissed
 */
export function isDismissed(hintId) {
  return dismissedHints.has(hintId);
}

/**
 * Dismiss a specific hint (don't show again)
 */
export async function dismissHint(hintId) {
  dismissedHints.add(hintId);

  // Persist to server
  try {
    await fetch("/api/ui-state/save", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        key: "dismissed_hints",
        value: Array.from(dismissedHints),
      }),
    });
  } catch (e) {
    console.warn("[Hints] Failed to save dismissed hints:", e);
  }
}

/**
 * Restore a single dismissed hint (make it show again)
 */
export async function undismissHint(hintId) {
  if (!dismissedHints.has(hintId)) return;
  dismissedHints.delete(hintId);

  try {
    await fetch("/api/ui-state/save", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        key: "dismissed_hints",
        value: Array.from(dismissedHints),
      }),
    });
  } catch (e) {
    console.warn("[Hints] Failed to restore hint:", e);
  }
}

/**
 * Reset all dismissed hints
 */
export async function resetDismissedHints() {
  dismissedHints.clear();

  try {
    await fetch("/api/ui-state/save", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        key: "dismissed_hints",
        value: [],
      }),
    });
  } catch (e) {
    console.warn("[Hints] Failed to reset dismissed hints:", e);
  }
}

/**
 * Get the set of dismissed hint IDs as an array
 */
export function getDismissedHints() {
  return Array.from(dismissedHints);
}

/**
 * Message ids of the labels for the top-level HINTS categories
 */
export const HINT_CATEGORY_LABELS = Object.freeze({
  tokens: "hints-category-tokens",
  positions: "hints-category-positions",
  filtering: "hints-category-filtering",
  trader: "hints-category-trader",
  services: "hints-category-services",
  wallet: "hints-category-wallet",
  wallets: "hints-category-wallets",
  tools: "hints-category-tools",
  config: "hints-category-config",
  configTelegram: "hints-category-config-telegram",
  tokenDetails: "hints-category-token-details",
  ui: "hints-category-ui",
});

/**
 * Flatten the HINTS registry into a list of hint entries, grouped by category.
 * Returns: [{ category, categoryLabel, hints: [{ path, id, title, content, learnMoreUrl }] }]
 */
export function getAllHintGroups() {
  const groups = [];
  for (const [category, hints] of Object.entries(HINTS)) {
    const entries = [];
    for (const [key, hint] of Object.entries(hints)) {
      if (!hint || !hint.id) continue;
      entries.push({
        path: `${category}.${key}`,
        id: hint.id,
        title: hint.title,
        content: hint.content,
        learnMoreUrl: hint.learnMoreUrl,
      });
    }
    if (entries.length) {
      groups.push({
        category,
        categoryLabel: Object.hasOwn(HINT_CATEGORY_LABELS, category)
          ? I18n.label(HINT_CATEGORY_LABELS, category)
          : category,
        hints: entries,
      });
    }
  }
  return groups;
}

/**
 * Resolve a hint's `title` and `content` message ids to text on every read, so
 * consumers keep reading plain `hint.title` / `hint.content` strings.
 */
function localizedHint(hint) {
  const messages = Object.freeze({ title: hint.title, content: hint.content });
  return {
    ...hint,
    get title() {
      return I18n.label(messages, "title");
    },
    // The body is markdown source. Its only placeables escape a line-leading
    // "*", so the isolation marks Fluent wraps around them are dropped to keep
    // the emphasis markers adjacent.
    get content() {
      return I18n.label(messages, "content").replace(/[\u2068\u2069]/g, "");
    },
  };
}

function localizeRegistry(registry) {
  return Object.fromEntries(
    Object.entries(registry).map(([category, hints]) => [
      category,
      Object.fromEntries(Object.entries(hints).map(([key, hint]) => [key, hint?.id ? localizedHint(hint) : hint])),
    ])
  );
}

/**
 * Hint definitions registry
 * Organized by page/feature for easy maintenance. Titles and bodies are
 * message ids in `locales/en/hints.ftl`.
 */
export const HINTS = localizeRegistry({
  // ═══════════════════════════════════════════════════════════════════════════
  // TOKENS PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  tokens: {
    poolService: {
      id: "tokens.pool_service",
      title: "hints-tokens-pool-service-title",
      content: "hints-tokens-pool-service-content",
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/tokens",
    },

    noMarketData: {
      id: "tokens.no_market",
      title: "hints-tokens-no-market-title",
      content: "hints-tokens-no-market-content",
    },

    allTokens: {
      id: "tokens.all",
      title: "hints-tokens-all-title",
      content: "hints-tokens-all-content",
    },

    passedTokens: {
      id: "tokens.passed",
      title: "hints-tokens-passed-title",
      content: "hints-tokens-passed-content",
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/filtering",
    },

    rejectedTokens: {
      id: "tokens.rejected",
      title: "hints-tokens-rejected-title",
      content: "hints-tokens-rejected-content",
    },

    blacklistedTokens: {
      id: "tokens.blacklisted",
      title: "hints-tokens-blacklisted-title",
      content: "hints-tokens-blacklisted-content",
    },

    positionsTokens: {
      id: "tokens.positions",
      title: "hints-tokens-positions-title",
      content: "hints-tokens-positions-content",
    },

    recentTokens: {
      id: "tokens.recent",
      title: "hints-tokens-recent-title",
      content: "hints-tokens-recent-content",
    },

    ohlcvData: {
      id: "tokens.ohlcv",
      title: "hints-tokens-ohlcv-title",
      content: "hints-tokens-ohlcv-content",
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // POSITIONS PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  positions: {
    overview: {
      id: "positions.overview",
      title: "hints-positions-overview-title",
      content: "hints-positions-overview-content",
    },

    dca: {
      id: "positions.dca",
      title: "hints-positions-dca-title",
      content: "hints-positions-dca-content",
      learnMoreUrl: "https://screenerbot.io/docs/trading/dca-guide",
    },

    partialExit: {
      id: "positions.partial_exit",
      title: "hints-positions-partial-exit-title",
      content: "hints-positions-partial-exit-content",
    },

    positionManagement: {
      id: "positions.management",
      title: "hints-positions-management-title",
      content: "hints-positions-management-content",
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // FILTERING PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  filtering: {
    overview: {
      id: "filtering.overview",
      title: "hints-filtering-overview-title",
      content: "hints-filtering-overview-content",
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/filtering",
    },

    dexscreener: {
      id: "filtering.dexscreener",
      title: "hints-filtering-dexscreener-title",
      content: "hints-filtering-dexscreener-content",
    },

    geckoterminal: {
      id: "filtering.geckoterminal",
      title: "hints-filtering-geckoterminal-title",
      content: "hints-filtering-geckoterminal-content",
    },

    rugcheck: {
      id: "filtering.rugcheck",
      title: "hints-filtering-rugcheck-title",
      content: "hints-filtering-rugcheck-content",
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/filtering",
    },

    meta: {
      id: "filtering.meta",
      title: "hints-filtering-meta-title",
      content: "hints-filtering-meta-content",
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // TRADER PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  trader: {
    overview: {
      id: "trader.overview",
      title: "hints-trader-overview-title",
      content: "hints-trader-overview-content",
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/trader",
    },

    entryMonitor: {
      id: "trader.entry",
      title: "hints-trader-entry-title",
      content: "hints-trader-entry-content",
    },

    exitMonitor: {
      id: "trader.exit",
      title: "hints-trader-exit-title",
      content: "hints-trader-exit-content",
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // SERVICES PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  services: {
    overview: {
      id: "services.overview",
      title: "hints-services-overview-title",
      content: "hints-services-overview-content",
    },

    health: {
      id: "services.health",
      title: "hints-services-health-title",
      content: "hints-services-health-content",
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // WALLET PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  wallet: {
    overview: {
      id: "wallet.overview",
      title: "hints-wallet-overview-title",
      content: "hints-wallet-overview-content",
    },

    tokens: {
      id: "wallet.tokens",
      title: "hints-wallet-tokens-title",
      content: "hints-wallet-tokens-content",
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // WALLETS PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  wallets: {
    mainWallet: {
      id: "wallets.main",
      title: "hints-wallets-main-title",
      content: "hints-wallets-main-content",
    },

    secondaryWallets: {
      id: "wallets.secondary",
      title: "hints-wallets-secondary-title",
      content: "hints-wallets-secondary-content",
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // TOOLS PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  tools: {
    walletCleanup: {
      id: "tools.wallet_cleanup",
      title: "hints-tools-wallet-cleanup-title",
      content: "hints-tools-wallet-cleanup-content",
    },

    burnTokens: {
      id: "tools.burn_tokens",
      title: "hints-tools-burn-tokens-title",
      content: "hints-tools-burn-tokens-content",
    },

    walletGenerator: {
      id: "tools.wallet_generator",
      title: "hints-tools-wallet-generator-title",
      content: "hints-tools-wallet-generator-content",
    },

    multiBuy: {
      id: "tools.multi_buy",
      title: "hints-tools-multi-buy-title",
      content: "hints-tools-multi-buy-content",
      learnMoreUrl: "https://screenerbot.io/docs/tools/multi-buy",
    },

    multiSell: {
      id: "tools.multi_sell",
      title: "hints-tools-multi-sell-title",
      content: "hints-tools-multi-sell-content",
      learnMoreUrl: "https://screenerbot.io/docs/tools/multi-sell",
    },

    tradeWatcher: {
      id: "tools.trade_watcher",
      title: "hints-tools-trade-watcher-title",
      content: "hints-tools-trade-watcher-content",
      learnMoreUrl: "https://screenerbot.io/docs/tools/trade-watcher",
    },

    walletConsolidation: {
      id: "tools.wallet_consolidation",
      title: "hints-tools-wallet-consolidation-title",
      content: "hints-tools-wallet-consolidation-content",
      learnMoreUrl: "https://screenerbot.io/docs/tools/consolidation",
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // CONFIG PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  config: {
    overview: {
      id: "config.overview",
      title: "hints-config-overview-title",
      content: "hints-config-overview-content",
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/system/config",
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // CONFIG PAGE - TELEGRAM
  // ═══════════════════════════════════════════════════════════════════════════
  configTelegram: {
    overview: {
      id: "config.telegram",
      title: "hints-config-telegram-title",
      content: "hints-config-telegram-content",
      learnMoreUrl: "https://screenerbot.io/docs/config/telegram",
    },
    password: {
      id: "config.telegram.password",
      title: "hints-config-telegram-password-title",
      content: "hints-config-telegram-password-content",
      learnMoreUrl: "https://screenerbot.io/docs/config/telegram",
    },
    totp: {
      id: "config.telegram.totp",
      title: "hints-config-telegram-totp-title",
      content: "hints-config-telegram-totp-content",
      learnMoreUrl: "https://screenerbot.io/docs/config/telegram",
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // TOKEN DETAILS DIALOG
  // ═══════════════════════════════════════════════════════════════════════════
  tokenDetails: {
    chart: {
      id: "token_details.chart",
      title: "hints-token-details-chart-title",
      content: "hints-token-details-chart-content",
      learnMoreUrl: "https://screenerbot.io/docs/concepts/pricing",
    },

    tokenInfo: {
      id: "token_details.token_info",
      title: "hints-token-details-token-info-title",
      content: "hints-token-details-token-info-content",
    },

    liquidity: {
      id: "token_details.liquidity",
      title: "hints-token-details-liquidity-title",
      content: "hints-token-details-liquidity-content",
    },

    marketPulse: {
      id: "token_details.market_pulse",
      title: "hints-token-details-market-pulse-title",
      content: "hints-token-details-market-pulse-content",
    },

    activity: {
      id: "token_details.activity",
      title: "hints-token-details-activity-title",
      content: "hints-token-details-activity-content",
    },

    security: {
      id: "token_details.security",
      title: "hints-token-details-security-title",
      content: "hints-token-details-security-content",
      learnMoreUrl: "https://screenerbot.io/docs/concepts/security",
    },

    pools: {
      id: "token_details.pools",
      title: "hints-token-details-pools-title",
      content: "hints-token-details-pools-content",
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // UI COMPONENTS
  // ═══════════════════════════════════════════════════════════════════════════
  ui: {
    featured: {
      id: "ui.featured",
      title: "hints-ui-featured-title",
      content: "hints-ui-featured-content",
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/featured",
    },
  },
});

/**
 * Get a hint by its path (e.g., "tokens.poolService")
 */
export function getHint(path) {
  const parts = path.split(".");
  let current = HINTS;

  for (const part of parts) {
    if (current && typeof current === "object" && part in current) {
      current = current[part];
    } else {
      return null;
    }
  }

  return current;
}

/**
 * Get all hints for a page
 */
export function getPageHints(page) {
  return HINTS[page] || {};
}
