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
