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
• **Running** (green) — operating normally
• **Starting** (yellow) — initializing
• **Stopped** (red) — not running
• **Error** (warning) — failed, may auto-restart

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

**What are ATAs?**
Associated Token Accounts (ATAs) are Solana accounts that hold your tokens. Each token you interact with creates an ATA that requires ~0.002 SOL in rent.

**Why clean up empty ATAs?**
• Reclaim rent (~0.002 SOL per ATA)
• Active traders can accumulate hundreds of empty ATAs
• 100 empty ATAs = ~0.2 SOL reclaimable

**How it works:**
• Scans your wallet for ATAs with zero balance
• Shows total reclaimable SOL amount
• Closes empty accounts to recover rent

**Auto Cleanup:**
When enabled, automatically scans and closes empty ATAs every 5 minutes in the background.

**Important:**
• Only closes accounts with exactly 0 balance
• Failed closures are cached to avoid retry spam
• Large wallets may require multiple cleanup passes`,
    },

    burnTokens: {
      id: "tools.burn_tokens",
      title: "Burn Tokens Tool",
      content: `**Permanently Destroy Tokens**

Burning tokens permanently removes them from your wallet and from circulation.

**What happens when you burn:**
• Tokens are sent to a burn address (unrecoverable)
• Token balance becomes zero
• ATA can then be closed via Wallet Cleanup to reclaim ~0.002 SOL rent

**Token Categories:**
• **Open Positions** - Cannot burn (active trades)
• **Closed Positions** - Leftovers from past trades
• **Has Value** - Tokens with liquidity (consider selling instead)
• **Zero Liquidity** - Dust/worthless tokens (safe to burn)

**Warning:** This action is **irreversible**. Burned tokens cannot be recovered under any circumstances.

**After burning:** Run Wallet Cleanup to close empty ATAs and reclaim SOL rent.`,
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

    multiBuy: {
      id: "tools.multi_buy",
      title: "Multi-Buy Tool",
      content: `**Coordinate Buys Across Multiple Wallets**

Execute buy orders across multiple sub-wallets with randomized amounts to simulate organic buying activity.

**How it works:**
1. Creates or uses existing sub-wallets
2. Distributes SOL from main wallet to sub-wallets
3. Executes buy orders with randomized amounts and delays
4. Each wallet buys independently with unique signatures

**Wallet Settings:**
• **Wallet Count** — number of sub-wallets to use (2-10)
• **SOL Buffer** — SOL reserved per wallet for fees (~0.015)

**Amount Settings:**
• **Min/Max SOL** — range for buy amounts per wallet
• **Total Limit** — optional cap on total SOL to spend

**Execution Settings:**
• **Delay** — random delay between transactions
• **Concurrency** — parallel execution (1 = sequential)
• **Slippage** — maximum acceptable slippage
• **Router** — swap routing (Auto, Jupiter, Raydium)

**Important:**
• Requires sufficient SOL in main wallet
• Failed buys are logged but don't stop the session
• Sub-wallets can be reused across sessions`,
      learnMoreUrl: "https://screenerbot.io/docs/tools/multi-buy",
    },

    multiSell: {
      id: "tools.multi_sell",
      title: "Multi-Sell Tool",
      content: `**Coordinate Sells Across Multiple Wallets**

Sell tokens from all sub-wallets holding a specific token with automatic SOL consolidation.

**How it works:**
1. Scans sub-wallets for token balances
2. Optionally tops up wallets with low SOL for fees
3. Executes sell orders with configurable percentage
4. Consolidates proceeds back to main wallet

**Sell Settings:**
• **Sell %** — percentage of tokens to sell (default 100%)
• **Min SOL for Fee** — minimum SOL needed for transaction
• **Auto Topup** — transfer SOL from main if needed

**Post-Sell Actions:**
• **Consolidate SOL** — transfer all SOL back to main wallet
• **Close ATAs** — close token accounts to reclaim rent (~0.002 SOL each)

**Execution Settings:**
• **Delay** — random delay between transactions
• **Concurrency** — parallel execution
• **Slippage** — maximum acceptable slippage
• **Router** — swap routing preference

**Tips:**
• Preview shows all wallets holding the token
• Deselect wallets you don't want to sell from
• Consolidation happens after all sells complete`,
      learnMoreUrl: "https://screenerbot.io/docs/tools/multi-sell",
    },

    tradeWatcher: {
      id: "tools.trade_watcher",
      title: "Trade Watcher Tool",
      content: `**Monitor Trades & Trigger Automatic Actions**

Watch a token's trading activity and automatically react when trades occur.

**Watch Types:**
• **Buy on Sell** — automatically buy when someone sells (catch dips)
• **Sell on Buy** — automatically sell when someone buys (follow the market)
• **Notify Only** — get alerts without taking action

**How it works:**
1. Enter a token mint address
2. Click "Search Pools" to find available liquidity pools
3. Select a pool to monitor (required for buy/sell actions)
4. Set trigger amount (minimum trade size to react to)
5. Set action amount (how much SOL to buy/sell)
6. Start the watch

**Requirements:**
• Valid token mint address
• Pool selection (for buy/sell actions)
• Sufficient SOL balance for action amounts

**Telegram Integration:**
Configure Telegram in Config → Telegram to receive instant notifications when watches trigger.`,
      learnMoreUrl: "https://screenerbot.io/docs/tools/trade-watcher",
    },

    walletConsolidation: {
      id: "tools.wallet_consolidation",
      title: "Wallet Consolidation Tool",
      content: `**Manage and Consolidate Sub-Wallet Funds**

View all sub-wallets and consolidate SOL, tokens, and reclaim ATA rent back to your main wallet.

**Summary Shows:**
• **Sub-wallets** — total count of created sub-wallets
• **Total SOL** — combined SOL balance across all sub-wallets
• **Token Types** — number of different tokens held
• **Reclaimable Rent** — SOL locked in empty ATAs

**Actions:**
• **Transfer SOL** — move all SOL from selected wallets to main
• **Transfer Tokens** — move all tokens to main wallet
• **Cleanup ATAs** — close empty token accounts for rent refund

**Table Info:**
• Checkbox to select wallets for batch operations
• Name, address, SOL balance, token count, empty ATAs
• Empty wallets are dimmed for easy identification

**Tips:**
• Use after Multi-Sell to collect remaining SOL
• Regularly cleanup ATAs to reclaim rent
• Empty wallets can be reused for future operations`,
      learnMoreUrl: "https://screenerbot.io/docs/tools/consolidation",
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
  // CONFIG PAGE - TELEGRAM
  // ═══════════════════════════════════════════════════════════════════════════
  configTelegram: {
    overview: {
      id: "config.telegram",
      title: "Telegram Notifications",
      content: `**Receive instant trading alerts via Telegram**

Get notified about trades, positions, and important events directly in Telegram.

**Setup Steps:**

1. **Create a bot:**
   • Open Telegram and message @BotFather
   • Send /newbot and follow the prompts
   • Copy the bot token (looks like: 123456:ABC-DEF...)

2. **Get your Chat ID:**
   • Message @userinfobot or @getidsbot
   • Copy the numeric ID it returns

3. **Configure in ScreenerBot:**
   • Enable notifications toggle
   • Paste bot token and chat ID
   • Click "Test Connection" to verify

**What you'll receive:**
• Trade execution confirmations
• Position updates (entry/exit)
• Trade Watcher alerts
• Error notifications

**Privacy:**
Messages are sent directly from ScreenerBot to your Telegram bot — no third-party servers involved.`,
      learnMoreUrl: "https://screenerbot.io/docs/config/telegram",
    },
    password: {
      id: "config.telegram.password",
      title: "Bot Authentication Password",
      content: `**Secure your Telegram bot with password authentication**

When you interact with your ScreenerBot Telegram bot, you'll need to authenticate with this password before executing sensitive commands.

**Why set a password?**
• Prevents unauthorized users from controlling your bot
• Required for executing trading commands via Telegram
• Must be at least 8 characters long

**How it works:**
1. Set a password here in the dashboard
2. When you message your bot with a trading command, it will ask for authentication
3. Enter your password to verify your identity
4. Optionally enable 2FA for additional security

**Note:** The password is stored as a secure SHA256 hash — we never store the plain text.`,
      learnMoreUrl: "https://screenerbot.io/docs/config/telegram",
    },
    totp: {
      id: "config.telegram.totp",
      title: "Two-Factor Authentication (2FA)",
      content: `**Add an extra layer of security with TOTP 2FA**

Two-factor authentication uses time-based one-time passwords (TOTP) from apps like Google Authenticator, Authy, or 1Password.

**Why enable 2FA?**
• Even if someone knows your password, they can't access your bot without the code
• 6-digit codes change every 30 seconds
• Works offline once set up

**Setup process:**
1. Click "Enable 2FA" and enter your password
2. Scan the QR code with your authenticator app
3. Enter the 6-digit code to verify setup

**Compatible apps:**
• Google Authenticator
• Authy
• 1Password
• Microsoft Authenticator
• Any TOTP-compatible app

**Important:** Save your secret key in a safe place. If you lose access to your authenticator app, you'll need to disable 2FA from this dashboard.`,
      learnMoreUrl: "https://screenerbot.io/docs/config/telegram",
    },
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // TOKEN DETAILS DIALOG
  // ═══════════════════════════════════════════════════════════════════════════
  tokenDetails: {
    chart: {
      id: "token_details.chart",
      title: "Price Chart (OHLCV)",
      content: `**Important:** This chart displays **cached OHLCV data** for strategy evaluation, *not* the live execution price.

**Why Cached Data?**
• **Purpose:** Used by automated strategies and indicators (e.g., RSI, MA).
• **Freshness:** Updates depend on token priority (Open positions = Faster updates).
• **Source:** Aggregated from DexScreener/GeckoTerminal, not direct on-chain RPC.

**DEX Price Reality:**
In DeFi, tokens trade across **multiple pools** (Raydium, Orca, Meteora). Each pool has a unique price based on liquidity depth and recent trades.
• **Chart Price:** An average/aggregate across markets.
• **Swap Price:** The specific rate you get from the best route at the exact moment of trade.

*Expect small differences between this chart and your final execution price.*

**Status:** "Waiting for data" means background workers are fetching fresh candles.`,
      learnMoreUrl: "https://screenerbot.io/docs/concepts/pricing",
    },

    tokenInfo: {
      id: "token_details.token_info",
      title: "Token Information",
      content: `Basic token metadata from on-chain and market sources.

    • **Mint** — unique token address on Solana (click to copy)
    • **Decimals** — token precision (usually 6-9)
    • **Age** — time since the primary pool/token was created
    • **DEX** — primary trading venue for this token
    • **Holders** — unique wallets holding the token
    • **Top 10 Hold** — % held by the top 10 wallets

    Higher holder count and lower concentration generally indicate healthier distribution.`,
    },

    liquidity: {
      id: "token_details.liquidity",
      title: "Liquidity & Market Data",
      content: `Market metrics from the highest-liquidity SOL pool.

    • **FDV** — price × total supply (aggregator price)
    • **Liquidity** — USD value of pool reserves
    • **Pool SOL** / **Pool Token** — live reserves that set pool price

    **Why it matters:**
    • Deeper liquidity = lower slippage
    • Shallow pools can move on small trades
    • Pool reserves directly set swap execution price

    Data is refreshed periodically from DexScreener/GeckoTerminal plus on-chain pool reads.`,
    },

    priceChanges: {
      id: "token_details.price_changes",
      title: "Price Changes",
      content: `Price movement over timeframes (aggregator-derived, not the live pool price).

    • **5M / 1H / 6H / 24H** — % change from market sources

    May differ from on-chain pool prices because of averaging across pools, refresh cadence, and calculation methods. Use alongside the live pool price for execution decisions.`,
    },

    volume: {
      id: "token_details.volume",
      title: "Trading Volume",
      content: `USD trading volume from market aggregators over each timeframe.

**Interpretation:**
• **High Volume:** Strong interest, efficient price discovery, easier exits.
• **Low Volume:** Risk of slippage, wider spreads, and difficulty exiting large positions.
• **Volume/Liquidity Ratio:** High volume with low liquidity = enormous volatility.

**Data Source:** Aggregated from major DEXs (Raydium, Orca, etc.) via DexScreener/GeckoTerminal.`,
    },

    activity: {
      id: "token_details.activity",
      title: "Transaction Activity (Counts)",
      content: `Analyzes the **number of trades** (buys vs. sells) across multiple timeframes. This reveals trader intent regardless of trade size.

**Metric Breakdown:**
• **Timeframes:** 5M, 1H, 6H, 24H windows.
• **Bars:** Visual ratio of Buy count (Green) vs. Sell count (Red).
• **Rate:** Trades per minute (e.g., "12.5/m"). Higher rates = viral activity.
• **Counts:** Exact number of buys/sells and their percentage share.

**Insights Panel:**
• **24H Buy %:** >50% is bullish (more buyers), <50% is bearish (more sellers).
• **Net Flow:** Total buys minus sells. Positive = Accumulation.
• **5M Spike:** How much faster trading is *right now* vs. the 1H average.
  • **>1.0x:** Accelerating interest.
  • **>3.0x:** Viral breakout or panic event.
  • **<1.0x:** Cooling down.

**Strategy Tip:** High "Buy %" with high "Spike Factor" often signals a strong breakout entry.`,
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

  // ═══════════════════════════════════════════════════════════════════════════
  // UI COMPONENTS
  // ═══════════════════════════════════════════════════════════════════════════
  ui: {
    billboard: {
      id: "ui.billboard",
      title: "Billboard",
      content: `Billboard showcases **featured tokens** and trending projects from the community.

**What you'll see:**
• Promoted tokens with verified listings
• Featured projects highlighted with ⭐
• Token logos and names for quick recognition

**Submitting your token:**
Want your token featured? Visit **screenerbot.io/submit-token** to apply for a listing.

**Disabling Billboard:**
If you prefer a cleaner interface, you can hide this row in **Settings → Interface → Show Billboard**.`,
      learnMoreUrl: "https://screenerbot.io/docs/dashboard/billboard",
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
