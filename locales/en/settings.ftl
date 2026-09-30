# The Settings dialog. Each section is named after the script that owns it.

## Shared

settings-duration-minutes =
    { $count ->
        [one] { $count } minute
       *[other] { $count } minutes
    }
settings-duration-hours =
    { $count ->
        [one] { $count } hour
       *[other] { $count } hours
    }

## settings_dialog.js

settings-dialog-title = Settings
settings-dialog-close =
    .title = Close (ESC)
    .aria-label = Close settings
settings-dialog-save = Save Changes
settings-dialog-saving = Saving...
settings-dialog-saved = Saved
settings-dialog-save-success = Settings saved successfully
settings-dialog-save-failed = Failed to save settings
settings-dialog-update-attention = Update needs attention
settings-dialog-tab-interface = Interface
settings-dialog-tab-navigation = Navigation
settings-dialog-tab-startup = Startup
settings-dialog-tab-hints = Hints
settings-dialog-tab-data = Data
settings-dialog-tab-security = Security
settings-dialog-tab-account = Account
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = Agent Connections
settings-dialog-tab-updates = Updates
settings-dialog-tab-licenses = Licenses
settings-dialog-tab-about = About
settings-dialog-link-privacy = Privacy Policy
settings-dialog-link-terms = Terms of Service

## settings_dialog.js: Startup tab

settings-startup-section-title = Startup Behavior
settings-startup-auto-start-label = Auto-start Trader
settings-startup-auto-start-hint = Automatically start trader on launch
settings-startup-coming-soon = Coming Soon
settings-startup-default-page-label = Default Page
settings-startup-default-page-hint = Page to show when opening the app
settings-startup-page-dashboard = Dashboard
settings-startup-page-tokens = Tokens
settings-startup-page-positions = Positions
settings-startup-page-wallet = Wallet
settings-startup-page-config = Config
settings-startup-notifications-label = Show Background Notifications
settings-startup-notifications-hint = Display notifications for background events

## settings_dialog.js: About tab

settings-about-logo =
    .alt = { -brand }
settings-about-tagline = Native Solana Trading Engine
settings-about-link-github = { -github }
settings-about-link-docs = Documentation
settings-about-link-telegram = { -telegram }
settings-about-link-website = Website
settings-about-credits = Built for Solana traders
settings-about-copyright = © { $year } { -brand }. All rights reserved.

## interface_tab.js

settings-interface-section-appearance = Appearance
settings-interface-theme-label = Theme
settings-interface-theme-hint = Choose your preferred color scheme
settings-interface-theme-dark = Dark
settings-interface-theme-light = Light
settings-interface-language-label = Language
settings-interface-language-hint = Display language of the dashboard
settings-interface-logo-shape-label = Token Logo Shape
settings-interface-logo-shape-hint = Circle crops every logo; Natural preserves each artwork's own silhouette
settings-interface-logo-shape-circle = Circle
settings-interface-logo-shape-natural = Natural
settings-interface-animations-label = Enable Animations
settings-interface-animations-hint = Smooth transitions and effects
settings-interface-compact-label = Compact Mode
settings-interface-compact-hint = Reduce padding for more content
settings-interface-section-data = Data & Display
settings-interface-refresh-label = Refresh Interval
settings-interface-refresh-hint = How often to refresh data
settings-interface-refresh-seconds =
    { $count ->
        [one] { $count } second
       *[other] { $count } seconds
    }
settings-interface-refresh-minutes =
    { $count ->
        [one] { $count } minute
       *[other] { $count } minutes
    }
settings-interface-ticker-label = Show Ticker Bar
settings-interface-ticker-hint = Live metrics ticker in header
settings-interface-page-size-label = Table Page Size
settings-interface-page-size-hint = Default rows per table page
settings-interface-page-size-rows =
    { $count ->
        [one] { $count } row
       *[other] { $count } rows
    }
settings-interface-auto-expand-label = Auto-expand Categories
settings-interface-auto-expand-hint = Expand config categories by default
settings-interface-hints-label = Show Contextual Hints
settings-interface-hints-hint = Display help icons explaining dashboard features
settings-interface-featured-label = Show Featured Row
settings-interface-featured-hint = Display featured tokens row on Home and Tokens pages
settings-interface-section-sound = Sound Effects
settings-interface-sounds-label = Enable Sounds
settings-interface-sounds-hint = Tactile cues for navigation, state changes, and outcomes

## security_tab.js

settings-security-loading = Loading security settings...
settings-security-load-failed = Failed to load security settings

# Password types of the lockscreen. The ids are the stored `password_type` values.
settings-security-type-pin4 = 4-Digit PIN
settings-security-type-pin6 = 6-Digit PIN
settings-security-type-text = Text Password
settings-security-type-unset = Not Set

settings-security-lockscreen-title = Dashboard Lockscreen
settings-security-lockscreen-description = Protect your dashboard with a PIN or password. The lockscreen will appear when triggered, requiring authentication to continue.
settings-security-enable-label = Enable Lockscreen
settings-security-enable-hint = Protect your dashboard with password authentication
settings-security-password-status-label = Password Status
settings-security-password-current = Current: { $type }
settings-security-password-none = No password set
settings-security-change = Change
settings-security-remove = Remove
settings-security-set-password = Set Password
settings-security-auto-lock-label = Auto-Lock After Inactivity
settings-security-auto-lock-hint = Automatically lock after period of no activity
settings-security-auto-lock-never = Never
settings-security-lock-blur-label = Lock When Window Loses Focus
settings-security-lock-blur-hint = Automatically lock when you switch to another application
settings-security-quick-actions-title = Quick Actions
settings-security-lock-now-label = Lock Dashboard Now
settings-security-lock-now-hint = Immediately lock the dashboard
settings-security-lock-now = Lock Now
settings-security-lock-not-ready = Cannot lock - lockscreen not ready
settings-security-setting-save-failed = Could not save security setting

## security_tab.js: two-factor authentication

settings-security-2fa-title = Two-Factor Authentication
settings-security-2fa-description = Add an extra layer of security using an authenticator app (Google Authenticator, Authy, etc.)
settings-security-2fa-status-label = 2FA Status
settings-security-2fa-status-enabled = Two-factor authentication is enabled
settings-security-2fa-status-none = Not configured
settings-security-2fa-disable = Disable 2FA
settings-security-2fa-enable = Enable 2FA

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = Close
settings-security-password-set-title = Set Password
settings-security-password-change-title = Change Password
settings-security-password-current-label = Current Password
settings-security-password-current-input =
    .placeholder = Enter current password
settings-security-password-type-label = Password Type
settings-security-password-new-label = New Password
settings-security-password-new-input =
    .placeholder = Enter new password
settings-security-password-confirm-label = Confirm Password
settings-security-password-confirm-input =
    .placeholder = Confirm password
settings-security-password-update = Update Password
settings-security-placeholder-pin4 = Enter 4-digit PIN
settings-security-placeholder-pin6 = Enter 6-digit PIN
settings-security-placeholder-text = Enter password
settings-security-password-required = Please enter a password
settings-security-password-mismatch = Passwords do not match
settings-security-pin4-invalid = PIN must be exactly 4 digits
settings-security-pin6-invalid = PIN must be exactly 6 digits
settings-security-text-too-short = Password must be at least 4 characters
settings-security-password-saved = Password saved
settings-security-password-save-failed = Failed to save password
settings-security-password-save-failed-detail = Failed to save password: { $message }

## security_tab.js: remove password dialog

settings-security-remove-title = Remove Password
settings-security-remove-description = Enter your current password to remove lockscreen protection.
settings-security-remove-confirm = Remove Password
settings-security-current-required = Please enter your current password
settings-security-password-removed = Password removed
settings-security-password-remove-failed = Failed to remove password
settings-security-password-remove-failed-detail = Failed to remove password: { $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = Enable Two-Factor Authentication
settings-security-2fa-password-prompt = Enter your password to continue:
settings-security-2fa-password-input =
    .placeholder = Enter password
settings-security-2fa-continue = Continue
settings-security-2fa-manual-code = Manual entry code:
settings-security-2fa-qr =
    .alt = TOTP QR Code
settings-security-2fa-code-prompt = Enter the 6-digit code from your authenticator app:
settings-security-2fa-verify-enable = Verify & Enable
settings-security-2fa-password-required = Please enter your password
settings-security-2fa-setup-failed = Failed to setup 2FA
settings-security-2fa-code-invalid-length = Please enter a 6-digit code
settings-security-2fa-code-invalid = Invalid code
settings-security-2fa-enabled = Two-factor authentication enabled
settings-security-2fa-verify-failed = Failed to verify code
settings-security-2fa-disable-title = Disable Two-Factor Authentication
settings-security-2fa-disable-prompt = Enter your password to disable 2FA:
settings-security-2fa-disable-failed = Failed to disable 2FA
settings-security-2fa-disabled = Two-factor authentication disabled

## agent_connections_tab.js

# Ids are the `ToolCategory` values of the agent tool registry.
settings-agent-category-analysis = Analysis
settings-agent-category-portfolio = Portfolio
settings-agent-category-trading = Trading
settings-agent-category-config = Configuration
settings-agent-category-system = System
settings-agent-category-analysis-description = Token analysis, market data and security checks.
settings-agent-category-portfolio-description = Open positions, balances and P&L.
settings-agent-category-trading-description = Buying, selling and closing positions with real funds.
settings-agent-category-config-description = Every bot setting, including RPC endpoints. Never wallet keys.
settings-agent-category-system-description = Status, events, and the emergency stop.
# The category as it reads inside a sentence.
settings-agent-category-analysis-inline = analysis
settings-agent-category-portfolio-inline = portfolio
settings-agent-category-trading-inline = trading
settings-agent-category-config-inline = configuration
settings-agent-category-system-inline = system

# Ids are the `PermissionLevel` values.
settings-agent-level-allow = Allow
settings-agent-level-ask-user = Ask
settings-agent-level-deny = Off
settings-agent-level-allow-hint = Runs immediately.
settings-agent-level-ask-user-hint = Waits for your approval in the app.
settings-agent-level-deny-hint = Refused, and hidden from the agent.

settings-agent-preset-full = Full access
settings-agent-preset-ask = Ask first
settings-agent-preset-read = Read only
settings-agent-preset-full-description = Everything runs without asking. Wallet keys stay unreachable.
settings-agent-preset-ask-description = Every action waits for your approval in the app.
settings-agent-preset-read-description = Analysis and portfolio reads. Nothing can be changed.
settings-agent-preset-custom = Custom
settings-agent-preset-group =
    .aria-label = Permission preset
settings-agent-permission-group = { $category } permission

# Summary of a limited connection. `$asking` and `$off` are category lists.
settings-agent-summary-asks-only = Limited — asks for { $asking }
settings-agent-summary-off-only = Limited — no { $off }
settings-agent-summary-asks-and-off = Limited — asks for { $asking }; no { $off }
settings-agent-list-separator = { ", " }

# Client kinds offered for setup.
settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = Generic stdio MCP

# Setup notes. Commands and paths are literal text, never placeables.
settings-agent-note-placeholder = Replace /absolute/path/to/screenerbot with the absolute path to your { -brand } binary — the running app could not represent its executable path on this system.
settings-agent-note-data-dir = If you run { -brand } with a non-default data directory, also set SCREENERBOT_DATA_DIR on the client (another -e / --env flag, or an env entry) to the same path.
settings-agent-note-codex-run = Run the command, or add the TOML block to ~/.codex/config.toml ($CODEX_HOME/config.toml). Restart { -codex } afterwards.
settings-agent-note-codex-get = `codex mcp get screenerbot` masks the secret in its output.
settings-agent-note-claude-code = { -claude } Code: run the command, then restart { -claude } Code. `claude mcp get screenerbot` will print the configured environment, including the secret.
settings-agent-note-claude-desktop = { -claude } Desktop: merge the JSON into claude_desktop_config.json under `mcpServers` and restart the app.
settings-agent-note-openclaw = Run the command, then use `openclaw mcp doctor screenerbot --probe` to verify that the saved stdio server starts and exposes tools.
settings-agent-note-hermes = Add this under `mcp_servers` in { -hermes }' configuration file, then restart { -hermes }.
settings-agent-note-generic = Any MCP client that speaks stdio: run this command with these args and environment, wherever the client keeps its server list.
settings-agent-block-codex-command = { -codex } CLI — terminal command
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (fallback)
settings-agent-block-claude-command = { -claude } Code — terminal command
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — terminal command
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = Generic stdio MCP client

settings-agent-name-required = Enter a name for this connection.
settings-agent-name-too-long = Name must be { $max } characters or fewer.
settings-agent-name-control-characters = Name must not contain control characters.

settings-agent-title = Agent Connections
settings-agent-description = Connect { -claude }, { -codex }, { -hermes }, { -openclaw }, or any stdio MCP client. { -brand } must remain running. Each connection carries its own permissions: full access by default, limited per connection whenever you want. No connection can ever read or change your wallet key.
settings-agent-name-label = Connection name
settings-agent-name-hint = Shown in the list below so you can tell connections apart.
settings-agent-name-input =
    .placeholder = Laptop coding agent
settings-agent-client-label = Client
settings-agent-client-hint = Picks the setup shown after the connection is created.
settings-agent-permissions-label = Permissions
settings-agent-permissions-hint = A new connection can do everything. Limit any category now, or later from the list below — wallet keys are never reachable either way.
settings-agent-create = Create connection
settings-agent-issued-group =
    .aria-label = New connection credential
settings-agent-issued-warning = Copy the secret now. It is shown once and cannot be retrieved again — revoke and recreate the connection if you lose it. { -brand } keeps only a one-way verifier; your MCP client stores the plaintext under its own configuration.
settings-agent-issued-client-id = Client ID
settings-agent-issued-secret = One-time secret
settings-agent-setup-for = Setup for
settings-agent-done = Done
settings-agent-list-title = Connections
settings-agent-loading = Loading connections...
settings-agent-active-count = { $count } active
settings-agent-empty = No connections yet. Create one above to pair a client.
settings-agent-empty-active = No active connections.
settings-agent-revoked-title = Revoked connections
settings-agent-created = Created { $time }
settings-agent-last-used = Last used { $time }
settings-agent-never-used = Never used
settings-agent-permissions-edit = Permissions
settings-agent-revoke = Revoke
settings-agent-permissions-save = Save permissions

settings-agent-load-failed = Failed to load Agent Connections
settings-agent-list-failed = Could not load connections
settings-agent-create-failed = Could not create the connection.
settings-agent-unreachable-create = Could not reach { -brand } to create the connection.
settings-agent-permissions-update-failed = Could not update the permissions
settings-agent-permissions-updated = Permissions updated
settings-agent-permissions-updated-detail = Applies to the connection's next request.
settings-agent-unreachable-save = Could not reach { -brand } to save
settings-agent-revoke-title = Revoke connection
settings-agent-revoke-message = Revoke "{ $label }"? The client stops working on its next request and cannot be restored.
settings-agent-revoke-fallback-name = this connection
settings-agent-revoke-failed = Could not revoke the connection
settings-agent-unreachable-revoke = Could not reach { -brand } to revoke

## telegram_tab.js

settings-telegram-loading = Loading Telegram settings...
settings-telegram-load-failed = Failed to load Telegram settings
settings-telegram-unknown = Unknown
settings-telegram-session-active = Active: { $duration }
settings-telegram-sessions-empty = No active sessions
settings-telegram-session-revoke = Revoke

settings-telegram-connection-title = Connection
settings-telegram-connection-description = Connect your { -telegram } bot to receive notifications and control { -brand } remotely.
settings-telegram-enable-label = Enable { -telegram }
settings-telegram-enable-hint = Enable { -telegram } bot integration
settings-telegram-token-label = Bot Token
settings-telegram-token-saved = Token saved
settings-telegram-token-help = Get this from @BotFather on { -telegram }
settings-telegram-token-input-saved =
    .placeholder = Token saved (enter new to change)
settings-telegram-token-input =
    .placeholder = Enter bot token
settings-telegram-token-toggle =
    .title = Show/Hide
settings-telegram-chat-label = Chat ID
settings-telegram-chat-connected = Connected to chat:
settings-telegram-chat-discover-hint = Discover your chat ID automatically
settings-telegram-chat-change =
    .title = Change
settings-telegram-chat-discover = Discover Chat ID
settings-telegram-discovery-step-add = Add your bot to a { -telegram } group, or start a direct chat with it
settings-telegram-discovery-step-privacy = For groups: Check @BotFather → /mybots → [your bot] → Bot Settings → Group Privacy
settings-telegram-discovery-privacy = <strong>Privacy Mode OFF:</strong> Bot receives all group messages<br/><strong>Privacy Mode ON:</strong> Bot only receives messages when @mentioned
settings-telegram-discovery-step-send = Send any message (or @mention your bot if Privacy Mode is ON)
settings-telegram-discovery-listening = Listening for messages...
settings-telegram-discovery-select = Select
settings-telegram-chat-id-label = ID:
settings-telegram-test-label = Test Connection
settings-telegram-test-hint = Send a test message to verify configuration
settings-telegram-test-send = Send Test
settings-telegram-test-sending = Sending...

# Ids are the chat kinds the Telegram poller reports.
settings-telegram-chat-type-private = private
settings-telegram-chat-type-group = group
settings-telegram-chat-type-supergroup = supergroup
settings-telegram-chat-type-channel = channel

settings-telegram-auth-title = Command Authentication
settings-telegram-auth-description = { -telegram } commands use the same 2FA as the dashboard lockscreen.
settings-telegram-auth-protected = Protected
settings-telegram-auth-disabled = Disabled
settings-telegram-auth-not-configured = Not Configured
settings-telegram-auth-error = Error
settings-telegram-auth-protected-note = Commands are protected by lockscreen 2FA. When sessions expire, users must provide their authenticator code via the <code>/login</code> command.
settings-telegram-auth-disabled-note = Lockscreen 2FA is configured but disabled for { -telegram }. Enable "Require 2FA for Commands" above to protect { -telegram } commands.
settings-telegram-auth-missing-note = Lockscreen 2FA is not configured. Without 2FA, expired sessions will auto-reactivate without verification.
settings-telegram-auth-managed-in = 2FA is managed in
settings-telegram-auth-configure-in = Configure 2FA in
settings-telegram-auth-configure-suffix = to require verification for { -telegram } commands.
settings-telegram-security-link = Security Settings
settings-telegram-timeout-title = Session Timeout
settings-telegram-timeout-description = How long an authenticated session stays active
settings-telegram-sessions-title = Active Sessions

settings-telegram-notifications-title = Notification Settings
settings-telegram-notifications-description = Choose which events trigger { -telegram } notifications.
settings-telegram-notify-opened-label = Position Opened
settings-telegram-notify-opened-hint = Notify when a new position is opened
settings-telegram-notify-closed-label = Position Closed
settings-telegram-notify-closed-hint = Notify when a position is closed
settings-telegram-notify-partial-label = Partial Exit
settings-telegram-notify-partial-hint = Notify on partial position exits
settings-telegram-notify-dca-label = DCA Executed
settings-telegram-notify-dca-hint = Notify when DCA orders are executed
settings-telegram-notify-errors-label = Errors
settings-telegram-notify-errors-hint = Notify on errors and failures
settings-telegram-notify-startup-label = Startup/Shutdown
settings-telegram-notify-startup-hint = Notify when bot starts or stops
settings-telegram-notify-filtering-label = Filtering Alerts
settings-telegram-notify-filtering-hint = Notify when new tokens pass filtering criteria
settings-telegram-notify-trades-label = Trade Alerts
settings-telegram-notify-trades-hint = Notify on significant trades for watched tokens
settings-telegram-notify-daily-label = Daily Summary
settings-telegram-notify-daily-hint = Receive daily trading activity and P&L summary

settings-telegram-features-title = Features
settings-telegram-features-description = Configure { -telegram } bot capabilities.
settings-telegram-commands-label = Enable Commands
settings-telegram-commands-hint = Allow controlling the bot via { -telegram } commands
settings-telegram-require-2fa-label = Require 2FA for Commands
settings-telegram-require-2fa-hint = When sessions expire, require 2FA code to reactivate. Uses lockscreen 2FA.
settings-telegram-inline-label = Inline Action Buttons
settings-telegram-inline-hint = Show action buttons in notification messages

settings-telegram-setting-save-failed = Could not save { -telegram } setting
settings-telegram-discovery-start-failed = Could not start discovery
settings-telegram-chat-selected = Chat selected
settings-telegram-chat-select-failed = Could not select chat
settings-telegram-test-sent = Test message sent
settings-telegram-test-failed = Test message failed
settings-telegram-session-revoked = Session revoked
settings-telegram-session-revoke-failed = Could not revoke session
