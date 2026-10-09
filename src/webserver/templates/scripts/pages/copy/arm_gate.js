// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Arming live is its own step: the readiness evidence from the paper book, what
// real exposure the task carries, and explicit acknowledgements before the
// confirmation phrase is sent.
import {
  definitionRows,
  duration,
  exitModeLabel,
  modeLabel,
  pct,
  sol,
  solNumber,
  taskName,
} from "./format.js";
import { formatList } from "../../core/format.js";
import { readinessChecks } from "./overview.js";
import { RULES, ruleSummary } from "./policy.js";

export function createArmGate(page) {
  const { $, Utils, api, state, on, notify, paint, dialogs } = page;
  const esc = Utils.escapeHtml;
  let task = null;

  function setup() {
    on($("#copy-arm-body"), "change", sync);
    on($("#copy-arm-confirm"), "click", arm);
  }

  const runtimeBlocked = () =>
    task?.readiness?.checks?.some((check) => check.id === "runtime" && !check.passed) ?? true;

  function sync() {
    const acks = [...($("#copy-arm-body")?.querySelectorAll("[data-ack]") || [])];
    const pending = acks.filter((ack) => !ack.checked).length;
    const confirm = $("#copy-arm-confirm");
    if (confirm) confirm.disabled = runtimeBlocked() || pending > 0;
    // The acknowledgements sit below the fold; say why Arm live is still disabled.
    const hint = $("#copy-arm-hint");
    if (hint) {
      hint.textContent =
        !runtimeBlocked() && pending ? I18n.t("copy-arm-acks-left", { count: pending }) : "";
    }
  }

  function body(ws) {
    const size =
      ws.sizing?.kind === "ratio_of_target"
        ? I18n.t("copy-rules-size-ratio", { pct: pct(ws.sizing.pct, 1) })
        : sol(ws.sizing?.sol, 3);
    const stop = ws.effective_policy?.stop_loss;
    const stopNote =
      ws.policy_manages_exits && stop?.enabled && Number(stop.min_hold_seconds) > 0
        ? I18n.t("copy-arm-stop-note", { hold: duration(stop.min_hold_seconds) })
        : "";
    const exposure = definitionRows(
      [
        [I18n.t("copy-arm-per-copy"), size],
        [I18n.t("copy-field-per-trade-cap"), sol(ws.max_native_per_trade, 3)],
        [I18n.t("copy-field-per-token-cap"), sol(ws.max_native_per_token, 3)],
        [
          I18n.t("copy-arm-budget-left"),
          I18n.t("copy-arm-budget-left-value", {
            left: solNumber(ws.live_remaining_budget_native, 3),
            total: solNumber(ws.total_budget_native, 3),
          }),
          I18n.t("copy-arm-budget-left-note"),
        ],
        [I18n.t("copy-field-slippage"), pct(ws.slippage_pct, 1)],
        [I18n.t("copy-arm-exits"), exitModeLabel(ws.exit_mode)],
        [
          I18n.t("copy-exit-stop-loss"),
          ws.policy_manages_exits
            ? ruleSummary(RULES[0], ws.effective_policy)
            : I18n.t("copy-rules-wallet-sells-only"),
          stopNote,
        ],
      ],
      esc
    );
    const siblings = (state.overview?.tasks || []).filter(
      (other) => other.id !== ws.id && other.enabled && other.target_address === ws.target_address
    );
    const shared = siblings.length
      ? `<p class="copy-warning" role="note"><i class="icon-triangle-alert" aria-hidden="true"></i>${esc(
          I18n.t("copy-arm-shared", {
            tasks: formatList(
              siblings.map((other) =>
                I18n.t("copy-task-ref", { name: taskName(other), mode: modeLabel(other.mode) })
              )
            ),
          })
        )}</p>`
      : "";
    const ack = (text) =>
      `<label class="copy-ack"><input type="checkbox" data-ack /><span>${esc(text)}</span></label>`;
    const acks = runtimeBlocked()
      ? `<p class="copy-warning" role="alert"><i class="icon-triangle-alert" aria-hidden="true"></i>${esc(I18n.t("copy-arm-unavailable"))}</p>`
      : [
          ack(
            I18n.t("copy-arm-ack-real-native", {
              budget: solNumber(ws.live_remaining_budget_native, 3),
              trade: solNumber(ws.max_native_per_trade, 3),
            })
          ),
          ack(I18n.t("copy-arm-ack-fees")),
          ws.readiness?.ready ? "" : ack(I18n.t("copy-arm-ack-unready")),
        ].join("");
    return `<p class="copy-arm-lead">${esc(I18n.t("copy-arm-lead", { name: taskName(ws) }))}</p>
      <h4>${esc(I18n.t("copy-arm-readiness-title"))}</h4>${readinessChecks(ws, esc)}
      <h4>${esc(I18n.t("copy-arm-exposure-title"))}</h4><dl class="copy-defs">${exposure}</dl>${shared}
      <div class="copy-acks">${acks}</div>`;
  }

  function open(ws) {
    task = ws;
    const error = $("#copy-arm-error");
    if (error) error.textContent = "";
    paint($("#copy-arm-body"), body(ws));
    sync();
    dialogs.show("copy-arm");
  }

  async function arm(event) {
    if (!task) return;
    const button = event.currentTarget;
    const error = $("#copy-arm-error");
    button.disabled = true;
    try {
      if (!state.defaults) await page.reloadDefaults();
      const confirmation = state.defaults?.live_confirmation;
      if (!confirmation)
        throw Object.assign(new Error("missing"), {
          detail: I18n.t("copy-arm-confirmation-missing"),
        });
      await api.setMode(task.id, "live", confirmation);
      dialogs.hide("copy-arm");
      notify("warning", I18n.t("copy-arm-armed"), taskName(task));
      await page.reload();
    } catch (failure) {
      if (error) error.textContent = failure.detail || I18n.t("copy-arm-failed");
      sync();
    }
  }

  return { setup, open };
}
