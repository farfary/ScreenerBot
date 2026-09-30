# Source: templates/pages/onboarding.html

## Slide: welcome

onboarding-welcome-title = Welcome to { -brand }
onboarding-welcome-description = Your local-first Solana trading companion — built in Rust for native speed. Discover tokens, analyze markets, and control trading from your own machine.
onboarding-welcome-free-title = Free & Source-Available
onboarding-welcome-free-description = No subscriptions or paywalls. Inspect the code published on GitHub.
onboarding-welcome-custody-title = Self-Custody
onboarding-welcome-custody-description = Private keys encrypted at rest and never transmitted anywhere.
onboarding-welcome-engine-title = Always-On Engine
onboarding-welcome-engine-description = Orchestrated services with health checks and graceful lifecycle control.
onboarding-welcome-realtime-title = Real-Time On-Chain Data
onboarding-welcome-realtime-description = Direct pool reserve calculations — not delayed API snapshots.

## Slide: discover

onboarding-discover-title = Discover & Filter
onboarding-discover-description = Scan three data sources for new Solana pairs, decode 12+ DEX pool types on-chain, then run every token through configurable quality and security rules.
onboarding-discover-dex-title = Multi-DEX Discovery
onboarding-discover-dex-description = { -dexscreener }, { -geckoterminal }, and Raydium feeds for fresh { -sol } pairs.
onboarding-discover-scanner-title = Smart Token Scanner
onboarding-discover-scanner-description = Liquidity, volume, token age, holder distribution, and { -rugcheck } rules.
onboarding-discover-intelligence-title = Token Intelligence
onboarding-discover-intelligence-description = Market, security, and blacklist data unified in a single view.
onboarding-discover-price-action-title = Price Action Tracking
onboarding-discover-price-action-description = Seven timeframes of candle data with gap detection and momentum signals.

## Slide: trade

onboarding-trade-title = Trade Smart
onboarding-trade-description = Automated trading with a six-tier exit priority system. DCA into positions, set trailing stops, build strategy trees — or trade manually with one click.
onboarding-trade-auto-title = Auto-Trading
onboarding-trade-auto-description = Entry/exit evaluators, DCA rounds, partial exits, and trailing stop-loss.
onboarding-trade-strategy-title = Strategy Engine
onboarding-trade-strategy-description = Condition trees combining price, volume, and time-based signals.
onboarding-trade-routing-title = Best-Price Routing
onboarding-trade-routing-description = Concurrent quotes from every enabled router — best route wins.
onboarding-trade-safety-title = Safety Controls
onboarding-trade-safety-description = Emergency stop, period-based loss limits, and independent monitor toggles.

## Slide: connect

onboarding-connect-title = Stay Connected
onboarding-connect-description = Monitor your portfolio from anywhere. An Assistant backed by nine LLM providers, { -telegram } alerts with inline trading, and a searchable event log.
onboarding-connect-assistant-title = Assistant
onboarding-connect-assistant-description = Chat-driven analysis with tool calling for trades, config, and portfolio.
onboarding-connect-telegram-title = { -telegram } Integration
onboarding-connect-telegram-description = Notifications, inline commands, and 2FA-secured sessions from your phone.
onboarding-connect-wallets-title = Multi-Wallet Tracking
onboarding-connect-wallets-description = All your Solana wallets and token holdings in a single dashboard.
onboarding-connect-events-title = Live Event Stream
onboarding-connect-events-description = Every trade, swap, and system event logged with category and severity.

## Slide: data

onboarding-data-title = { -brand } Data
onboarding-data-description = We run a shared market-data service so every install is not separately rate limited by the public providers. It is free with a { -brand } account, and { -brand } works without one.
onboarding-data-candles-title = Pooled Candle History
onboarding-data-candles-description = Seven timeframes of shared history, years deep, served from one cache.
onboarding-data-pools-title = Resolved Pools & Security
onboarding-data-pools-description = A central pool registry and cached { -rugcheck } reports, already fetched.
onboarding-data-signin-title = Sign In To Use It
onboarding-data-signin-description = Without an account this data is unavailable and the public providers are used.
onboarding-data-reading-title = Reading Only
onboarding-data-reading-description = We see which tokens you look up. Never a key, a balance, a position or a trade.

## Slide: privacy

onboarding-privacy-title = Your Keys, Your Data
onboarding-privacy-description = Your configuration, keys and trading history stay on this machine. Next, choose Explore Mode for discovery without credentials, or link a wallet and RPC to enable the full bot — and sign in there if you want { -brand } data.
onboarding-privacy-local-title = Local-First Architecture
onboarding-privacy-local-description = Config, analytics, and databases stored on your desktop.
onboarding-privacy-wallet-title = Encrypted Wallet
onboarding-privacy-wallet-description = Your private key is encrypted at rest and never transmitted.
onboarding-privacy-security-title = Dashboard Security
onboarding-privacy-security-description = Password lock, TOTP two-factor, and session timeout protection.
onboarding-privacy-config-title = Flexible Configuration
onboarding-privacy-config-description = Most settings can be adjusted from the dashboard after setup.

## Footer

onboarding-setup-shortcut =
    .aria-label = Go directly to wallet and RPC setup or choose Explore Mode
onboarding-setup-shortcut-label = Go to setup
onboarding-progress-dot =
    .aria-label = Go to slide { $number }
onboarding-action-continue-to-setup = Continue to setup
