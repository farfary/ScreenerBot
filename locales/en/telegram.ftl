# Telegram bot text. Server-only: rendered by src/telegram/text.rs, never sent to the dashboard.
#
# Messages are sent as Telegram HTML. The only tags are b, i, u, s, code and pre,
# without attributes; links are built in Rust. A line break is a literal newline.
# Icons are prepended by Rust and never appear here. Copyable values (chat ids)
# arrive as arguments and are wrapped in code inside the message. Keep the
# command names (/status) and the literal ampersand placeable unchanged.

## Reply keyboard words. Icons come from ReplyCommand in src/telegram/reply.rs.

telegram-reply-status = Status
telegram-reply-balance = Balance
telegram-reply-positions = Positions
telegram-reply-pause = Pause
telegram-reply-resume = Resume
telegram-reply-stop = Stop
telegram-reply-stats = Stats
telegram-reply-menu = Menu
telegram-reply-help = Help

## Inline keyboard buttons.

telegram-button-positions = Positions
telegram-button-balance = Balance
telegram-button-stats = Stats
telegram-button-tokens = Tokens
telegram-button-pause = Pause
telegram-button-stop = Stop
telegram-button-settings = Settings
telegram-button-refresh = Refresh
telegram-button-menu = Menu
telegram-button-back = Back
telegram-button-back-to-menu = Back to Menu
telegram-button-back-to-tokens = Back to Tokens
telegram-button-cancel = Cancel
telegram-button-close-all-positions = Close All Positions
telegram-button-sell-percent = Sell { $percent }%
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = Blacklist
telegram-button-blacklist-symbol = Blacklist { $symbol }
telegram-button-close-position = Close Position
telegram-button-confirm-close = Confirm Close
telegram-button-confirm-close-all = Close ALL Positions
telegram-button-confirm-sell = Confirm Sell { $percent }%
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = CONFIRM FORCE STOP
telegram-button-confirm-buy = Buy { $amount } { -sol }
telegram-button-notifications = Notifications
telegram-button-trading = Trading
telegram-button-entry-monitor = Entry Monitor
telegram-button-exit-monitor = Exit Monitor
telegram-button-auto-trading = Auto Trading
telegram-button-force-stop = Force Stop
telegram-button-notify-opened = Opened
telegram-button-notify-closed = Closed
telegram-button-notify-partial = Partial
telegram-button-notify-dca = DCA
telegram-button-notify-errors = Errors
telegram-button-details = Details
telegram-button-position = Position
telegram-button-sell-more = Sell More
telegram-button-more-dca = More DCA
telegram-button-history = History
telegram-button-status = Status
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = Re-authenticate
telegram-button-previous = Prev
telegram-button-next = Next
telegram-button-passed = Passed
telegram-button-rejected = Rejected
telegram-button-new-24h = New (24h)
telegram-button-all-tokens = All Tokens
telegram-button-search-token = Search Token
telegram-button-filter-stats = Filter Stats
telegram-button-refresh-stats = Refresh Stats
telegram-button-view-position = View Position
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    Unknown command: { $command }

    Use /help to see available commands.
telegram-session-expired =
    <b>Session Expired</b>

    Use /login to re-authenticate.
telegram-2fa-required =
    <b>2FA Required</b>

    Please enter your 6-digit authenticator code.
telegram-account-locked =
    <b>Account Locked</b>

    Too many failed attempts.
    Try again in { $seconds ->
        [one] { $seconds } second.
       *[other] { $seconds } seconds.
    }
telegram-code-invalid = Please enter a valid 6-digit code.
telegram-authenticated =
    <b>Authenticated!</b>

    You now have access to bot commands.
telegram-wrong-code =
    <b>Wrong Code</b>

    { $remaining ->
        [one] { $remaining } attempt remaining.
       *[other] { $remaining } attempts remaining.
    }
telegram-auth-required =
    <b>Authentication Required</b>

    Please enter your password to continue.

    <i>Type your password and send it.</i>
telegram-login-required =
    <b>Login Required</b>

    Please enter your 6-digit authenticator code:
telegram-session-activated =
    <b>Session Activated</b>

    2FA is not configured. Your session is now active.

    <i>Tip: Enable 2FA in Security settings for better security.</i>

## Chat discovery.

telegram-discovery-hello = Hello { $name }!
telegram-discovery-default-name = User
telegram-discovery-detected = <b>Chat detected!</b>
telegram-discovery-details =
    Chat ID: <code>{ $chat_id }</code>
    Type: { $chat_type }

    Please go to the { -brand } dashboard and click on this chat to select it.
telegram-chat-type-private = private
telegram-chat-type-group = group
telegram-chat-type-supergroup = supergroup
telegram-chat-type-channel = channel

## Menus.

telegram-menu-title =
    <b>Control Panel</b>

    Select an option to view information or control the bot.
telegram-menu-positions-empty =
    <b>No Open Positions</b>

    Waiting for new opportunities...
telegram-menu-positions-title = <b>Positions ({ $count })</b>
telegram-menu-positions-hint = <i>Tap a position to manage it.</i>
telegram-menu-settings =
    <b>Settings</b>

    Configure notifications and trading parameters.
telegram-settings-notifications =
    <b>Notification Settings</b>

    Toggle notifications on/off:
telegram-settings-trading =
    <b>Trading Controls</b>

    Toggle trading features:
telegram-pagination-expired = Pagination session expired.

## Status commands.

telegram-status-state-stopped = <b>STOPPED</b> (Force Stop Active)
telegram-status-state-active = <b>ACTIVE</b>
telegram-status-state-paused = <b>PAUSED</b>
telegram-status-on = ON
telegram-status-off = OFF
telegram-status-body =
    <b>System Status</b>

    <b>System</b>
    State — { $state }
    Uptime — { $uptime }
    Version — v{ $version }

    <b>Trading</b>
    Entries — { $entries }
    Exits — { $exits }
    Positions — { $positions }
telegram-positions-empty =
    <b>No Open Positions</b>

    Waiting for opportunities...
telegram-positions-title = <b>Open Positions ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+{ $count } more...</i>
telegram-positions-summary =
    <b>Portfolio Summary</b>
    Invested — { $invested } { -sol }
    Net P{ "&amp;" }L — { $pnl } { -sol }
telegram-balance-body =
    <b>Wallet Balance</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>Daily Statistics</b>

    Positions — { $positions }
    Invested — { $invested } { -sol }
    P{ "&amp;" }L — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } is Ready!</b>

    Trading is <b>enabled</b>.

    Use the keyboard below to control the bot.
    Type /help for available commands.
telegram-stop-already = <b>Trading is already disabled</b>
telegram-stop-done =
    <b>Trading Disabled</b>

    All trading monitors (entries { "&amp;" } exits) are stopped.
    Use /pause to stop only entries.
telegram-stop-failed =
    <b>Failed to disable trading</b>

    Error: { $detail }
telegram-pause-done =
    <b>Entry Monitor Paused</b>

    No new positions will be opened.
    Exit monitor continues running.
telegram-pause-failed =
    <b>Failed to pause entries</b>

    Error: { $detail }
telegram-resume-done =
    <b>Entry Monitor Resumed</b>

    Now watching for entry signals.
telegram-resume-failed =
    <b>Failed to resume entries</b>

    Error: { $detail }
telegram-force-stop-confirm =
    <b>FORCE STOP</b>

    This will immediately halt ALL trading activity:
    • No new entries
    • No exits (including stop losses)
    • No DCA operations
telegram-force-stop-warning = <b>This is an emergency action!</b>
telegram-force-stop-question = Are you sure?
telegram-force-stop-active =
    <b>FORCE STOP ACTIVATED</b>

    All trading has been halted.

    Use /resume_trading to clear this flag.
telegram-resume-trading-not-stopped =
    <b>Trading is not force-stopped</b>

    No action needed.
telegram-resume-trading-done =
    <b>Trading Resumed</b>

    Force stop flag has been cleared.
    Normal trading operations can now resume.

## Help.

telegram-help-title = <b>{ -brand } Help</b>
telegram-help-heading-dashboard = Dashboard
telegram-help-heading-market = Market
telegram-help-heading-trading = Trading
telegram-help-heading-safety = Safety
telegram-help-heading-system = System
telegram-help-commands-dashboard =
    /status — System status { "&amp;" } uptime
    /stats — Daily performance
    /balance — Wallet balance
    /positions — Open positions
telegram-help-commands-market =
    /tokens — Token explorer
    /rejected — Filtered tokens
telegram-help-commands-trading =
    /start — Enable trading system
    /stop — Disable trading system
    /pause — Pause new entries
    /resume — Resume new entries
    /menu — Interactive menu
telegram-help-commands-safety =
    /force_stop — <b>EMERGENCY HALT</b>
    /resume_trading — Clear emergency status
telegram-help-commands-system =
    /update — Update status { "&amp;" } install
    /login — 2FA Authentication
telegram-help-tip = <i>Tip: Tap a command to run it.</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>Up to date</b>

    Running v{ $version }, installed automatically.
telegram-update-up-to-date =
    <b>Up to date</b>

    Running v{ $version }.
telegram-update-check-failed =
    <b>Update check failed</b>

    { $reason }
telegram-update-unreachable = screenerbot.io could not be reached.
telegram-update-installing = <b>Installing v{ $version }</b>
telegram-update-restarting =
    { -brand } is restarting onto the new version. Trading resumes automatically.
telegram-update-install-failed =
    <b>Could not install v{ $version }</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } is downloaded</b>

    This release also updates the desktop app, so its installer has to run on the machine. Open Settings → Updates there.
telegram-update-downloading =
    <b>Downloading v{ $version }</b>

    { $percent }% of { $size } MB.
telegram-update-available =
    <b>v{ $version } is available</b>

    { $how }
    Download size: { $size } MB.

    It downloads on its own; send /update again once it is ready.
telegram-update-how-core = Installs silently with a short restart.
telegram-update-how-installer = Needs the desktop installer to run once.

## Shared values and units. Numbers arrive formatted; only the unit words live here.

telegram-value-unknown = Unknown
telegram-value-na = N/A
telegram-percent-value = { $percent }%
telegram-price-sol = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds }s
telegram-duration-minutes = { $minutes }m
telegram-duration-minutes-seconds = { $minutes }m { $seconds }s
telegram-duration-hours = { $hours }h
telegram-duration-hours-minutes = { $hours }h { $minutes }m
telegram-duration-days = { $days }d
telegram-duration-days-hours = { $days }d { $hours }h
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-sol = { $amount } { -sol }
telegram-error-line = Error: { $detail }
telegram-ai-reasoning =
    <b>LLM Analysis</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens. Icons come from Rust.

telegram-row-entry = Entry — { $price } { -sol }
telegram-row-exit = Exit — { $price } { -sol }
telegram-row-current = Current — { $price } { -sol }
telegram-row-invested = Invested — { $amount } { -sol }
telegram-row-received = Received — { $amount } { -sol }
telegram-row-value = Value — { $amount } { -sol }
telegram-row-total = Total — { $amount } { -sol }
telegram-row-tokens = Tokens — { $tokens }
telegram-row-duration = Duration — { $duration }
telegram-row-reason = Reason — { $reason }
telegram-row-remaining = Remaining — { $percent }%
telegram-row-pnl = P{ "&amp;" }L — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>Position Opened</b>
telegram-notify-opened-size = Size — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = Price — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>Position Closed</b> — Profit
telegram-notify-closed-title-loss = <b>Position Closed</b> — Loss
telegram-notify-partial-title = <b>Partial Exit</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — Sold { $percent }%
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = Added — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = Avg — { $price } { -sol }
telegram-notify-severity-critical = <b>Critical Error</b>
telegram-notify-severity-error = <b>Error</b>
telegram-notify-severity-warning = <b>Warning</b>
telegram-notify-severity-info = <b>Info</b>
telegram-notify-alert-title = <b>Trade Alert</b>
telegram-notify-alert-token = Token: <code>${ $symbol }</code>
telegram-notify-alert-mint = Mint: <code>{ $mint }</code>
telegram-notify-alert-bought = Action: bought { $amount } { -sol }
telegram-notify-alert-sold = Action: sold { $amount } { -sol }
telegram-notify-alert-wallet = Wallet: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (paper)
telegram-notify-copy-task = Task: { $task }
telegram-notify-command = <b>Command:</b> /{ $command }
telegram-notify-summary-title = <b>Daily Summary</b> — { $date }
telegram-notify-summary-performance = <b>Performance</b>
telegram-notify-summary-trades = Trades — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = Win Rate — { $percent }%
telegram-notify-summary-pnl = P{ "&amp;" }L — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = Open Positions — { $count }
telegram-notify-started-title = <b>{ -brand } Started</b>
telegram-notify-started-version = <b>Version</b> — { $version }
telegram-notify-started-mode = <b>Mode</b> — { $mode }
telegram-notify-started-ready = Ready for trading!
telegram-notify-stopped-title = <b>{ -brand } Stopped</b>
telegram-notify-stopped-reason = <b>Reason</b> — { $reason }
telegram-notify-stopped-goodbye = Goodbye! { $icon }
telegram-notify-update-available =
    <b>Update v{ $version } available</b>

    { $how }
    Download size: { $size } MB
telegram-notify-update-how-installer = This release also updates the desktop app, so its installer has to run once.
telegram-notify-update-ready =
    <b>Update v{ $version } ready</b>

    { $how }
telegram-notify-update-ready-silent = Send /update to apply it now, or it installs the next time { -brand } starts.
telegram-notify-update-ready-installer = Open Settings → Updates to run the installer.
telegram-notify-update-applying =
    <b>Installing v{ $version }</b>

    The backend is restarting; trading resumes automatically.
telegram-notify-new-tokens =
    <b>Filtering Alert</b>

    { $count ->
        [one] Found { $count } new token matching your criteria.
       *[other] Found { $count } new tokens matching your criteria.
    }
telegram-notify-crash =
    <b>Bot Crashed!</b>

    <b>Location:</b> <code>{ $location }</code>
    <b>Error:</b> <code>{ $error }</code>
telegram-notify-crash-restart = Please restart the bot.

## Filter results page.

telegram-filter-results-title = <b>Filter Results</b> ({ $count })
telegram-filter-results-empty = <i>No tokens found.</i>
telegram-filter-results-page = <i>Page { $page } of { $total }</i>

## Position screens.

telegram-position-not-found = Position not found
telegram-position-no-positions = No positions to close
telegram-position-history-empty =
    <b>Trade History</b>

    No closed positions yet.
telegram-position-history-title = <b>Recent Trades</b>
telegram-position-history-more = <i>+{ $count } more trades...</i>
telegram-position-confirm-hint = <i>Confirm within 30s to execute.</i>
telegram-position-confirm-close-title = <b>Close Position?</b>
telegram-position-confirm-close-selling = Selling { $tokens } tokens
telegram-position-confirm-close-estimated = Estimated — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>Confirm within 30 seconds</i>
telegram-position-confirm-sell =
    <b>Confirm Sell</b>

    Token — { $symbol }
    Amount — { $percent }%
    Tokens — { $tokens }
telegram-position-confirm-dca =
    <b>Confirm Buy More</b>

    Token — { $symbol }
    Add — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>Close All Positions?</b>

    Count — { $count }
telegram-position-confirm-close-all-hint =
    <i>This will market sell all open positions.
    Confirm within 30s.</i>
telegram-position-confirm-force-stop =
    <b>FORCE STOP</b>

    This will immediately halt ALL trading:
    • No new entries
    • No exits
    • No DCA
telegram-position-confirm-force-stop-warning = <b>This is an emergency action.</b>
telegram-position-confirm-blacklist =
    <b>Blacklist Token?</b>

    Token — { $symbol }
    Mint — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>This will close the position and prevent future entries.</i>
telegram-position-selling = Selling { $percent }% of { $symbol }...
telegram-position-sell-done =
    <b>Sell Executed</b>

    Token — { $symbol }
    Sold — { $percent }%
    Received — { $amount } { -sol }
telegram-position-sell-failed = <b>Sell Failed</b>
telegram-position-adding = Adding { $amount } { -sol } to { $symbol }...
telegram-position-dca-done =
    <b>DCA Executed</b>

    Token — { $symbol }
    Added — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA Failed</b>
telegram-position-closing-all = Closing all positions...
telegram-position-close-all-done =
    <b>Close All Complete</b>

    Closed — { $closed }
    Failed — { $failed }
telegram-position-blacklisted =
    <b>Token Blacklisted</b>

    Token — { $symbol }
    Status — Closed { "&amp;" } Blacklisted

## Token screens.

telegram-token-not-found = Token not found
telegram-token-not-found-prefix = Token not found. Try searching with a longer prefix.
telegram-token-stats-failed = Failed to fetch stats: { $detail }
telegram-token-list-failed = Failed to fetch tokens: { $detail }
telegram-token-list-empty = No tokens found in <b>{ $view }</b> view.
telegram-token-view-passed = Passed Filter
telegram-token-view-rejected = Rejected
telegram-token-view-recent = Recently Added
telegram-token-view-all = All Tokens
telegram-token-list-title = <b>{ $name }</b> (Page { $page }/{ $total })
telegram-token-list-stats = Liq: { $liquidity } • Price: { $price }
telegram-token-list-hint = <i>Tap /token_ID to view details</i>
telegram-token-explorer =
    <b>Market Explorer</b>

    <b>Overview</b>
    Passed Filter — { $passed }
    Rejected — { $rejected }
    Active Prices — { $priced }
    Total Discovered — { $total }

    <i>Select a category to browse:</i>
telegram-token-filter-title = <b>Filter Analysis</b>
telegram-token-filter-distribution = <b>Distribution</b>
telegram-token-filter-passed = Passed — { $count } ({ $percent }%)
telegram-token-filter-rejected = Rejected — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = Blacklisted — { $count }
telegram-token-filter-coverage = <b>Coverage</b>
telegram-token-filter-priced = With Pool Price — { $count }
telegram-token-filter-open = Open Positions — { $count }
telegram-token-filter-total = Total Discovered — { $count }
telegram-token-filter-updated = <b>Last Updated</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>Auto-refreshes every { $interval }</i>
telegram-token-detail-active = <b>Active Position</b>
telegram-token-detail-price = Price — { $price } { -sol }
telegram-token-detail-liquidity = Liquidity — { $value }
telegram-token-detail-volume = 24h Volume — { $value }
telegram-token-detail-change = 24h Change — { $value }
telegram-token-detail-risk = Risk Assessment: { $score }/100
telegram-token-detail-risk-unknown = Risk Assessment: Unknown
telegram-token-detail-action = <i>Select action:</i>
telegram-token-search =
    <b>Search Market</b>

    Enter symbol or mint address to search:

    <i>Example: /token_BONK or /token_So11111</i>
telegram-token-confirm-buy =
    <b>Confirm Direct Buy</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>
    Amount — { $amount } { -sol }

    <i>Confirm within 30s to execute.</i>
telegram-token-confirm-blacklist =
    <b>Blacklist Token?</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>

    <i>This will prevent this token from satisfying filters.</i>
telegram-token-blacklisted =
    <b>Token Blacklisted</b>

    Token — ${ $symbol }
    Status — Added to blacklist
telegram-token-blacklist-failed = <b>Blacklist Failed</b>
telegram-token-buy-processing =
    <b>Processing Buy...</b>

    Token — ${ $symbol }
    Amount — { $amount } { -sol }
telegram-token-buy-done =
    <b>Buy Successful</b>

    Token — ${ $symbol }
    Amount — { $amount } { -sol }

    <i>View details in /positions</i>
telegram-token-buy-failed =
    <b>Buy Failed</b>

    Token — ${ $symbol }
    Error — { $detail }
