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
