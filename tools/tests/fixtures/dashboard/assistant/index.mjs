// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the assistant dashboard page.

const tab = (id) => `#subTabsContainer [data-tab-id="${id}"]`;
// Dialogs built on demand are removed on close, which the suite reads as hidden.
const modal = (trigger, dialog) => ({ trigger, dialog, close: `${dialog} .modal-close` });

export const endpoints = [
  {
    method: "GET",
    path: "/api/assistant/chat/sessions",
    fixture: "chat_sessions.json",
    empty: "chat_sessions.empty.json",
    rust: "src/assistant/chat/database.rs::Vec<ChatSession>",
  },
  {
    method: "GET",
    path: "/api/assistant/chat/sessions/{id}",
    fixture: "chat_session.json",
    rust: "src/webserver/routes/assistant/types.rs::GetChatSessionResponse",
    record: "/api/assistant/chat/sessions/3",
  },
  {
    method: "GET",
    path: "/api/llm-analysis/status",
    fixture: "analysis_status.json",
    empty: "analysis_status.empty.json",
    rust: "src/webserver/routes/llm_analysis/types.rs::AnalysisStatusResponse",
  },
  {
    method: "GET",
    path: "/api/llm/providers",
    fixture: "llm_providers.json",
    rust: "src/webserver/routes/llm/types.rs::ProvidersListResponse",
  },
  {
    method: "GET",
    path: "/api/llm/config",
    fixture: "llm_config.json",
    rust: "src/webserver/routes/llm/types.rs::LlmConfigResponse",
  },
  {
    method: "GET",
    path: "/api/llm-analysis/instructions",
    fixture: "instructions.json",
    empty: "instructions.empty.json",
    rust: "src/webserver/routes/llm_analysis/types.rs::InstructionsListResponse",
  },
  {
    method: "GET",
    path: "/api/llm-analysis/instructions/{id}",
    fixture: "instruction.json",
    rust: "src/webserver/routes/llm_analysis/types.rs::InstructionResponse",
    record: "/api/llm-analysis/instructions/1",
  },
  {
    method: "GET",
    path: "/api/llm-analysis/templates",
    fixture: "templates.json",
    rust: "src/webserver/routes/llm_analysis/types.rs::TemplatesListResponse",
  },
  {
    // `list_automation_tasks` wraps the rows as `{ "tasks": [...] }`.
    method: "GET",
    path: "/api/assistant/automation",
    fixture: "automation_tasks.json",
    empty: "automation_tasks.empty.json",
    rust: "src/assistant/scheduled/types.rs::ScheduledTask",
  },
  {
    // `{ "runs": [...] }`.
    method: "GET",
    path: "/api/assistant/automation/runs",
    fixture: "automation_runs.json",
    empty: "automation_runs.empty.json",
    rust: "src/assistant/scheduled/types.rs::TaskRun",
  },
  {
    // `{ "run": ... }`.
    method: "GET",
    path: "/api/assistant/automation/runs/{id}",
    fixture: "automation_run.json",
    rust: "src/assistant/scheduled/types.rs::TaskRun",
    record: null,
  },
  {
    // `{ "stats": ... }`.
    method: "GET",
    path: "/api/assistant/automation/stats",
    fixture: "automation_stats.json",
    empty: "automation_stats.empty.json",
    rust: "src/assistant/scheduled/types.rs::AutomationStats",
  },
  {
    method: "GET",
    path: "/api/llm-analysis/history",
    fixture: "history.json",
    empty: "history.empty.json",
    rust: "src/webserver/routes/llm_analysis/types.rs::HistoryListResponse",
    record: "/api/llm-analysis/history?page=1&per_page=20",
  },
  {
    method: "GET",
    path: "/api/llm-analysis/config",
    fixture: "analysis_config.json",
    rust: "src/webserver/routes/llm_analysis/types.rs::AnalysisConfigResponse",
  },
  {
    method: "GET",
    path: "/api/llm-analysis/cache/stats",
    fixture: "analysis_reuse_stats.json",
    empty: "analysis_reuse_stats.empty.json",
    rust: "src/webserver/routes/llm_analysis/types.rs::CacheStatsResponse",
  },
  {
    method: "GET",
    path: "/api/tokens/identities",
    fixture: "token_identities.json",
    rust: "src/webserver/routes/tokens/identity.rs::IdentitiesResponse",
    record: null,
  },
];

export const views = [
  {
    name: "chat",
    click: [tab("chat")],
    populated: [
      { selector: "#chat-panel .session-item", min: 3 },
      { selector: "#chat-panel .cw-chat-messages .message", min: 4 },
    ],
    empty: [{ selector: "#chat-panel .sessions-empty", text: "No chat sessions yet" }],
  },
  {
    name: "overview",
    click: [tab("stats")],
    populated: [{ selector: "#recent-decisions-container .decision-card", min: 4 }],
    empty: [{ selector: "#recent-decisions-container .empty-state", text: "No recent decisions" }],
  },
  {
    name: "providers",
    click: [tab("providers")],
    populated: [{ selector: "#providers-list .provider-item", min: 9 }],
    dialogs: [
      modal(
        '#providers-list .provider-item[data-provider="anthropic"] .provider-btn',
        ".provider-config-modal"
      ),
    ],
  },
  {
    name: "instructions",
    click: [tab("instructions")],
    populated: [
      { selector: "#instructions-list .instruction-item", min: 4 },
      { selector: "#templates-list .template-card", min: 6 },
    ],
    empty: [{ selector: "#instructions-list .empty-state", text: "No custom instructions yet" }],
    dialogs: [
      modal("#new-instruction-btn", ".instruction-modal"),
      modal("#templates-list .template-card", ".instruction-modal"),
    ],
  },
  {
    name: "automation",
    click: [tab("automation")],
    populated: [
      { selector: "#automation-list .automation-task-item", min: 3 },
      { selector: "#automation-runs-list .automation-run-item", min: 3 },
    ],
    empty: [
      { selector: "#automation-list .empty-state", text: "No scheduled tasks yet" },
      { selector: "#automation-runs-list .automation-runs-empty", text: "No runs yet" },
    ],
    dialogs: [
      modal("#new-automation-btn", ".automation-modal"),
      modal("#automation-runs-list .automation-run-item", ".automation-modal"),
    ],
  },
  {
    name: "history",
    click: [tab("history")],
    populated: [{ selector: "#history-list .history-table tbody tr", min: 4 }],
    empty: [{ selector: "#history-list .empty-state", text: "No LLM-analysis requests yet" }],
  },
  {
    name: "testing",
    click: [tab("testing")],
    populated: [{ selector: "#testing-panel #evaluate-btn", min: 1 }],
  },
  {
    name: "settings",
    click: [tab("settings")],
    populated: [{ selector: "#setting-default-provider option", min: 10 }],
  },
];
