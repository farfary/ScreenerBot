# Contextual hints: one title and one body per hint, keyed by the hint id.
# Bodies use a small markdown subset (**bold** and bullet lines) that the hint popover renders.

# Source: scripts/core/hints.js

## Categories

hints-category-positions = Positions
hints-category-filtering = Filtering
hints-category-trader = Auto Trader
hints-category-services = Services
hints-category-wallet = Wallet
hints-category-wallets = Wallets
hints-category-config = Config
hints-category-config-telegram = { -telegram }
hints-category-token-details = Token Details
hints-category-ui = Interface

## positions

hints-positions-overview-title = Positions Overview
hints-positions-overview-content =
    Your current token holdings and trading positions.

    Key metrics:
    • **Entry Price** — average price paid (including DCA)
    • **Current Price** — live price from pool reserves
    • **P&L** — unrealized profit/loss in SOL and %
    • **Size** — total token amount held

    Click any position for detailed management options.
hints-positions-dca-title = DCA (Dollar Cost Average)
hints-positions-dca-content =
    DCA allows adding to existing positions at different prices.

    When DCA is triggered:
    • Additional tokens are purchased
    • Entry price is recalculated as weighted average
    • Position size increases
    • Entry count increments

    Configure DCA rules in **Auto Trader** settings.
hints-positions-partial-exit-title = Partial Exit
hints-positions-partial-exit-content =
    Sell a portion of your position while keeping the rest.

    Benefits:
    • Lock in some profits while staying exposed
    • Reduce position size without fully closing
    • Implement take-profit ladders

    Each partial exit is recorded separately for accurate P&L tracking.
hints-positions-management-title = Position Management
hints-positions-management-content =
    Management defines which automation may act on a position.

    • Auto Trader: safety exits, policy exits, and auto-DCA
    • User Only: no automatic actions
    • Copy Task: safety exits and copy-sells
    • Hybrid: safety exits, policy exits, and copy-sells

    You sell or add to it yourself. Manual buys default to manual management so the bot can't sell a token you bought on purpose. Turn it off to hand the position back to the auto-trader.

## filtering

hints-filtering-overview-title = Token Filtering
hints-filtering-overview-content =
    Filtering determines which tokens are eligible for trading.

    Tokens must pass **all enabled criteria** to appear in the passed list:
    • { -dexscreener } metrics (liquidity, volume, etc.)
    • { -geckoterminal } metrics (market cap, FDV)
    • { -rugcheck } security analysis
    • Meta filters (token age, etc.)

    Disabled criteria are skipped entirely.
hints-filtering-dexscreener-title = { -dexscreener } Filters
hints-filtering-dexscreener-content =
    Filters based on { -dexscreener } market data.

    • **Liquidity** — minimum USD liquidity in pools
    • **Volume 24h** — minimum trading volume
    • **Transactions** — activity thresholds (buys/sells)
    • **Price Change** — volatility filters

    { -dexscreener } data updates every few minutes.
hints-filtering-geckoterminal-title = { -geckoterminal } Filters
hints-filtering-geckoterminal-content =
    Filters based on { -geckoterminal } market data.

    • **Market Cap** — minimum market capitalization
    • **FDV** — Fully Diluted Valuation limits
    • **Reserve Ratio** — pool health indicators

    { -geckoterminal } often has data for newer tokens.
hints-filtering-rugcheck-title = Security Filters
hints-filtering-rugcheck-content =
    Security analysis from { -rugcheck }.xyz.

    • **Risk Score** — overall risk rating (0-100)
    • **Mint Authority** — can new tokens be minted?
    • **Freeze Authority** — can transfers be frozen?
    • **Top Holders** — concentration risk

    Higher risk scores indicate more potential red flags.
hints-filtering-meta-title = Meta Filters
hints-filtering-meta-content =
    Additional filtering criteria.

    • **Token Age** — minimum time since token creation
    • **Pool Age** — minimum time since pool creation
    • **Has Website** — require social/website links
    • **Has Socials** — require { -twitter }/{ -telegram }

    These help filter out very new or suspicious tokens.

## trader

hints-trader-overview-title = Auto Trader
hints-trader-overview-content =
    Automated trading engine that monitors tokens and executes trades.

    Components:
    • **Entry Monitor** — watches for buy opportunities
    • **Exit Monitor** — manages sells and take-profits
    • **DCA Monitor** — handles position averaging
    • **Risk Controls** — loss limits and safety gates

    Start/stop trading from the control panel.
hints-trader-entry-title = Entry Monitor
hints-trader-entry-content =
    Watches filtered tokens for entry signals.

    Entry evaluation checks:
    • Token passes current filtering
    • Not already in a position
    • Not blacklisted
    • Position limits not exceeded
    • Strategy conditions met (if configured)

    Configure entry size and limits in Config.
hints-trader-exit-title = Exit Monitor
hints-trader-exit-content =
    Monitors open positions for exit signals.

    Exit triggers:
    • **Take Profit** — price target reached
    • **Stop Loss** — maximum loss exceeded
    • **Trailing Stop** — price retraced from peak
    • **Strategy Exit** — custom conditions met
    • **Time-based** — maximum hold duration

    Configure thresholds in Config.

## services

hints-services-overview-title = System Services
hints-services-overview-content =
    Background services powering { -brand }.

    Service states:
    • **Running** (green) — operating normally
    • **Starting** (yellow) — initializing
    • **Stopped** (red) — not running
    • **Error** (warning) — failed, may auto-restart

    Services have dependencies and start in order.
hints-services-health-title = Service Health
hints-services-health-content =
    Health indicators show service status.

    • **Uptime** — time since last start
    • **Tasks** — active background operations
    • **Errors** — recent error count
    • **Metrics** — performance data (if available)

    Critical services affect trading capability.

## wallet

hints-wallet-overview-title = Wallet Overview
hints-wallet-overview-content =
    Your connected Solana wallet status.

    Displays:
    • **SOL Balance** — native SOL for gas and trading
    • **Token Holdings** — SPL tokens with values
    • **24h Change** — portfolio value change
    • **History** — balance snapshots over time

    Balances refresh every minute.
hints-wallet-tokens-title = Token Balances
hints-wallet-tokens-content =
    SPL tokens held in your wallet.

    Shows:
    • Token symbol and name
    • Amount held
    • Current value in SOL/USD
    • Price from pool or market data

    Empty token accounts can be cleaned up in Settings.

## wallets

hints-wallets-main-title = Main Wallet
hints-wallets-main-content =
    The primary wallet used for all trading operations.

    • **Auto-Trading** — entry/exit trades execute from this wallet
    • **Balance Display** — shown in header and dashboard
    • **Token Holdings** — SPL tokens held by this wallet

## config

hints-config-overview-title = Configuration
hints-config-overview-content =
    System-wide settings for { -brand }.

    Categories:
    • **Trader** — entry/exit rules, position sizing
    • **Filtering** — token filter thresholds
    • **Swaps** — routing and slippage settings
    • **RPC** — node configuration
    • **Services** — background service settings

    Changes take effect immediately (hot reload).
hints-config-telegram-title = { -telegram } Notifications
hints-config-telegram-content =
    { "*" }*Receive instant trading alerts via { -telegram }**

    Get notified about trades, positions, and important events directly in { -telegram }.

    { "*" }*Setup Steps:**

    1. **Create a bot:**
       • Open { -telegram } and message @BotFather
       • Send /newbot and follow the prompts
       • Copy the bot token (looks like: 123456:ABC-DEF...)

    2. **Get your Chat ID:**
       • Message @userinfobot or @getidsbot
       • Copy the numeric ID it returns

    3. **Configure in { -brand }:**
       • Enable notifications toggle
       • Paste bot token and chat ID
       • Click "Test Connection" to verify

    { "*" }*What you'll receive:**
    • Trade execution confirmations
    • Position updates (entry/exit)
    • Error notifications

    { "*" }*Privacy:**
    Messages are sent directly from { -brand } to your { -telegram } bot — no third-party servers involved.
hints-config-telegram-password-title = Bot Authentication Password
hints-config-telegram-password-content =
    { "*" }*Secure your { -telegram } bot with password authentication**

    When you interact with your { -brand } { -telegram } bot, you'll need to authenticate with this password before executing sensitive commands.

    { "*" }*Why set a password?**
    • Prevents unauthorized users from controlling your bot
    • Required for executing trading commands via { -telegram }
    • Must be at least 8 characters long

    { "*" }*How it works:**
    1. Set a password here in the dashboard
    2. When you message your bot with a trading command, it will ask for authentication
    3. Enter your password to verify your identity
    4. Optionally enable 2FA for additional security

    { "*" }*Note:** The password is stored as a secure SHA256 hash — we never store the plain text.
hints-config-telegram-totp-title = Two-Factor Authentication (2FA)
hints-config-telegram-totp-content =
    { "*" }*Add an extra layer of security with TOTP 2FA**

    Two-factor authentication uses time-based one-time passwords (TOTP) from apps like Google Authenticator, Authy, or 1Password.

    { "*" }*Why enable 2FA?**
    • Even if someone knows your password, they can't access your bot without the code
    • 6-digit codes change every 30 seconds
    • Works offline once set up

    { "*" }*Setup process:**
    1. Click "Enable 2FA" and enter your password
    2. Scan the QR code with your authenticator app
    3. Enter the 6-digit code to verify setup

    { "*" }*Compatible apps:**
    • Google Authenticator
    • Authy
    • 1Password
    • Microsoft Authenticator
    • Any TOTP-compatible app

    { "*" }*Important:** Save your secret key in a safe place. If you lose access to your authenticator app, you'll need to disable 2FA from this dashboard.

## token_details

hints-token-details-chart-title = Price Chart (OHLCV)
hints-token-details-chart-content =
    { "*" }*Important:** This chart displays **cached OHLCV data** for strategy evaluation, *not* the live execution price.

    { "*" }*Why Cached Data?**
    • **Purpose:** Used by automated strategies and indicators (e.g., RSI, MA).
    • **Freshness:** Updates depend on token priority (Open positions = Faster updates).
    • **Source:** Aggregated from { -dexscreener }/{ -geckoterminal }, not direct on-chain RPC.

    { "*" }*DEX Price Reality:**
    In DeFi, tokens trade across **multiple pools** (Raydium, Orca, Meteora). Each pool has a unique price based on liquidity depth and recent trades.
    • **Chart Price:** An average/aggregate across markets.
    • **Swap Price:** The specific rate you get from the best route at the exact moment of trade.

    { "*" }Expect small differences between this chart and your final execution price.*

    { "*" }*Status:** "Waiting for data" means background workers are fetching fresh candles.
hints-token-details-token-info-title = Token Information
hints-token-details-token-info-content =
    Basic token metadata from on-chain and market sources.

        • **Mint** — unique token address on Solana (click to copy)
        • **Decimals** — token precision (usually 6-9)
        • **Age** — time since the primary pool/token was created
        • **DEX** — primary trading venue for this token
        • **Holders** — unique wallets holding the token
        • **Top 10 Hold** — % held by the top 10 wallets

        Higher holder count and lower concentration generally indicate healthier distribution.
hints-token-details-liquidity-title = Liquidity & Market Data
hints-token-details-liquidity-content =
    Market metrics from the highest-liquidity SOL pool.

        • **FDV** — price × total supply (aggregator price)
        • **Liquidity** — USD value of pool reserves
        • **Pool SOL** / **Pool Token** — live reserves that set pool price

        { "*" }*Why it matters:**
        • Deeper liquidity = lower slippage
        • Shallow pools can move on small trades
        • Pool reserves directly set swap execution price

        Data is refreshed periodically from { -dexscreener }/{ -geckoterminal } plus on-chain pool reads.
hints-token-details-market-pulse-title = Market Pulse
hints-token-details-market-pulse-content =
    Price movement and USD trading volume share the same **5M / 1H / 6H / 24H** timeline so momentum and participation can be compared directly.

    { "*" }*Interpretation:**
    • **Price** — aggregator-derived percentage change, not the live pool execution price.
    • **High Volume** — stronger interest, more efficient price discovery, and easier exits.
    • **Low Volume** — greater slippage, wider spreads, and harder large exits.
    • **High Volume + Low Liquidity** — elevated volatility and execution risk.

    Market data is aggregated across major DEXs via { -dexscreener }/{ -geckoterminal }, so price change can differ from the current on-chain pool price.
hints-token-details-activity-title = Transaction Activity (Counts)
hints-token-details-activity-content =
    Analyzes the **number of trades** (buys vs. sells) across multiple timeframes. This reveals trader intent regardless of trade size.

    { "*" }*Metric Breakdown:**
    • **Timeframes:** 5M, 1H, 6H, 24H windows.
    • **Bars:** Visual ratio of Buy count (Green) vs. Sell count (Red).
    • **Rate:** Trades per minute (e.g., "12.5/m"). Higher rates = viral activity.
    • **Counts:** Exact number of buys/sells and their percentage share.

    { "*" }*Summary metrics:**
    • **24H Buy %:** >50% is bullish (more buyers), { "<" }50% is bearish (more sellers).
    • **Net Flow:** Total buys minus sells. Positive = Accumulation.
    • **5M Spike:** How much faster trading is *right now* vs. the 1H average.
      • **>1.0x:** Accelerating interest.
      • **>3.0x:** Viral breakout or panic event.
      • **{ "<" }1.0x:** Cooling down.

    { "*" }*Strategy Tip:** High "Buy %" with high "Spike Factor" often signals a strong breakout entry.
hints-token-details-security-title = Security Analysis
hints-token-details-security-content =
    Risk assessment from { -rugcheck }.xyz and on-chain analysis.

    { "*" }*Safety Score (0-100):**
    Higher scores indicate safer tokens. Factors include:
    • Authority permissions (mint/freeze)
    • Holder concentration
    • LP lock status
    • Known risk patterns

    { "*" }*Key Risk Indicators:**
    • **Mint Authority** — can create new tokens (inflation risk)
    • **Freeze Authority** — can freeze token accounts
    • **Top Holder %** — concentration risk
    • **LP Providers** — liquidity provider count

    Always verify security before trading significant amounts.
hints-token-details-pools-title = Liquidity Pools
hints-token-details-pools-content =
    All discovered liquidity pools for this token.

    { "*" }*Why multiple pools matter:**
    • Each pool has different liquidity and pricing
    • Swap routers find the best route across pools
    • Price can vary 1-5% between pools

    { "*" }*Pool Information:**
    • **DEX** — which exchange hosts the pool
    • **Liquidity** — USD value of pool reserves
    • **Volume** — recent trading activity
    • **Price** — current pool price

    The Pool Service calculates prices from the highest-liquidity SOL pair.

## ui

hints-ui-featured-title = Featured
hints-ui-featured-content =
    Boosted tokens first, then trending projects from { -jupiter } and { -dexscreener }.

    { "*" }*What you'll see:**
    • Boosted tokens — their teams paid to promote them — pinned to the front, marked in gold
    • Trending tokens from the discovery boards after them
    • Click any token to open its full details

    { "*" }*Boosting a token:**
    A boost buys visibility, never a recommendation. Boosted rows are marked in gold everywhere
    they appear, including your token table, so you always know which is which. Boost a token at
    { "*" }*screenerbot.io/boost**.

    { "*" }*Disabling the row:**
    Hide it under **Settings → Interface → Show Featured Row**. The header action still opens the
    full Featured view.

## Hint popover chrome (ui/hint_popover.js)

# The help button beside a control. $title is the hint's title.
hints-trigger =
    .aria-label = Help: { $title }
hints-popover-close =
    .aria-label = Close
hints-popover-learn-more = Learn more
hints-popover-dismiss = Don't show again
