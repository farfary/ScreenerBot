// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for the status of a recorded tool call, in one place.
 *
 * Keys are the lowercased `ToolCallStatus` variants of
 * src/assistant/chat/types.rs, as the chat widget and the automation run
 * details both read them. `pending` is the fallback for a status a surface does
 * not know.
 */

/** Message key of each tool-call status. */
export const TOOL_CALL_STATUS_LABELS = Object.freeze({
  executed: "assistant-chat-tool-status-executed",
  failed: "assistant-chat-tool-status-failed",
  denied: "assistant-chat-tool-status-denied",
  pendingconfirmation: "assistant-chat-tool-status-pending-confirmation",
  pending: "assistant-chat-tool-status-pending",
});
