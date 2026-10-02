// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for a transaction's chain status, shared by the transaction list,
 * the details dialog and the position activity feed.
 */
import { escapeHtml } from "../core/utils.js";

// `TransactionStatus` (src/transactions/types.rs) variant names. `Success` and
// `Unknown` label a row whose status is missing and only its outcome is known.
export const TRANSACTION_STATUS_LABELS = Object.freeze({
  Pending: "transactions-status-pending",
  Confirmed: "transactions-status-confirmed",
  Finalized: "transactions-status-finalized",
  Failed: "transactions-status-failed",
  Success: "transactions-status-success",
  Unknown: "transactions-status-unknown",
});

const STATUS_STYLES = {
  Pending: { variant: "warning", icon: "icon-loader" },
  Confirmed: { variant: "success", icon: "icon-check" },
  Finalized: { variant: "success", icon: "icon-check-check" },
  Failed: { variant: "error", icon: "icon-x" },
  Success: { variant: "success", icon: "icon-check" },
};

export function statusLabel(status) {
  return I18n.label(TRANSACTION_STATUS_LABELS, status);
}

/** Badge with the status icon; `name` is a key of `STATUS_STYLES`. */
function iconBadge(name) {
  const { variant, icon } = STATUS_STYLES[name];
  return `<span class="badge ${variant}"><i class="${icon}"></i> ${escapeHtml(statusLabel(name))}</span>`;
}

/** Badge for a list row's status string, or `null` when it is not a known status. */
export function listStatusBadge(status) {
  return Object.hasOwn(STATUS_STYLES, status) && status !== "Success" ? iconBadge(status) : null;
}

/** Badge for a detail response: a status string, a `{ Failed }` object, or only the `success` flag. */
export function statusBadge(status, success) {
  const unknown = `<span class="badge secondary">${escapeHtml(statusLabel("Unknown"))}</span>`;
  if (!status) return unknown;
  if (typeof status === "string" && status !== "Failed") {
    const badge = listStatusBadge(status);
    if (badge) return badge;
  }
  if (status.Failed) return iconBadge("Failed");
  if (success === true) return iconBadge("Success");
  if (success === false) return iconBadge("Failed");
  return unknown;
}
