# Dashboard shell: header, ticker, notification drawer and status bar.

# Source: templates/base.html
# Document title: the page title, then the product name.
shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
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
shell-setup-gate-detail = Explore Mode runs without a wallet or RPC. Complete setup to connect them.
shell-wallet-card =
    .aria-label = Wallet worth; open Positions
    .title = Wallet worth ({ -sol } + tokens) · open Positions
shell-wallet-worth-label = WORTH
shell-wallet-native-label = { -sol }
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
shell-tabs-scroll-start =
    .aria-label = Show earlier tabs
    .title = Show earlier tabs
shell-tabs-scroll-end =
    .aria-label = Show more tabs
    .title = Show more tabs
shell-ticker-scroll-start =
    .aria-label = Show earlier metrics
    .title = Show earlier metrics
shell-ticker-scroll-end =
    .aria-label = Show more metrics
    .title = Show more metrics

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
shell-ticker-rpc-rate = { $amount }/min
shell-ticker-services-segment =
    .title = Background services health status
shell-ticker-services-loading = Services: <strong>Loading</strong>

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
# RPC calls in the last minute; $rate is a formatted number.
shell-status-rpc-per-minute = { $rate }/min
shell-status-bar-trading = Trading
shell-status-bar-positions = Pos
shell-status-bar-tokens = Tokens

# Source: templates/pages/splash.html, scripts/core/splash.js

## Splash

shell-splash-starting = Starting { -brand }
shell-splash-waiting = Waiting for the local core to answer.
shell-splash-failed = { -brand } could not start
shell-splash-failed-detail = Check the log file, then restart the app.

# Source: scripts/core/header.js, scripts/core/connectivity_watcher.js, scripts/core/router.js

## Connection state

shell-connection-connected = Core Connected
shell-connection-waiting = Waiting for core…
shell-connection-retry-now = Retry now
shell-connection-overlay-detail = The core is unreachable. Trading is paused; this will recover automatically.
shell-connection-restored = Core connection restored

# Source: scripts/core/header.js
shell-trader-control-failed = Trader control failed
shell-notification-button-unread = Actions and notifications, { $count } unread
shell-restart-confirm-title = Restart Bot
shell-restart-confirm-message =
    Are you sure you want to restart the bot?

    This will:
    • Stop all services
    • Restart the process
    • Take ~10-15 seconds

    All active operations will be interrupted.
shell-restart-confirm-action = Restart
shell-restart-progress = Restarting bot
shell-restart-failed = Restart failed
shell-restart-failed-status = Restart failed: { $status }
shell-restart-helper-unavailable = Automatic restart helper is unavailable. Reload the dashboard shortly.

# Source: scripts/core/router.js
shell-page-title-fallback = Dashboard
shell-page-load-failed = Failed to Load Page
shell-page-offline-detail = The core is unreachable right now. This page will load automatically once the connection is back.

# Source: scripts/core/header_metrics.js

## Auto Trader card

shell-bot-state-explore = EXPLORE
shell-bot-state-halted = HALTED
shell-bot-state-off = OFF
shell-bot-state-waiting = WAITING
shell-bot-state-idle = IDLE
shell-bot-state-entry-paused = ENTRY PAUSED
shell-bot-state-running = RUNNING
shell-bot-control-explore = Auto Trader unavailable in Explore Mode. Open wallet and RPC setup.
shell-bot-control-halted = Emergency stop is active. Open Auto Trader controls.
shell-bot-control-off = Auto Trader is off. Click to enable it.
shell-bot-control-waiting = Auto Trader is enabled and waiting for core services. Click to disable it.
shell-bot-control-idle = Auto Trader is enabled, but both monitors are off. Open Auto Trader controls.
shell-bot-control-entry-paused = Loss protection paused entries; exits can continue. Open Auto Trader controls.
shell-bot-control-running = Auto Trader is running. Click to disable it.

## Wallet and copy cards

shell-wallet-card-summary = Wallet worth: { $equity } { -sol } ({ $balance } { -sol } cash, { $tokens } tokens); open Positions
shell-copy-running-live = { $count } live
shell-copy-running-paper = { $count } paper
shell-copy-value-paused = Paused
shell-copy-value-idle = Idle
shell-copy-sub-active = { $active } of { $total } active

## Ticker services state

shell-ticker-services-healthy = Services: <strong>Healthy</strong>
shell-ticker-services-issues =
    { $count ->
        [one] Services: <strong>{ $count } Issue</strong>
       *[other] Services: <strong>{ $count } Issues</strong>
    }

# Source: scripts/core/agent_approvals.js

## Agent approval prompt

shell-agent-request-title = Agent request
shell-agent-request-client-fallback = A paired agent
shell-agent-request-message = { $client } wants to run "{ $tool }" in { -brand }. This request { $expiry }.
shell-agent-request-message-arguments = { $client } wants to run "{ $tool }" in { -brand }. Arguments: { $summary }. This request { $expiry }.
shell-agent-request-expires-minutes = expires in { $minutes }m
shell-agent-request-expires-seconds = expires in { $seconds }s
shell-agent-request-approve = Approve
shell-agent-request-deny = Deny

# Source: scripts/core/utils.js, scripts/core/toast.js, scripts/ui/toast.js, scripts/ui/confirmation_dialog.js

## Toasts, dialogs and shared widgets

shell-toast-copied = { $label } copied
shell-toast-copy-failed = Copy failed
shell-toast-still-running = Still running — check the notification center
shell-toast-dismiss =
    .aria-label = Dismiss
shell-confirm-title = Confirm Action
shell-confirm-message = Are you sure?

# Source: scripts/core/global_chat.js
shell-assistant-label = Assistant
shell-assistant-dialog =
    .aria-label = Assistant

# Source: scripts/core/status_bar.js
shell-status-bar-trading-active = Active
shell-status-bar-trading-inactive = Inactive

# Source: scripts/core/action_toasts.js

## Action toasts

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title } cancelled
shell-action-swap-buy-live = Buying
shell-action-swap-buy-done = Bought
shell-action-swap-buy-failed = Buy failed
shell-action-swap-sell-live = Selling
shell-action-swap-sell-done = Sold
shell-action-swap-sell-failed = Sell failed
shell-action-position-open-live = Opening position
shell-action-position-open-done = Opened
shell-action-position-open-failed = Open failed
shell-action-position-close-live = Closing position
shell-action-position-close-done = Closed
shell-action-position-close-failed = Close failed
shell-action-position-dca-live = Adding to position
shell-action-position-dca-done = Added to
shell-action-position-dca-failed = Add failed
shell-action-partial-exit-live = Partial exit
shell-action-partial-exit-done = Partial exit
shell-action-partial-exit-failed = Partial exit failed
shell-action-manual-order-live = Placing order
shell-action-manual-order-done = Order placed
shell-action-manual-order-failed = Order failed
shell-action-trade-live = Trade
shell-action-trade-done = Trade done
shell-action-trade-failed = Trade failed

# Source: scripts/core/action_message.js
shell-action-via-router = { $action } via { $router }
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = avoiding { $venue }
shell-action-cost-guard-avoiding-cost = avoiding { $venue } · { $cost }
shell-action-cost-guard-avoiding-unnamed = avoiding a venue
shell-action-cost-guard-avoiding-unnamed-cost = avoiding a venue · { $cost }
shell-action-cost-guard-avoided = { $outcome } · avoided { $cost } in { $venue } rent
shell-action-cost-guard-avoided-unnamed = { $outcome } · avoided { $cost } in venue rent
shell-action-exit-full = Full exit
shell-action-exit-percent = { $percent } exit

## Exit dialog (ui/exit_dialog.js)

shell-exit-title = Close { -brand }?
shell-exit-description = Choose how you'd like to close the application
shell-exit-minimize = Minimize to Tray
shell-exit-minimize-detail = Keep running in background
shell-exit-quit = Exit App
shell-exit-quit-detail = Close completely and stop all services

## Image lightbox (ui/image_lightbox.js)

shell-lightbox-save =
    .title = Save image
shell-lightbox-close =
    .title = Close (ESC)

## Theme control (scripts/theme.js)

shell-theme-light = Light
shell-theme-dark = Dark
shell-theme-switch-to-light = Switch to light theme
shell-theme-switch-to-dark = Switch to dark theme
