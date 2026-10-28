// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The one path that switches a saved strategy on or off, for the roster switch and the editor's On switch alike.

import { requestManager, errorMessageOf } from "../../core/request_manager.js";
import * as Utils from "../../core/utils.js";

/**
 * Switch the saved strategy `id` on (`enabled`) or off through the route that changes
 * nothing else of it. The moved switch confirms the change itself, so success raises no
 * toast; a refusal toasts the API's reason and resolves false for the caller to revert.
 *
 * @param {string} id
 * @param {boolean} enabled
 * @returns {Promise<boolean>} whether the strategy now has the requested state
 */
export async function setStrategyEnabled(id, enabled) {
  try {
    await requestManager.fetch(`/api/strategies/${encodeURIComponent(id)}/enabled`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ enabled }),
      priority: "high",
    });
    return true;
  } catch (error) {
    console.error("[Strategies] Failed to switch the strategy:", error);
    Utils.showToast({
      type: "error",
      key: `strategy-enabled-${id}`,
      title: I18n.t("strategies-toast-item-toggle-failed"),
      message: await errorMessageOf(error, ""),
    });
    return false;
  }
}
