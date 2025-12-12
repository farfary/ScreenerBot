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

    ohlcvData: {
      id: "tokens.ohlcv",
      title: "OHLCV Data Management",
      content: `View and manage OHLCV (candlestick) data stored for tokens.

Shows:
• **Candle Count** — total data points stored
• **Backfill Progress** — timeframe completion status
• **Data Span** — time coverage in hours
• **Pool Count** — tracked liquidity pools
• **Status** — active monitoring or inactive

Actions:
• **Delete** — remove all OHLCV data for a token
• **Cleanup** — bulk remove inactive token data

OHLCV data is preserved permanently and never auto-deleted.`,
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
  // WALLETS PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  wallets: {
    security: {
      id: "wallets.security",
      title: "Wallet Security",
      content: `**Bank-Grade Encryption**

All private keys are encrypted with AES-256-GCM using a machine-bound key:

• **AES-256-GCM** — military-grade encryption standard
• **Machine-Bound Key** — derived from your device's unique identifier
• **Local Storage Only** — keys never leave your device
• **No Cloud Backup** — keys cannot be recovered if lost

**What this means:**
• Your keys are safe even if the database file is stolen
• Only this specific machine can decrypt the keys
• Always backup your private keys externally`,
    },

    mainWallet: {
      id: "wallets.main",
      title: "Main Wallet",
      content: `The primary wallet used for all trading operations.

• **Auto-Trading** — entry/exit trades execute from this wallet
• **Balance Display** — shown in header and dashboard
• **Token Holdings** — SPL tokens held by this wallet

Change the main wallet by selecting "Set as Main" on any secondary wallet.`,
    },

    secondaryWallets: {
      id: "wallets.secondary",
      title: "Secondary Wallets",
      content: `Additional wallets for multi-wallet operations.

• **Multi-Wallet Trading** — coordinate buys/sells across wallets
• **Portfolio Separation** — organize by strategy or purpose
• **Independent Balances** — each wallet has its own SOL/tokens

Secondary wallets are not used by auto-trading unless explicitly configured.`,
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // TOOLS PAGE
  // ═══════════════════════════════════════════════════════════════════════════
  tools: {
    walletCleanup: {
      id: "tools.wallet_cleanup",
      title: "Wallet Cleanup Tool",
      content: `**Reclaim SOL from Empty Token Accounts**

Every token you interact with creates an Associated Token Account (ATA) that requires ~0.002 SOL in rent.

**How it works:**
• Scans your wallet for ATAs with zero balance
• Shows total reclaimable SOL amount
• Closes empty accounts to recover rent

**Important:**
• Only closes accounts with exactly 0 balance
• Failed closures are cached to avoid retry spam
• Large wallets may require multiple cleanup passes`,
    },

    burnTokens: {
      id: "tools.burn_tokens",
      title: "Burn Tokens Tool",
      content: `**Permanently Destroy Tokens**

Burning tokens removes them from circulation forever.

**Use cases:**
• Clean up worthless dust tokens
• Reduce token supply (if you're the creator)
• Remove scam/spam tokens

**Warning:** This action is irreversible. Burned tokens cannot be recovered.`,
    },

    walletGenerator: {
      id: "tools.wallet_generator",
      title: "Wallet Generator Tool",
      content: `**Generate New Solana Keypairs**

Create new wallets securely on your device.

**Features:**
• Generates cryptographically secure keypairs
• Optional vanity address prefix (e.g., "SOL...")
• Export as base58 or JSON array

**Security:**
• Keys are generated locally
• Never transmitted over the network
• Always backup keys securely`,
    },

    volumeAggregator: {
      id: "tools.volume_aggregator",
      title: "Volume Aggregator Tool",
      content: `**Generate Trading Volume**

Creates organic-looking trading activity for a token using multiple wallets.

**How it works:**
• Uses your secondary wallets to execute buy/sell pairs
• Distributes transactions across wallets for natural appearance
• Configurable amounts and delays between transactions

**Requirements:**
• At least 2 secondary wallets configured
• Each wallet needs SOL for gas fees (~0.01 SOL minimum)
• Token must have active liquidity pools

**Configuration:**
• **Total Volume** — target SOL volume to generate
• **Min/Max Amount** — range for individual transaction sizes
• **Delay** — time between transactions (min 1000ms)
• **Randomize** — vary amounts within range

**Risks:**
• Wallet balances are used for transactions
• Failed transactions may result in partial fills
• High-frequency trading may trigger rate limits`,
      learnMoreUrl: "https://screenerbot.io/docs/tools/volume-aggregator",
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

  // ═══════════════════════════════════════════════════════════════════════════
  // TOKEN DETAILS DIALOG
  // ═══════════════════════════════════════════════════════════════════════════
  tokenDetails: {
    chart: {
      id: "token_details.chart",
      title: "Price Chart (OHLCV)",
      content: `**Important:** This chart shows cached OHLCV data used for automated trading strategies, not live prices.

**Why cached data?**
• OHLCV data is collected and aggregated for strategy evaluation
• Update frequency depends on token priority (positions get faster updates)
• Used by entry/exit monitors for technical analysis decisions

**DEX Price Complexity:**
In decentralized trading, a token can have **multiple liquidity pools** across different DEXs (Raydium, Orca, Meteora, etc.). Each pool may show slightly different prices based on:
• Pool liquidity depth
• Recent trading activity
• Arbitrage lag between pools

There is **no single "true" price** until you execute a swap — the actual price depends on which pool/route is used and current slippage.

**Data Sources:**
• DexScreener and GeckoTerminal provide aggregated OHLCV
• Timeframe selection affects candle granularity
• "Waiting for data" means OHLCV is being fetched`,
      learnMoreUrl: "https://screenerbot.io/docs/concepts/pricing",
    },

    tokenInfo: {
      id: "token_details.token_info",
      title: "Token Information",
      content: `Basic token metadata from on-chain and market sources.

• **Mint** — unique token address on Solana (click to copy)
• **Decimals** — token precision (usually 6-9)
• **Age** — time since token/pool creation
• **DEX** — primary trading venue for this token
• **Holders** — unique wallet addresses holding the token
• **Top 10 Hold** — percentage held by top 10 wallets

Higher holder count and lower concentration generally indicate healthier distribution.`,
    },

    liquidity: {
      id: "token_details.liquidity",
      title: "Liquidity & Market Data",
      content: `Market metrics from the primary liquidity pool.

• **FDV** — Fully Diluted Valuation (price × total supply)
• **Liquidity** — USD value of pool reserves
• **Pool SOL** — SOL reserves in the pool
• **Pool Token** — token reserves in the pool

**Why liquidity matters:**
• Higher liquidity = less slippage on trades
• Low liquidity can cause significant price impact
• Pool reserves directly determine swap prices

Data from DexScreener/GeckoTerminal, refreshed periodically.`,
    },

    priceChanges: {
      id: "token_details.price_changes",
      title: "Price Changes",
      content: `Price movement over various timeframes.

• **5M** — last 5 minutes
• **1H** — last hour
• **6H** — last 6 hours
• **24H** — last 24 hours

**Note:** These percentages come from market aggregators and may differ slightly from on-chain pool prices due to:
• Data aggregation delays
• Multiple pool price averaging
• Different calculation methodologies`,
    },

    volume: {
      id: "token_details.volume",
      title: "Trading Volume",
      content: `USD trading volume across timeframes.

Higher volume indicates:
• More active trading interest
• Better price discovery
• Generally lower slippage

Very low volume tokens may have:
• Wide bid-ask spreads
• Difficult exits
• Higher manipulation risk`,
    },

    activity: {
      id: "token_details.activity",
      title: "Transaction Activity",
      content: `Buy/sell transaction counts and ratios.

• **Buy/Sell bars** — visual ratio of buys vs sells
• **B/S Ratio** — buys divided by sells (>1 = more buying)
• **Net Flow** — difference between buy and sell counts

**Interpreting activity:**
• High buy ratio may indicate accumulation
• High sell ratio may indicate distribution
• Transaction count doesn't reflect volume size`,
    },

    security: {
      id: "token_details.security",
      title: "Security Analysis",
      content: `Risk assessment from Rugcheck.xyz and on-chain analysis.

**Safety Score (0-100):**
Higher scores indicate safer tokens. Factors include:
• Authority permissions (mint/freeze)
• Holder concentration
• LP lock status
• Known risk patterns

**Key Risk Indicators:**
• **Mint Authority** — can create new tokens (inflation risk)
• **Freeze Authority** — can freeze token accounts
• **Top Holder %** — concentration risk
• **LP Providers** — liquidity provider count

Always verify security before trading significant amounts.`,
      learnMoreUrl: "https://screenerbot.io/docs/concepts/security",
    },

    pools: {
      id: "token_details.pools",
      title: "Liquidity Pools",
      content: `All discovered liquidity pools for this token.

**Why multiple pools matter:**
• Each pool has different liquidity and pricing
• Swap routers find the best route across pools
• Price can vary 1-5% between pools

**Pool Information:**
• **DEX** — which exchange hosts the pool
• **Liquidity** — USD value of pool reserves
• **Volume** — recent trading activity
• **Price** — current pool price

The Pool Service calculates prices from the highest-liquidity SOL pair.`,
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
