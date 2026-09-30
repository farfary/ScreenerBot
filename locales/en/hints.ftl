# Contextual hints: one title and one body per hint, keyed by the hint id.
# Bodies use a small markdown subset (**bold** and bullet lines) that the hint popover renders.

# Source: scripts/core/hints.js

## Categories

hints-category-tokens = Tokens
hints-category-positions = Positions
hints-category-filtering = Filtering
hints-category-trader = Auto Trader
hints-category-services = Services
hints-category-wallet = Wallet
hints-category-wallets = Wallets
hints-category-tools = Tools
hints-category-config = Config
hints-category-config-telegram = Telegram
hints-category-token-details = Token Details
hints-category-ui = Interface

## tokens

hints-tokens-pool-service-title = Pool Service Tokens
hints-tokens-pool-service-content =
    Tokens shown here have:

    • **Passed all filtering criteria** — liquidity, volume, age, and security checks
    • **Valid SOL liquidity pools** — supported by our DEX decoders (Raydium, Orca, Meteora, etc.)
    • **Successful price calculation** — prices computed directly from on-chain pool reserves

    This is the most reliable token list for trading as prices are derived from actual pool data, not external APIs.

    Click any token to view detailed information and manage blacklist status.
hints-tokens-no-market-title = No Market Data
hints-tokens-no-market-content =
    Tokens discovered on-chain but missing market data from DexScreener or GeckoTerminal.

    Common reasons:
    • **Very new tokens** — not yet indexed by aggregators
    • **Low trading volume** — below aggregator thresholds
    • **Unlisted pairs** — trading on DEXs not tracked by aggregators

    These tokens may still have valid pools and can be traded, but lack external market metrics.
hints-tokens-all-title = All Tokens
hints-tokens-all-content =
    Complete database of discovered tokens regardless of filtering status.

    Includes:
    • Tokens that passed filtering
    • Tokens that were rejected
    • Tokens without market data
    • Blacklisted tokens

    Use this view for research or to find tokens that may have been filtered out.
hints-tokens-passed-title = Passed Filtering
hints-tokens-passed-content =
    Tokens that passed all active filtering criteria.

    Filtering checks include:
    • **Liquidity** — minimum SOL liquidity threshold
    • **Volume** — 24h trading volume requirements
    • **Token age** — minimum time since creation
    • **Security** — Rugcheck risk score limits
    • **Market cap** — optional FDV/MC filters

    Configure filters in the **Filtering** page.
hints-tokens-rejected-title = Rejected Tokens
hints-tokens-rejected-content =
    Tokens that failed one or more filtering criteria.

    Each token shows the specific rejection reason:
    • Which filter failed
    • The actual value vs required threshold
    • When the check occurred

    Review rejected tokens to fine-tune your filter settings.
hints-tokens-blacklisted-title = Blacklisted Tokens
hints-tokens-blacklisted-content =
    Tokens permanently excluded from trading.

    Blacklist reasons include:
    • **Manual blacklist** — tokens you've explicitly blocked
    • **Security risks** — detected rug pull indicators
    • **Loss threshold** — exceeded configured loss limits
    • **Failed transactions** — repeated swap failures

    Blacklisted tokens are never shown in passed lists or considered for auto-trading.
hints-tokens-positions-title = Position Tokens
hints-tokens-positions-content =
    Tokens currently held in open positions.

    Shows real-time data for your active holdings:
    • Current price from pool reserves
    • Unrealized P&L
    • Position size and entry price
    • Time held

    Click any token for detailed position management.
hints-tokens-recent-title = Recently Discovered
hints-tokens-recent-content =
    Newly discovered tokens ordered by discovery time.

    Useful for:
    • Spotting new token launches
    • Monitoring fresh liquidity
    • Early entry opportunities

    Note: New tokens may lack complete market data initially.
hints-tokens-ohlcv-title = OHLCV Data Management
hints-tokens-ohlcv-content =
    View and manage OHLCV (candlestick) data stored for tokens.

    Shows:
    • **Candle Count** — total data points stored
    • **Backfill Progress** — timeframe completion status
    • **Data Span** — time coverage in hours
    • **Pool Count** — tracked liquidity pools
    • **Status** — active monitoring or inactive

    Actions:
    • **Delete** — remove all OHLCV data for a token
    • **Cleanup** — bulk remove inactive token data

    OHLCV data is preserved permanently and never auto-deleted.

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
    Management defines which automation may act on a position:

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
    • DexScreener metrics (liquidity, volume, etc.)
    • GeckoTerminal metrics (market cap, FDV)
    • Rugcheck security analysis
    • Meta filters (token age, etc.)

    Disabled criteria are skipped entirely.
hints-filtering-dexscreener-title = DexScreener Filters
hints-filtering-dexscreener-content =
    Filters based on DexScreener market data:

    • **Liquidity** — minimum USD liquidity in pools
    • **Volume 24h** — minimum trading volume
    • **Transactions** — activity thresholds (buys/sells)
    • **Price Change** — volatility filters

    DexScreener data updates every few minutes.
hints-filtering-geckoterminal-title = GeckoTerminal Filters
hints-filtering-geckoterminal-content =
    Filters based on GeckoTerminal market data:

    • **Market Cap** — minimum market capitalization
    • **FDV** — Fully Diluted Valuation limits
    • **Reserve Ratio** — pool health indicators

    GeckoTerminal often has data for newer tokens.
hints-filtering-rugcheck-title = Security Filters
hints-filtering-rugcheck-content =
    Security analysis from Rugcheck.xyz:

    • **Risk Score** — overall risk rating (0-100)
    • **Mint Authority** — can new tokens be minted?
    • **Freeze Authority** — can transfers be frozen?
    • **Top Holders** — concentration risk

    Higher risk scores indicate more potential red flags.
hints-filtering-meta-title = Meta Filters
hints-filtering-meta-content =
    Additional filtering criteria:

    • **Token Age** — minimum time since token creation
    • **Pool Age** — minimum time since pool creation
    • **Has Website** — require social/website links
    • **Has Socials** — require Twitter/Telegram

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
    Background services powering ScreenerBot.

    Service states:
    • **Running** (green) — operating normally
    • **Starting** (yellow) — initializing
    • **Stopped** (red) — not running
    • **Error** (warning) — failed, may auto-restart

    Services have dependencies and start in order.
hints-services-health-title = Service Health
hints-services-health-content =
    Health indicators show service status:

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

    Change the main wallet by selecting "Set as Main" on any secondary wallet.
hints-wallets-secondary-title = Secondary Wallets
hints-wallets-secondary-content =
    Additional wallets for multi-wallet operations.

    • **Multi-Wallet Trading** — coordinate buys/sells across wallets
    • **Portfolio Separation** — organize by strategy or purpose
    • **Independent Balances** — each wallet has its own SOL/tokens

    Secondary wallets are not used by auto-trading unless explicitly configured.

## tools

hints-tools-wallet-cleanup-title = Wallet Cleanup Tool
hints-tools-wallet-cleanup-content =
    { "*" }*Reclaim SOL from Empty Token Accounts**

    { "*" }*What are ATAs?**
    Associated Token Accounts (ATAs) are Solana accounts that hold your tokens. Each token you interact with creates an ATA that requires ~0.002 SOL in rent.

    { "*" }*Why clean up empty ATAs?**
    • Reclaim rent (~0.002 SOL per ATA)
    • Active traders can accumulate hundreds of empty ATAs
    • 100 empty ATAs = ~0.2 SOL reclaimable

    { "*" }*How it works:**
    • Scans your wallet for ATAs with zero balance
    • Shows total reclaimable SOL amount
    • Closes empty accounts to recover rent

    { "*" }*Auto Cleanup:**
    When enabled, automatically scans and closes empty ATAs every 5 minutes in the background.

    { "*" }*Important:**
    • Only closes accounts with exactly 0 balance
    • Failed closures are cached to avoid retry spam
    • Large wallets may require multiple cleanup passes
hints-tools-burn-tokens-title = Burn Tokens Tool
hints-tools-burn-tokens-content =
    { "*" }*Permanently Destroy Tokens**

    Burning tokens permanently removes them from your wallet and from circulation.

    { "*" }*What happens when you burn:**
    • Tokens are sent to a burn address (unrecoverable)
    • Token balance becomes zero
    • ATA can then be closed via Wallet Cleanup to reclaim ~0.002 SOL rent

    { "*" }*Token Categories:**
    • **Open Positions** - Cannot burn (active trades)
    • **Closed Positions** - Leftovers from past trades
    • **Has Value** - Tokens with liquidity (consider selling instead)
    • **Zero Liquidity** - Dust/worthless tokens (safe to burn)

    { "*" }*Warning:** This action is **irreversible**. Burned tokens cannot be recovered under any circumstances.

    { "*" }*After burning:** Run Wallet Cleanup to close empty ATAs and reclaim SOL rent.
hints-tools-wallet-generator-title = Wallet Generator Tool
hints-tools-wallet-generator-content =
    { "*" }*Generate New Solana Keypairs**

    Create new wallets securely on your device.

    { "*" }*Features:**
    • Generates cryptographically secure keypairs
    • Optional vanity address prefix (e.g., "SOL...")
    • Export as base58 or JSON array

    { "*" }*Security:**
    • Keys are generated locally
    • Never transmitted over the network
    • Always backup keys securely
hints-tools-multi-buy-title = Multi-Buy Tool
hints-tools-multi-buy-content =
    { "*" }*Coordinate Buys Across Multiple Wallets**

    Execute buy orders across multiple sub-wallets with randomized amounts to simulate organic buying activity.

    { "*" }*How it works:**
    1. Creates or uses existing sub-wallets
    2. Distributes SOL from main wallet to sub-wallets
    3. Executes buy orders with randomized amounts and delays
    4. Each wallet buys independently with unique signatures

    { "*" }*Wallet Settings:**
    • **Wallet Count** — number of sub-wallets to use (2-10)
    • **SOL Buffer** — SOL reserved per wallet for fees (~0.015)

    { "*" }*Amount Settings:**
    • **Min/Max SOL** — range for buy amounts per wallet
    • **Total Limit** — optional cap on total SOL to spend

    { "*" }*Execution Settings:**
    • **Delay** — random delay between transactions
    • **Concurrency** — parallel execution (1 = sequential)
    • **Slippage** — maximum acceptable slippage
    • **Router** — swap routing (Auto, Jupiter, Raydium)

    { "*" }*Important:**
    • Requires sufficient SOL in main wallet
    • Failed buys are logged but don't stop the session
    • Sub-wallets can be reused across sessions
hints-tools-multi-sell-title = Multi-Sell Tool
hints-tools-multi-sell-content =
    { "*" }*Coordinate Sells Across Multiple Wallets**

    Sell tokens from all sub-wallets holding a specific token with automatic SOL consolidation.

    { "*" }*How it works:**
    1. Scans sub-wallets for token balances
    2. Optionally tops up wallets with low SOL for fees
    3. Executes sell orders with configurable percentage
    4. Consolidates proceeds back to main wallet

    { "*" }*Sell Settings:**
    • **Sell %** — percentage of tokens to sell (default 100%)
    • **Min SOL for Fee** — minimum SOL needed for transaction
    • **Auto Topup** — transfer SOL from main if needed

    { "*" }*Post-Sell Actions:**
    • **Consolidate SOL** — transfer all SOL back to main wallet
    • **Close ATAs** — close token accounts to reclaim rent (~0.002 SOL each)

    { "*" }*Execution Settings:**
    • **Delay** — random delay between transactions
    • **Concurrency** — parallel execution
    • **Slippage** — maximum acceptable slippage
    • **Router** — swap routing preference

    { "*" }*Tips:**
    • Preview shows all wallets holding the token
    • Deselect wallets you don't want to sell from
    • Consolidation happens after all sells complete
hints-tools-trade-watcher-title = Trade Watcher Tool
hints-tools-trade-watcher-content =
    { "*" }*Monitor Trades & Trigger Automatic Actions**

    Watch a token's trading activity and automatically react when trades occur.

    { "*" }*Watch Types:**
    • **Buy on Sell** — automatically buy when someone sells (catch dips)
    • **Sell on Buy** — automatically sell when someone buys (follow the market)
    • **Notify Only** — get alerts without taking action

    { "*" }*How it works:**
    1. Enter a token mint address
    2. Click "Search Pools" to find available liquidity pools
    3. Select a pool to monitor (required for buy/sell actions)
    4. Set trigger amount (minimum trade size to react to)
    5. Set action amount (how much SOL to buy/sell)
    6. Start the watch

    { "*" }*Requirements:**
    • Valid token mint address
    • Pool selection (for buy/sell actions)
    • Sufficient SOL balance for action amounts

    { "*" }*Telegram Integration:**
    Configure Telegram in Config → Telegram to receive instant notifications when watches trigger.
hints-tools-wallet-consolidation-title = Wallet Consolidation Tool
hints-tools-wallet-consolidation-content =
    { "*" }*Manage and Consolidate Sub-Wallet Funds**

    View all sub-wallets and consolidate SOL, tokens, and reclaim ATA rent back to your main wallet.

    { "*" }*Summary Shows:**
    • **Sub-wallets** — total count of created sub-wallets
    • **Total SOL** — combined SOL balance across all sub-wallets
    • **Token Types** — number of different tokens held
    • **Reclaimable Rent** — SOL locked in empty ATAs

    { "*" }*Actions:**
    • **Transfer SOL** — move all SOL from selected wallets to main
    • **Transfer Tokens** — move all tokens to main wallet
    • **Cleanup ATAs** — close empty token accounts for rent refund

    { "*" }*Table Info:**
    • Checkbox to select wallets for batch operations
    • Name, address, SOL balance, token count, empty ATAs
    • Empty wallets are dimmed for easy identification

    { "*" }*Tips:**
    • Use after Multi-Sell to collect remaining SOL
    • Regularly cleanup ATAs to reclaim rent
    • Empty wallets can be reused for future operations

## config

hints-config-overview-title = Configuration
hints-config-overview-content =
    System-wide settings for ScreenerBot.

    Categories:
    • **Trader** — entry/exit rules, position sizing
    • **Filtering** — token filter thresholds
    • **Swaps** — routing and slippage settings
    • **RPC** — node configuration
    • **Services** — background service settings

    Changes take effect immediately (hot reload).
hints-config-telegram-title = Telegram Notifications
hints-config-telegram-content =
    { "*" }*Receive instant trading alerts via Telegram**

    Get notified about trades, positions, and important events directly in Telegram.

    { "*" }*Setup Steps:**

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

    { "*" }*What you'll receive:**
    • Trade execution confirmations
    • Position updates (entry/exit)
    • Trade Watcher alerts
    • Error notifications

    { "*" }*Privacy:**
    Messages are sent directly from ScreenerBot to your Telegram bot — no third-party servers involved.
hints-config-telegram-password-title = Bot Authentication Password
hints-config-telegram-password-content =
    { "*" }*Secure your Telegram bot with password authentication**

    When you interact with your ScreenerBot Telegram bot, you'll need to authenticate with this password before executing sensitive commands.

    { "*" }*Why set a password?**
    • Prevents unauthorized users from controlling your bot
    • Required for executing trading commands via Telegram
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
    • **Source:** Aggregated from DexScreener/GeckoTerminal, not direct on-chain RPC.

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

        Data is refreshed periodically from DexScreener/GeckoTerminal plus on-chain pool reads.
hints-token-details-market-pulse-title = Market Pulse
hints-token-details-market-pulse-content =
    Price movement and USD trading volume share the same **5M / 1H / 6H / 24H** timeline so momentum and participation can be compared directly.

    { "*" }*Interpretation:**
    • **Price** — aggregator-derived percentage change, not the live pool execution price.
    • **High Volume** — stronger interest, more efficient price discovery, and easier exits.
    • **Low Volume** — greater slippage, wider spreads, and harder large exits.
    • **High Volume + Low Liquidity** — elevated volatility and execution risk.

    Market data is aggregated across major DEXs via DexScreener/GeckoTerminal, so price change can differ from the current on-chain pool price.
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
    Risk assessment from Rugcheck.xyz and on-chain analysis.

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
    Boosted tokens first, then trending projects from Jupiter and DexScreener.

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
