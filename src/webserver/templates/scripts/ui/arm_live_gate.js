// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * The gate in front of arming paper work live, shared by copy tasks and strategies:
 * the readiness evidence from the paper book and the acknowledgements the user ticks.
 * Arm live stays disabled until the runtime check passes and every acknowledgement is
 * ticked; the request then carries the confirmation the API named, so the server
 * applies the same gate.
 */

/** Readiness checks as the API sends them: each title and detail is a `UiText`. */
export function readinessList(checks, esc) {
  const items = (checks || [])
    .map(
      (check) =>
        `<li class="arm-gate-check ${check.passed ? "is-passed" : "is-failed"}"><i class="${
          check.passed ? "icon-circle-check" : "icon-circle-x"
        }" aria-hidden="true"></i><span><strong>${esc(I18n.text(check.text))}</strong><small>${esc(
          I18n.text(check.detail)
        )}</small></span><span class="sr-only">${esc(
          check.passed ? I18n.t("common-check-passed") : I18n.t("common-check-not-passed")
        )}</span></li>`
    )
    .join("");
  return `<ul class="arm-gate-checks">${items}</ul>`;
}

/** True when live trading cannot run: the runtime check failed, or no readiness loaded. */
export const runtimeBlocked = (readiness) =>
  readiness?.checks?.some((check) => check.id === "runtime" && !check.passed) ?? true;

/** Why arming live is unavailable right now, in place of the acknowledgements. */
export function unavailableNote(text, esc) {
  return `<p class="arm-gate-unavailable" role="alert"><i class="icon-triangle-alert" aria-hidden="true"></i><span>${esc(
    text
  )}</span></p>`;
}

/** One ticked-off acknowledgement per text. */
export function ackList(texts, esc) {
  const acks = texts
    .filter(Boolean)
    .map(
      (text) =>
        `<label class="arm-gate-ack"><input type="checkbox" data-ack /><span>${esc(text)}</span></label>`
    )
    .join("");
  return `<div class="arm-gate-acks">${acks}</div>`;
}

/** The acknowledgements under `root` still unticked. */
export const pendingAcks = (root) =>
  [...(root?.querySelectorAll("[data-ack]") || [])].filter((ack) => !ack.checked).length;
