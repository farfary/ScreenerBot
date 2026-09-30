# Dashboard shell: header, ticker, notification drawer and status bar.

# Source: templates/base.html
# Document title: the page title, then the product name.
shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-brand-logo =
    .alt = { -brand }
# Version label; the number itself is passed as an argument.
shell-version = v{ $version }

## Header

shell-header-brand =
    .aria-label = Open dashboard home
    .title = Dashboard home
shell-bot-card =
    .aria-label = Auto Trader status loading
shell-bot-label = Auto
shell-bot-status-loading = LOADING
shell-bot-today = Today
shell-explore-control =
    .aria-label = Explore Mode. Connect a wallet and RPC endpoint to enable all features
    .title = Connect a wallet and RPC endpoint to enable trading, balances, and live on-chain data
shell-explore-title = Explore Mode
shell-explore-detail = Wallet & RPC not connected
shell-explore-action = Complete setup
shell-wallet-card =
    .aria-label = Wallet worth; open Positions
    .title = Wallet worth ({ -sol } + tokens) · open Positions
shell-wallet-worth-label = WORTH
shell-wallet-sol-label = { -sol }
shell-wallet-tokens-label = TKN
shell-sol-price-card =
    .aria-label = { -sol } price in USD — open chart
    .title = { -sol } price · click for chart
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24h
shell-copy-card =
    .aria-label = Copy trading; open Copy Trading
    .title = Copy trading · open Copy Trading
shell-copy-label = COPY
shell-actions-more =
    .aria-label = More header actions
    .title = More actions
shell-actions-group =
    .aria-label = Header actions
shell-action-search =
    .aria-label = Search tokens
    .title = Search tokens (Ctrl/Cmd+K)
shell-action-featured =
    .aria-label = Featured tokens
    .title = Featured tokens
shell-action-notifications =
    .aria-label = Actions and notifications
    .title = Actions and notifications
shell-action-restart =
    .aria-label = Restart app
    .title = Restart app
shell-action-theme =
    .aria-label = Toggle theme
    .title = Toggle theme
shell-action-settings =
    .aria-label = Settings
    .title = Settings

## Ticker

shell-ticker-monitoring-segment =
    .title = Tokens being monitored by Pool Service
shell-ticker-monitoring = Monitoring:
shell-ticker-filtering-segment =
    .title = Tokens that passed/failed filtering criteria
shell-ticker-passed = Passed:
shell-ticker-rejected = Rejected:
shell-ticker-pnl-segment =
    .title = Today's realized profit and loss
shell-ticker-pnl = Today P&L:
shell-ticker-rpc-segment =
    .title = RPC calls per minute and success rate
shell-ticker-rpc = RPC:
shell-ticker-rpc-per-minute = /min
shell-ticker-services-segment =
    .title = Background services health status
shell-ticker-services = Services:
shell-ticker-services-loading = Loading

## Notification drawer

shell-notification-title = Actions
shell-notification-mark-all-read =
    .title = Mark all as read
shell-notification-clear-all =
    .title = Clear all
shell-notification-close =
    .aria-label = Close
shell-notification-tab-all = All
shell-notification-tab-active = Active
shell-notification-tab-done = Done
shell-notification-tab-failed = Failed
shell-notification-filter-type-all = All Types
shell-notification-filter-type-buy = Buy
shell-notification-filter-type-sell = Sell
shell-notification-filter-type-open = Open
shell-notification-filter-type-close = Close
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = Partial
shell-notification-filter-state-all = All States
shell-notification-filter-state-in-progress = In Progress
shell-notification-filter-state-completed = Completed
shell-notification-filter-state-failed = Failed
shell-notification-filter-state-cancelled = Cancelled
shell-notification-list =
    .aria-label = Notifications
shell-notification-empty = No actions yet
shell-notification-loading-more = Loading more...
shell-notification-back-to-top =
    .title = Back to top

## Status bar

shell-status-bar-version = v
shell-status-bar-uptime = Up
shell-status-bar-memory = Mem
shell-status-bar-rpc = RPC
shell-status-bar-trading = Trading
shell-status-bar-positions = Pos
shell-status-bar-tokens = Tokens

# Source: templates/pages/splash.html, scripts/core/splash.js

## Splash

shell-splash-starting = Starting { -brand }
shell-splash-waiting = Waiting for the local core to answer.
shell-splash-failed = { -brand } could not start
shell-splash-failed-detail = Check the log file, then restart the app.
