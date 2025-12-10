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
 * Hint definitions registry
 * Organized by page/feature for easy maintenance
 */
export const HINTS = {
  // ═══════════════════════════════════════════════════════════════════════════
  // TOKENS PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  tokens: {
    poolService: {
      id: "tokens.pool_service",
      title: "Pool Service Tokens",
      content: `Tokens shown here have:

• **Passed all filtering criteria** — liquidity, volume, age, and security checks
• **Valid SOL liquidity pools** — supported by our DEX decoders (Raydium, Orca, Meteora, etc.)
• **Successful price calculation** — prices computed directly from on-chain pool reserves

This is the most reliable token list for trading as prices are derived from actual pool data, not external APIs.

Click any token to view detailed information and manage blacklist status.`,
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/tokens",
    },

    noMarketData: {
      id: "tokens.no_market",
      title: "No Market Data",
      content: `Tokens discovered on-chain but missing market data from DexScreener or GeckoTerminal.

Common reasons:
• **Very new tokens** — not yet indexed by aggregators
• **Low trading volume** — below aggregator thresholds
• **Unlisted pairs** — trading on DEXs not tracked by aggregators

These tokens may still have valid pools and can be traded, but lack external market metrics.`,
    },

    allTokens: {
      id: "tokens.all",
      title: "All Tokens",
      content: `Complete database of discovered tokens regardless of filtering status.

Includes:
• Tokens that passed filtering
• Tokens that were rejected
• Tokens without market data
• Blacklisted tokens

Use this view for research or to find tokens that may have been filtered out.`,
    },

    passedTokens: {
      id: "tokens.passed",
      title: "Passed Filtering",
      content: `Tokens that passed all active filtering criteria.

Filtering checks include:
• **Liquidity** — minimum SOL liquidity threshold
• **Volume** — 24h trading volume requirements
• **Token age** — minimum time since creation
• **Security** — Rugcheck risk score limits
• **Market cap** — optional FDV/MC filters

Configure filters in the **Filtering** page.`,
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/filtering",
    },

    rejectedTokens: {
      id: "tokens.rejected",
      title: "Rejected Tokens",
      content: `Tokens that failed one or more filtering criteria.

Each token shows the specific rejection reason:
• Which filter failed
• The actual value vs required threshold
• When the check occurred

Review rejected tokens to fine-tune your filter settings.`,
    },

    blacklistedTokens: {
      id: "tokens.blacklisted",
      title: "Blacklisted Tokens",
      content: `Tokens permanently excluded from trading.

Blacklist reasons include:
• **Manual blacklist** — tokens you've explicitly blocked
• **Security risks** — detected rug pull indicators
• **Loss threshold** — exceeded configured loss limits
• **Failed transactions** — repeated swap failures

Blacklisted tokens are never shown in passed lists or considered for auto-trading.`,
    },

    positionsTokens: {
      id: "tokens.positions",
      title: "Position Tokens",
      content: `Tokens currently held in open positions.

Shows real-time data for your active holdings:
• Current price from pool reserves
• Unrealized P&L
• Position size and entry price
• Time held

Click any token for detailed position management.`,
    },

    recentTokens: {
      id: "tokens.recent",
      title: "Recently Discovered",
      content: `Newly discovered tokens ordered by discovery time.

Useful for:
• Spotting new token launches
• Monitoring fresh liquidity
• Early entry opportunities

Note: New tokens may lack complete market data initially.`,
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // POSITIONS PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  positions: {
    overview: {
      id: "positions.overview",
      title: "Positions Overview",
      content: `Your current token holdings and trading positions.

Key metrics:
• **Entry Price** — average price paid (including DCA)
• **Current Price** — live price from pool reserves
• **P&L** — unrealized profit/loss in SOL and %
• **Size** — total token amount held

Click any position for detailed management options.`,
    },

    dca: {
      id: "positions.dca",
      title: "DCA (Dollar Cost Average)",
      content: `DCA allows adding to existing positions at different prices.

When DCA is triggered:
• Additional tokens are purchased
• Entry price is recalculated as weighted average
• Position size increases
• Entry count increments

Configure DCA rules in **Auto Trader** settings.`,
      learnMoreUrl: "https://screenerbot.io/docs/trading/dca-guide",
    },

    partialExit: {
      id: "positions.partial_exit",
      title: "Partial Exit",
      content: `Sell a portion of your position while keeping the rest.

Benefits:
• Lock in some profits while staying exposed
• Reduce position size without fully closing
• Implement take-profit ladders

Each partial exit is recorded separately for accurate P&L tracking.`,
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // FILTERING PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  filtering: {
    overview: {
      id: "filtering.overview",
      title: "Token Filtering",
      content: `Filtering determines which tokens are eligible for trading.

Tokens must pass **all enabled criteria** to appear in the passed list:
• DexScreener metrics (liquidity, volume, etc.)
• GeckoTerminal metrics (market cap, FDV)
• Rugcheck security analysis
• Meta filters (token age, etc.)

Disabled criteria are skipped entirely.`,
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/filtering",
    },

    dexscreener: {
      id: "filtering.dexscreener",
      title: "DexScreener Filters",
      content: `Filters based on DexScreener market data:

• **Liquidity** — minimum USD liquidity in pools
• **Volume 24h** — minimum trading volume
• **Transactions** — activity thresholds (buys/sells)
• **Price Change** — volatility filters

DexScreener data updates every few minutes.`,
    },

    geckoterminal: {
      id: "filtering.geckoterminal",
      title: "GeckoTerminal Filters",
      content: `Filters based on GeckoTerminal market data:

• **Market Cap** — minimum market capitalization
• **FDV** — Fully Diluted Valuation limits
• **Reserve Ratio** — pool health indicators

GeckoTerminal often has data for newer tokens.`,
    },

    rugcheck: {
      id: "filtering.rugcheck",
      title: "Security Filters",
      content: `Security analysis from Rugcheck.xyz:

• **Risk Score** — overall risk rating (0-100)
• **Mint Authority** — can new tokens be minted?
• **Freeze Authority** — can transfers be frozen?
• **Top Holders** — concentration risk

Higher risk scores indicate more potential red flags.`,
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/filtering",
    },

    meta: {
      id: "filtering.meta",
      title: "Meta Filters",
      content: `Additional filtering criteria:

• **Token Age** — minimum time since token creation
• **Pool Age** — minimum time since pool creation
• **Has Website** — require social/website links
• **Has Socials** — require Twitter/Telegram

These help filter out very new or suspicious tokens.`,
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // TRADER PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  trader: {
    overview: {
      id: "trader.overview",
      title: "Auto Trader",
      content: `Automated trading engine that monitors tokens and executes trades.

Components:
• **Entry Monitor** — watches for buy opportunities
• **Exit Monitor** — manages sells and take-profits
• **DCA Monitor** — handles position averaging
• **Risk Controls** — loss limits and safety gates

Start/stop trading from the control panel.`,
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/trader",
    },

    entryMonitor: {
      id: "trader.entry",
      title: "Entry Monitor",
      content: `Watches filtered tokens for entry signals.

Entry evaluation checks:
• Token passes current filtering
• Not already in a position
• Not blacklisted
• Position limits not exceeded
• Strategy conditions met (if configured)

Configure entry size and limits in Config.`,
    },

    exitMonitor: {
      id: "trader.exit",
      title: "Exit Monitor",
      content: `Monitors open positions for exit signals.

Exit triggers:
• **Take Profit** — price target reached
• **Stop Loss** — maximum loss exceeded
• **Trailing Stop** — price retraced from peak
• **Strategy Exit** — custom conditions met
• **Time-based** — maximum hold duration

Configure thresholds in Config.`,
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // SERVICES PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  services: {
    overview: {
      id: "services.overview",
      title: "System Services",
      content: `Background services powering ScreenerBot.

Service states:
• 🟢 **Running** — operating normally
• 🟡 **Starting** — initializing
• 🔴 **Stopped** — not running
• ⚠️ **Error** — failed, may auto-restart

Services have dependencies and start in order.`,
    },

    health: {
      id: "services.health",
      title: "Service Health",
      content: `Health indicators show service status:

• **Uptime** — time since last start
• **Tasks** — active background operations
• **Errors** — recent error count
• **Metrics** — performance data (if available)

Critical services affect trading capability.`,
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // WALLET PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  wallet: {
    overview: {
      id: "wallet.overview",
      title: "Wallet Overview",
      content: `Your connected Solana wallet status.

Displays:
• **SOL Balance** — native SOL for gas and trading
• **Token Holdings** — SPL tokens with values
• **24h Change** — portfolio value change
• **History** — balance snapshots over time

Balances refresh every minute.`,
    },

    tokens: {
      id: "wallet.tokens",
      title: "Token Balances",
      content: `SPL tokens held in your wallet.

Shows:
• Token symbol and name
• Amount held
• Current value in SOL/USD
• Price from pool or market data

Empty token accounts can be cleaned up in Settings.`,
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // CONFIG PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  config: {
    overview: {
      id: "config.overview",
      title: "Configuration",
      content: `System-wide settings for ScreenerBot.

Categories:
• **Trader** — entry/exit rules, position sizing
• **Filtering** — token filter thresholds
• **Swaps** — routing and slippage settings
• **RPC** — node configuration
• **Services** — background service settings

Changes take effect immediately (hot reload).`,
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/system/config",
    },
  },
};

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
