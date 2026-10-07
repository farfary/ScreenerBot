# Assistant page shell and chat widget. Model output (chat replies, tool
# results, decision reasoning) is data and is never part of this catalog.

# Provider names shown by the Assistant page. Ids are the `Provider::as_str`
# values in src/apis/llm/mod.rs.
assistant-provider-openai = { -openai }
assistant-provider-anthropic = { -anthropic }
assistant-provider-groq = { -groq }
assistant-provider-deepseek = { -deepseek }
assistant-provider-gemini = { -google-gemini }
assistant-provider-ollama = { -ollama }
assistant-provider-together = { -together-ai }
assistant-provider-openrouter = { -openrouter }
assistant-provider-mistral = { -mistral-ai }
assistant-provider-select = Select Provider...

# Verdicts recorded for a model evaluation. `allow` is the alias the overview
# feed may carry; the rest are the ids written by src/llm_analysis/engine.rs.
assistant-decision-allow = Allow
assistant-decision-pass = Pass
assistant-decision-reject = Reject
assistant-decision-buy = Buy
assistant-decision-sell = Sell
assistant-decision-hold = Hold

# Risk levels of `RiskLevel` in src/llm_analysis/types.rs.
assistant-risk-low = Low
assistant-risk-medium = Medium
assistant-risk-high = High
assistant-risk-critical = Critical

# assistant.html: page shell.
assistant-overview-title = Assistant Overview
assistant-overview-description = Live LLM and analysis status, performance metrics, and the latest model evaluations.
assistant-features-title = Model Features
assistant-metric-total-evaluations = Total Evaluations
assistant-metric-cache-hit-rate = Cache Hit Rate
assistant-metric-avg-latency = Avg Latency
assistant-metric-active-providers = Active Providers
assistant-decisions-title = Recent Decisions
assistant-decisions-subtitle = Latest model-analysis decisions

assistant-providers-title = LLM Providers
assistant-providers-description = Configure the shared LLM providers used by analysis and the Assistant

assistant-master-title = Master Settings
assistant-master-description = Shared LLM master switch and provider selection
assistant-setting-enabled-description = Enable intelligent token analysis using LLM providers
assistant-setting-default-provider-label = Default Provider
assistant-setting-default-provider-description = The one provider used by analysis, the Assistant, and scheduled tasks

assistant-filtering-title = Filtering
assistant-filtering-description = Assistant-powered token filtering before entry decisions
assistant-setting-filtering-enabled-label = Assistant Token Filtering
assistant-setting-filtering-enabled-description = Evaluate tokens with Assistant before allowing entry trades
assistant-setting-min-confidence-label = Minimum Confidence
assistant-setting-min-confidence-description = Tokens must score at least this confidence level to pass (0-100)
assistant-setting-fallback-pass-label = Fallback to Pass
assistant-setting-fallback-pass-description = Allow token entry if Assistant evaluation fails or times out

assistant-trading-title = Trading
assistant-trading-description = Assistant analysis during trading operations
assistant-setting-entry-analysis-label = Entry Analysis
assistant-setting-entry-analysis-description = Run Assistant analysis before opening positions
assistant-setting-exit-analysis-label = Exit Analysis
assistant-setting-exit-analysis-description = Run Assistant analysis before closing positions
assistant-setting-trailing-stop-label = Trailing Stop Analysis
assistant-setting-trailing-stop-description = Run Assistant analysis when trailing stops are triggered

assistant-blacklist-title = Auto Blacklist
assistant-blacklist-description = Automatically block tokens based on Assistant scores
assistant-setting-auto-blacklist-label = Auto Blacklist
assistant-setting-auto-blacklist-description = Automatically blacklist tokens with very low Assistant scores
assistant-setting-blacklist-threshold-label = Blacklist Threshold
assistant-setting-blacklist-threshold-description = Tokens scoring below this level will be auto-blacklisted (0-100)

assistant-performance-title = Performance
assistant-performance-description = Caching and concurrency settings
assistant-setting-cache-ttl-label = Cache TTL
assistant-setting-cache-ttl-description = How long to cache Assistant results (60-3600 seconds)
assistant-setting-max-evaluations-label = Max Evaluations
assistant-setting-max-evaluations-description = Maximum concurrent Assistant evaluations allowed (1-20)

assistant-cache-title = Cache Management
assistant-cache-description = View and clear cached Assistant evaluations
assistant-cache-size-label = Cache Size:
assistant-cache-memory-label = Memory Usage:
assistant-cache-clear = Clear Cache

assistant-testing-title = Assistant Testing Playground
assistant-testing-subtitle = Test Assistant analysis on any token
assistant-testing-mint-label = Mint Address
assistant-testing-mint-input =
    .placeholder = Enter Solana token mint address...
assistant-testing-priority-label = Priority
assistant-testing-priority-low = Low
assistant-testing-priority-medium = Medium
assistant-testing-priority-high = High
assistant-testing-evaluate = Evaluate
assistant-testing-results = Results

assistant-instructions-title = Custom Instructions
assistant-instructions-description = Add custom prompts that are injected into Assistant evaluations
assistant-instructions-new = New Instruction
assistant-instructions-category-all = All Categories
assistant-instructions-category-filtering = Filtering
assistant-instructions-category-trading = Trading
assistant-instructions-category-analysis = Analysis
assistant-instructions-category-general = General
assistant-instructions-status-all = All Status
assistant-instructions-status-active = Active Only
assistant-instructions-status-inactive = Inactive Only
assistant-instructions-search =
    .placeholder = Search instructions...
assistant-instructions-empty = No custom instructions yet
assistant-instructions-empty-add = Add Your First Instruction
assistant-templates-title = Instruction Templates
assistant-templates-description = Pre-built instruction templates you can add

assistant-automation-title = Automation
assistant-automation-description = Schedule Assistant tasks to run automatically on intervals or at specific times
assistant-automation-new =
    .aria-label = Create new automation task
assistant-automation-new-label = New Task
assistant-automation-stat-total = Total Tasks
assistant-automation-stat-active = Active
assistant-automation-stat-runs = Total Runs
assistant-automation-stat-success-rate = Success Rate
assistant-automation-empty = No scheduled tasks yet
assistant-automation-empty-subtitle = Create your first automated Assistant task to get started
assistant-automation-empty-add = Create Your First Task
    .aria-label = Create first automation task
assistant-automation-runs-title = Recent Runs

assistant-history-title = Decision History
assistant-history-subtitle = Recent Assistant evaluations

# assistant.js: page tabs, overview, settings and history.
assistant-tab-chat = Chat
assistant-tab-overview = Overview
assistant-tab-providers = Providers
assistant-tab-instructions = Instructions
assistant-tab-automation = Automation
assistant-tab-history = History
assistant-tab-testing = Testing
assistant-tab-settings = Settings
assistant-toggle-on = ON
assistant-toggle-off = OFF
assistant-status-active = Assistant Active
assistant-status-disabled = Assistant Disabled
assistant-status-load-failed = Could not load model-feature status
assistant-toggle-enabled-title = Assistant Enabled
assistant-toggle-enabled-message = Model-backed features are now active
assistant-toggle-disabled-title = Assistant Disabled
assistant-toggle-disabled-message = Model-backed features are disabled
assistant-toggle-failed = Failed to update model-feature status
assistant-decisions-empty = No recent decisions
assistant-decision-latency =
    .title = Latency
assistant-decision-confidence =
    .title = Confidence
assistant-config-load-failed = Could not load analysis configuration
assistant-cache-clear-message = Are you sure you want to clear the analysis cache? This will remove all cached model decisions.
assistant-cache-cleared-title = Cache Cleared
assistant-cache-cleared-message = The analysis cache is empty
assistant-cache-clear-failed = Failed to clear cache
assistant-config-saved-title = Saved
assistant-config-saved-message = Configuration saved successfully
assistant-config-save-failed = Failed to save configuration
assistant-history-load-failed = Failed to load history
assistant-history-empty = No LLM-analysis requests yet
assistant-history-column-token = Token
assistant-history-column-decision = Decision
assistant-history-column-confidence = Confidence
assistant-history-column-risk = Risk
assistant-history-column-reasoning = Reasoning
assistant-history-column-model = Model
assistant-history-column-latency = Latency
assistant-history-column-when = When
assistant-history-previous = Previous
assistant-history-page = Page { $page } of { $total }
assistant-history-cached = cached

# chat_widget.js: sessions sidebar and header.
assistant-chat-sessions-title = Sessions
assistant-chat-sidebar-new =
    .title = New Chat
    .aria-label = Create new chat session
assistant-chat-search =
    .placeholder = Search chats...
    .aria-label = Search chat sessions
assistant-chat-history-close =
    .aria-label = Close chat history
assistant-chat-history-open =
    .title = Chat history
    .aria-label = Open chat history
assistant-chat-header-new =
    .title = New Chat
    .aria-label = Start a new chat
assistant-chat-delete =
    .title = Delete
    .aria-label = Delete session
assistant-chat-close =
    .title = Close
    .aria-label = Close Assistant
assistant-chat-title-new = New Chat
assistant-chat-sessions-empty = No chat sessions yet
assistant-chat-sessions-empty-search = No matching chats
assistant-chat-sessions-new = New Chat
assistant-chat-group-today = Today
assistant-chat-group-yesterday = Yesterday
assistant-chat-group-week = Previous 7 Days
assistant-chat-group-older = Older

# chat_widget.js: empty state and quick prompts. The prompt texts are sent as the
# user's message.
assistant-chat-empty-kicker = Assistant
assistant-chat-empty-title = How can I help you today?
assistant-chat-empty-subtitle = Review your portfolio, investigate a token, or understand recent trading activity.
assistant-chat-prompt-positions-label = Review open positions
assistant-chat-prompt-positions-text = What's my current wallet balance and open positions?
assistant-chat-prompt-token-label = Analyze a token
assistant-chat-prompt-token-text = Analyze the security and risks of this token:
assistant-chat-prompt-activity-label = Explain recent activity
assistant-chat-prompt-activity-text = Explain my recent trading activity and any important outcomes

# chat_widget.js: input area.
assistant-chat-input =
    .placeholder = Message Assistant...
    .aria-label = Message input
assistant-chat-hint-key-enter = Enter
assistant-chat-hint-key-shift = Shift
assistant-chat-hint-send = to send
assistant-chat-hint-newline = for a new line
assistant-chat-send =
    .title = Send message
    .aria-label = Send message
assistant-chat-send-empty =
    .aria-label = Type a message to send
assistant-chat-stop =
    .title = Stop response (Esc)
    .aria-label = Stop response
assistant-chat-cancelled-title = Cancelled
assistant-chat-cancelled-message = Request cancelled
assistant-chat-message-too-long-title = Message too long
assistant-chat-message-too-long-message = Please shorten your message to under { $limit } characters
assistant-chat-start-failed = Failed to start chat session
assistant-chat-send-failed = Assistant could not complete the response. Your message is ready to retry.

# chat_widget.js: messages and tool calls.
assistant-chat-role-user = You
assistant-chat-role-assistant = Assistant
assistant-chat-message-actions =
    .aria-label = Message actions
assistant-chat-message-copy =
    .title = Copy
    .aria-label = Copy message
assistant-chat-message-regenerate =
    .title = Regenerate
    .aria-label = Regenerate response
assistant-chat-copied-message = Message
assistant-chat-copy-failed = Failed to copy message
assistant-chat-regenerate-none = No message to regenerate
assistant-chat-regenerate-reload = Reload the chat before regenerating this response
assistant-chat-regenerate-done-title = Regenerated
assistant-chat-regenerate-done-message = Response regenerated successfully
assistant-chat-regenerate-failed = Failed to regenerate
assistant-chat-regenerate-failed-toast = Failed to regenerate response

# Status of a tool call; ids are the `ToolCallStatus` variants in
# src/assistant/chat/types.rs, lowercased and hyphenated.
assistant-chat-tool-status-executed = Executed
assistant-chat-tool-status-failed = Failed
assistant-chat-tool-status-denied = Denied
assistant-chat-tool-status-pending-confirmation = Awaiting Confirmation
assistant-chat-tool-status-pending = Pending
assistant-chat-tool-section-input = Input:
assistant-chat-tool-section-output = Output:
assistant-chat-tool-section-error = Error:

# chat_widget.js: progress while a response is generated.
assistant-chat-typing =
    .aria-label = Assistant is thinking
assistant-chat-progress-preparing = Preparing response
assistant-chat-progress-planning = Planning response
assistant-chat-progress-reviewing = Reviewing tool results
assistant-chat-progress-using-tools = Using tools
assistant-chat-progress-running = Running
assistant-chat-progress-failed = Failed
assistant-chat-progress-complete = Complete
assistant-chat-error-unknown = Unknown error
assistant-chat-stream-http = API error: { $status }
assistant-chat-stream-unavailable = Assistant progress stream is unavailable
assistant-chat-stream-failed = Assistant request failed
assistant-chat-stream-incomplete = Assistant response ended before completion

# chat_widget.js: tool approval and sessions.
assistant-chat-tool-review = Review input
assistant-chat-tool-deny = Deny
assistant-chat-tool-allow = Allow
assistant-chat-tool-unknown = Unknown Tool
assistant-chat-tool-default-description = This tool requires your approval to execute.
assistant-chat-tool-executed = Tool executed
assistant-chat-tool-cancelled = Tool execution cancelled
assistant-chat-tool-confirm-failed = Failed to confirm tool execution
assistant-chat-sessions-load-failed = Could not load chat sessions
assistant-chat-delete-title = Delete Chat Session
assistant-chat-delete-message = Are you sure you want to delete this chat session? This action cannot be undone.
assistant-chat-delete-done = Chat session deleted
assistant-chat-delete-failed = Failed to delete chat session

# Agent tool labels. Ids are the tool names registered by
# `create_tool_registry` in src/agent_control/tools/mod.rs, with hyphens.
assistant-tool-analyze-token = Analyze Token
assistant-tool-get-market-data = Get Market Data
assistant-tool-check-security = Check Security
assistant-tool-get-positions = Get Positions
assistant-tool-get-position = Get Position
assistant-tool-get-balance = Get Balance
assistant-tool-get-pnl = Get Profit and Loss
assistant-tool-buy-token = Buy Token
assistant-tool-add-to-position = Add to Position
assistant-tool-sell-token = Sell Token
assistant-tool-close-position = Close Position
assistant-tool-get-config = Get Configuration
assistant-tool-describe-config = Describe Configuration
assistant-tool-update-config = Update Configuration
assistant-tool-get-status = Get Status
assistant-tool-get-events = Get Events
assistant-tool-force-stop = Force Stop
assistant-tool-clear-force-stop = Clear Force Stop
assistant-tool-get-trader-status = Get Trader Status
assistant-tool-get-trader-stats = Get Trader Statistics
assistant-tool-set-trader-enabled = Set Trader Enabled
assistant-tool-set-trader-monitor = Set Trader Monitor
assistant-tool-manage-loss-limit = Manage Loss Limit
assistant-tool-list-trader-templates = List Trader Templates
assistant-tool-apply-trader-template = Apply Trader Template
assistant-tool-get-copy-trading-overview = Get Copy Trading Overview
assistant-tool-get-copy-task = Get Copy Task
assistant-tool-get-copy-activity = Get Copy Activity
assistant-tool-create-copy-task = Create Copy Task
assistant-tool-update-copy-task = Update Copy Task
assistant-tool-delete-copy-task = Delete Copy Task
assistant-tool-set-copy-task-mode = Set Copy Task Mode
assistant-tool-get-copy-insights = Get Copy Insights
assistant-tool-get-copy-wallet-profile = Get Copy Wallet Profile
assistant-tool-clone-copy-task = Clone Copy Task
assistant-tool-reset-copy-paper-book = Reset Copy Paper Book
assistant-tool-close-copy-paper-holding = Close Copy Paper Holding

# Built-in instruction templates (src/llm_analysis/database.rs
# `get_builtin_templates`). The template body is model input and stays in Rust.
assistant-template-liquidity-guard-name = Liquidity Guard
assistant-template-liquidity-guard-description = Rejects tokens with thin liquidity, which raises slippage risk and makes exits difficult.
assistant-template-holder-distribution-name = Holder Distribution Check
assistant-template-holder-distribution-description = Flags tokens whose largest holders control a large share of the supply.
assistant-template-honeypot-detection-name = Honeypot Detection
assistant-template-honeypot-detection-description = Rejects tokens with active freeze or mint authority or unusual transfer restrictions.
assistant-template-momentum-filter-name = Momentum Filter
assistant-template-momentum-filter-description = Prefers tokens with positive price momentum confirmed by rising volume.
assistant-template-new-token-caution-name = New Token Caution
assistant-template-new-token-caution-description = Requires higher confidence for tokens that are less than a day old.
assistant-template-whale-activity-name = Whale Activity Monitor
assistant-template-whale-activity-description = Watches for large holder movements and unusual deployer or early-wallet activity.

# Template tags; ids are the `tags` of a built-in template.
assistant-template-tag-activity = Activity
assistant-template-tag-age = Age
assistant-template-tag-authority = Authority
assistant-template-tag-caution = Caution
assistant-template-tag-distribution = Distribution
assistant-template-tag-holders = Holders
assistant-template-tag-honeypot = Honeypot
assistant-template-tag-large-holders = Large Holders
assistant-template-tag-liquidity = Liquidity
assistant-template-tag-momentum = Momentum
assistant-template-tag-new-tokens = New Tokens
assistant-template-tag-price-action = Price Action
assistant-template-tag-risk-management = Risk Management
assistant-template-tag-rug-risk = Rug Risk
assistant-template-tag-safety = Safety
assistant-template-tag-security = Security
assistant-template-tag-volume = Volume
assistant-template-tag-whales = Whales

# Shared by the Assistant tab dialogs.
assistant-modal-close =
    .aria-label = Close

# providers_tab.js: provider list.
assistant-providers-load-failed = Could not load providers
assistant-providers-select-default =
    .title = Set as default
assistant-providers-use-default =
    .aria-label = Use { $name } as the default provider
assistant-providers-model-none = Not configured
assistant-providers-status-ready = Ready
assistant-providers-status-not-set-up = Not Set Up
assistant-providers-status-default = Default
assistant-providers-test = Test
assistant-providers-configure = Configure
assistant-providers-default-set-title = Default Provider Set
assistant-providers-default-set-message = { $name } is now the default provider
assistant-providers-default-set-failed = Failed to set default provider
assistant-providers-testing-title = Testing Provider
assistant-providers-testing-message = Testing { $name }...
assistant-providers-test-http = HTTP { $status }
assistant-providers-test-success-title = Connection Successful
assistant-providers-test-success-message = { $name } is working correctly
assistant-providers-test-failed = Test Failed

# providers_tab.js: configuration dialog.
assistant-providers-config-title = { $name } Configuration
assistant-providers-api-key = API Key
assistant-providers-key-saved = Key saved
assistant-providers-key-missing = No key set
assistant-providers-api-key-update =
    .placeholder = Enter new key to update...
assistant-providers-api-key-enter =
    .placeholder = Enter API key...
assistant-providers-key-toggle =
    .title = Show/Hide
assistant-providers-key-help-saved = Leave empty to keep current key, or enter a new key to update
assistant-providers-key-help-new = Your API key is stored securely and never shared
assistant-providers-model = Model
assistant-providers-model-input =
    .placeholder = e.g., gpt-4, claude-3-opus...
assistant-providers-model-help = The model to use for Assistant analysis requests
assistant-providers-enable = Enable this provider
assistant-providers-enable-help = When enabled, this provider will be available for Assistant analysis
assistant-providers-connection-test = Connection Test
assistant-providers-test-connection = Test Connection
assistant-providers-testing = Testing...
assistant-providers-save = Save Configuration
assistant-providers-saving = Saving...
assistant-providers-missing-key-title = Missing API Key
assistant-providers-missing-key-test = Please enter an API key first
assistant-providers-missing-key-enable = Please enter an API key to enable this provider
assistant-providers-missing-model-title = Missing Model
assistant-providers-missing-model-message = Please enter a model name
assistant-providers-test-save-failed = Failed to save config for testing
assistant-providers-test-connected = Connection successful!
assistant-providers-detail-model = Model:
assistant-providers-detail-none = N/A
assistant-providers-detail-latency = Latency:
assistant-providers-detail-tokens = Tokens:
assistant-providers-saved-title = Provider Saved
assistant-providers-saved-message = { $name } configuration saved
assistant-providers-save-failed = Failed to save provider configuration

# instructions_tab.js: list, templates and dialogs.
assistant-instructions-load-failed = Failed to load instructions
assistant-instructions-priority = Priority: { $position }
assistant-instructions-actions =
    .aria-label = Instruction actions
assistant-instructions-hint-filtering = Instructions for token filtering decisions - helps LLM analysis determine which tokens to skip
assistant-instructions-hint-trading = Instructions for entry/exit analysis - guides model-scored trading decisions
assistant-instructions-hint-analysis = General market-analysis guidelines for model-scored decisions
assistant-instructions-hint-general = Other instructions for model-backed behavior
assistant-instructions-char-count =
    { $count ->
        [one] { $amount } character
       *[other] { $amount } characters
    }
assistant-instructions-reordered-title = Reordered
assistant-instructions-reordered-message = Instructions reordered successfully
assistant-instructions-reorder-failed = Failed to reorder instructions
assistant-templates-empty = No templates available
assistant-templates-preview-title = Template Preview: { $name }
assistant-templates-preview-content = Content:
assistant-templates-customize-add = Customize & Add
assistant-templates-customize-title = Customize Template
assistant-instructions-field-name = Name
assistant-instructions-field-category = Category
assistant-instructions-field-content = Content
assistant-instructions-name-input =
    .placeholder = e.g., Liquidity Guard
assistant-instructions-content-input =
    .placeholder = Enter your instruction...
assistant-instructions-create = Create
assistant-instructions-create-title = Create Instruction
assistant-instructions-missing-title = Missing Fields
assistant-instructions-missing-message = Name and content are required
assistant-instructions-created-title = Created
assistant-instructions-created-message = Instruction created successfully
assistant-instructions-created-from-template = Instruction created from template: { $name }
assistant-instructions-create-failed = Failed to create instruction
assistant-instructions-create-from-template-failed = Failed to create instruction from template
assistant-instructions-toggle-failed = Failed to toggle instruction
assistant-instructions-edit-title = Edit Instruction
assistant-instructions-preview = Preview
assistant-instructions-save-changes = Save Changes
assistant-instructions-untitled = Untitled
assistant-instructions-load-item-failed = Failed to load instruction data
assistant-instructions-updated-title = Updated
assistant-instructions-updated-message = Instruction updated successfully
assistant-instructions-update-failed = Failed to update instruction
assistant-instructions-delete-title = Delete Instruction
assistant-instructions-delete-message = Are you sure you want to delete this instruction?
assistant-instructions-deleted-title = Deleted
assistant-instructions-deleted-message = Instruction deleted successfully
assistant-instructions-delete-failed = Failed to delete instruction
assistant-instructions-copy-name = { $name } (Copy)
assistant-instructions-duplicated-title = Duplicated
assistant-instructions-duplicated-message = Instruction duplicated successfully
assistant-instructions-duplicate-failed = Failed to duplicate instruction

# automation_tab.js: task list, runs and dialogs. Schedule type, permission and
# run status ids are the `as_str` values of `ScheduleType`, `TaskToolPermissions`
# and `RunStatus` in src/assistant/scheduled/types.rs.
assistant-automation-schedule-type-interval = Interval
assistant-automation-schedule-type-daily = Daily
assistant-automation-schedule-type-weekly = Weekly
assistant-automation-permission-read-only = Read Only
assistant-automation-permission-full = Full Access
assistant-automation-permission-option-read-only = Read Only (safe)
assistant-automation-permission-option-full = Full Access (can trade)
assistant-automation-run-status-running = Running
assistant-automation-run-status-success = Success
assistant-automation-run-status-failed = Failed
assistant-automation-run-status-timeout = Timed Out
assistant-automation-run-status-skipped = Skipped
assistant-automation-hint-interval = Interval in seconds (e.g., 300 = every 5 minutes)
assistant-automation-hint-daily = Time in HH:MM UTC (e.g., 14:00)
assistant-automation-hint-weekly = Days and time: mon,wed,fri:09:00
assistant-automation-schedule-every = Every { $span }
assistant-automation-schedule-daily = Daily at { $time } UTC
assistant-automation-schedule-weekly = { $days } at { $time } UTC
assistant-automation-schedule-day-separator = { ", " }
assistant-automation-task-active = Active
assistant-automation-task-paused = Paused
assistant-automation-never = Never
assistant-automation-last-run = Last: { $when }
assistant-automation-next-run = Next: { $when }
assistant-automation-run-now =
    .title = Run Now
    .aria-label = Run Now
assistant-automation-actions =
    .aria-label = Automation actions
assistant-automation-runs-count =
    { $count ->
        [one] { $amount } run
       *[other] { $amount } runs
    }
assistant-automation-runs-empty = No runs yet
assistant-automation-task-runs-empty = No runs yet for this task
assistant-automation-task-fallback = Task #{ $id }
assistant-automation-task-generic = Task
assistant-automation-create-title = Create Automation Task
assistant-automation-edit-title = Edit Task
assistant-automation-field-name = Task Name
assistant-automation-name-input =
    .placeholder = e.g., Portfolio Monitor
assistant-automation-field-instruction = Instruction
assistant-automation-instruction-input =
    .placeholder = What should the Assistant do? e.g., Check open positions for reversal signs and report findings.
assistant-automation-field-schedule-type = Schedule Type
assistant-automation-field-schedule-value = Schedule Value
assistant-automation-field-permissions = Tool Permissions
assistant-automation-field-timeout = Timeout (seconds)
assistant-automation-notify-telegram = Notify via { -telegram }
assistant-automation-notify-success = Notify on success
assistant-automation-notify-failure = Notify on failure
assistant-automation-create-task = Create Task
assistant-automation-save-changes = Save Changes
assistant-automation-validation-title = Validation
assistant-automation-validation-required = Please fill in all required fields
assistant-automation-validation-interval = Interval must be at least 60 seconds
assistant-automation-validation-daily = Daily schedule must be in HH:MM format
assistant-automation-validation-weekly = Weekly schedule must be in format: mon,wed,fri:09:00
assistant-automation-created = Task created
assistant-automation-create-failed = Failed to create task
assistant-automation-updated = Task updated
assistant-automation-update-failed = Failed to update task
assistant-automation-toggle-failed = Failed to toggle task
assistant-automation-triggered = Task triggered
assistant-automation-trigger-failed = Failed to trigger task
assistant-automation-delete-title = Delete Task
assistant-automation-delete-message = Are you sure you want to delete this automation task? This action cannot be undone.
assistant-automation-deleted = Task deleted
assistant-automation-delete-failed = Failed to delete task
assistant-automation-view-runs = View Runs
assistant-automation-runs-load-failed = Failed to load runs
assistant-automation-runs-history-title = Run History — { $task }
assistant-automation-run-load-failed = Failed to load run details
assistant-automation-run-details-title = Run Details
assistant-automation-run-task = Task
assistant-automation-run-status = Status
assistant-automation-run-started = Started
assistant-automation-run-duration = Duration
assistant-automation-run-provider = Provider
assistant-automation-run-tokens = Tokens
assistant-automation-run-tools-title = Tool Calls ({ $amount })
assistant-automation-run-response = Assistant Response
