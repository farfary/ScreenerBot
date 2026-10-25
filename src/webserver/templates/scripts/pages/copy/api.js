// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Copy Trading API client. Every failure carries the server's own explanation
// (`error.detail`) so the page can say why a request was refused instead of a
// generic "could not be saved".
import { apiErrorMessage } from "../../core/request_manager.js";

const BASE = "/api/copy-trading";

/** A manual paper close waits in the task's queue for two router quotes. */
export const CLOSE_TIMEOUT_MS = 1_800_000;

/** Attach the server's error detail to a request failure. */
export async function withDetail(error) {
  if (error?.detail) return error;
  let detail = null;
  try {
    const body = await error?.response?.json();
    // The localized operation message, followed by its technical cause when present.
    detail = apiErrorMessage(body, null);
  } catch {
    // The body was not JSON or is already read; the status line is all there is.
  }
  const wrapped = error instanceof Error ? error : new Error(String(error));
  wrapped.detail = detail || wrapped.message || I18n.t("copy-request-failed");
  return wrapped;
}

/**
 * Request options shared by every copy-trading write. A write is never replayed
 * after a lost connection: the server may have committed it before the response
 * was lost, and a replayed create opens a second task on the same wallet. The
 * page reloads its state after a failed write instead.
 */
const WRITE_OPTIONS = { priority: "high", skipDedup: true, retryOnConnectionError: false };

export function createApi(requestManager) {
  async function call(path, { method = "GET", body, signal, timeout } = {}) {
    const write = method !== "GET";
    try {
      return await requestManager.fetch(`${BASE}${path}`, {
        method,
        ...(body !== undefined
          ? { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }
          : {}),
        ...(write ? WRITE_OPTIONS : { priority: "normal", skipDedup: false }),
        ...(timeout !== undefined ? { timeout } : {}),
        signal,
      });
    } catch (error) {
      throw await withDetail(error);
    }
  }

  const query = (params) => {
    const search = new URLSearchParams();
    Object.entries(params || {}).forEach(([key, value]) => {
      if (value !== null && value !== undefined && value !== "") search.set(key, String(value));
    });
    const text = search.toString();
    return text ? `?${text}` : "";
  };

  return {
    overview: () => call("/overview"),
    defaults: () => call("/defaults"),
    workspace: (id) => call(`/tasks/${id}/workspace`),
    activity: (id, params) => call(`/tasks/${id}/activity${query(params)}`),
    insights: (id, range) => call(`/tasks/${id}/insights${query(range)}`),
    compare: (range) => call(`/insights${query(range)}`),
    profile: (address) => call(`/wallets/${encodeURIComponent(address)}`),
    create: (task) => call("/tasks", { method: "POST", body: task }),
    update: (id, patch) => call(`/tasks/${id}`, { method: "PATCH", body: patch }),
    remove: (id) => call(`/tasks/${id}`, { method: "DELETE" }),
    setMode: (id, mode, confirmation) =>
      call(`/tasks/${id}/mode`, {
        method: "POST",
        body: confirmation ? { mode, confirmation } : { mode },
      }),
    reset: (id) => call(`/tasks/${id}/reset`, { method: "POST" }),
    // A close whose response was lost is reconciled from the workspace, never sent again.
    // `openedAt` is the exact opened_at of the confirmed round, so a round that
    // reopened after confirmation is refused by the server.
    closeHolding: (id, mint, openedAt) =>
      call(`/tasks/${id}/holdings/${encodeURIComponent(mint)}/close`, {
        method: "POST",
        body: { opened_at: openedAt },
        timeout: CLOSE_TIMEOUT_MS,
      }),
    async config() {
      try {
        const response = await requestManager.fetch("/api/config/copy_trading");
        return response.copy_trading || response.data?.copy_trading || response.data || response;
      } catch (error) {
        throw await withDetail(error);
      }
    },
    async patchConfig(patch) {
      try {
        return await requestManager.fetch("/api/config/copy_trading", {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(patch),
          ...WRITE_OPTIONS,
        });
      } catch (error) {
        throw await withDetail(error);
      }
    },
  };
}
