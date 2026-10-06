// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The Rules tab and the editor's review: every field with the value that applies
// and where it comes from (a task override or the inherited Trader default).
import { definitionRows, exitModeLabel, fixed, modeLabel, pct, sol } from "./format.js";
import { RULES, exitWarnings, fieldText, isOverridden } from "./policy.js";

function sizeText(task) {
  if (task.sizing?.kind === "ratio_of_target") {
    return I18n.t("copy-rules-size-ratio", { pct: pct(task.sizing.pct, 1) });
  }
  return I18n.t("copy-rules-size-fixed", { amount: sol(task.sizing?.sol, 3) });
}

function targetRange(task) {
  const min = task.min_target_trade_native;
  const max = task.max_target_trade_native;
  if (min == null && max == null) return I18n.t("copy-rules-target-any");
  if (max == null) return I18n.t("copy-rules-target-min", { amount: sol(min, 3) });
  if (min == null) return I18n.t("copy-rules-target-max", { amount: sol(max, 3) });
  return I18n.t("copy-rules-target-between", { min: fixed(min, 3), max: fixed(max, 3) });
}

function warningList(warnings, esc) {
  return warnings
    .map(
      (text) =>
        `<p class="copy-warning" role="note"><i class="icon-triangle-alert" aria-hidden="true"></i>${esc(text)}</p>`
    )
    .join("");
}

function ruleTable({ task, effective, traderDefaults, managesExits }, esc) {
  const overrides = task.exit_policy_overrides || {};
  const rows = RULES.map((rule) => {
    const enabled = Boolean(effective?.[rule.group]?.enabled);
    const fields = rule.fields
      .map((field) => {
        const partialOff =
          field.key === "partial_exit_default_pct" && !effective?.[rule.group]?.allow_partial;
        const inactive = !managesExits || (field.key !== "enabled" && (!enabled || partialOff));
        const source = isOverridden(overrides, rule.group, field.key)
          ? I18n.t("copy-rules-source-override", {
              value: fieldText(field, traderDefaults?.[rule.group]?.[field.key]),
            })
          : I18n.t("copy-rules-source-default");
        // A rule that is on but never runs must not read as running.
        const applies =
          !managesExits && field.key === "enabled"
            ? I18n.t("copy-rules-not-used")
            : fieldText(field, effective?.[rule.group]?.[field.key]);
        return `<tr class="${inactive ? "is-inactive" : ""}"><td>${esc(field.label)}</td><td>${esc(applies)}</td><td class="copy-rules-source">${esc(source)}</td></tr>`;
      })
      .join("");
    return `<tr class="copy-rules-group"><th colspan="3" scope="rowgroup">${esc(rule.title)}</th></tr>${fields}`;
  }).join("");
  return `<div class="copy-table-wrap"><table class="copy-rules"><thead><tr><th scope="col">${esc(I18n.t("copy-rules-col-rule"))}</th><th scope="col">${esc(I18n.t("copy-rules-col-applies"))}</th><th scope="col">${esc(I18n.t("copy-rules-col-source"))}</th></tr></thead><tbody>${rows}</tbody></table></div>`;
}

/** Sizing, entry and exit sections for a task-shaped object. */
export function rulesHtml(context, esc) {
  const { task, effective, managesExits, requireFilter, globalRequireFilter, feePct } = context;
  const budgetNote = Number.isFinite(Number(task.spent_native))
    ? I18n.t("copy-rules-budget-note", {
        spent: fixed(task.spent_native, 3),
        mode: modeLabel(task.mode),
        remaining: fixed(task.remaining_budget_native, 3),
      })
    : "";
  const perToken = Number(task.max_native_per_token);
  const perTrade = Number(task.max_native_per_trade);
  const sizing = definitionRows(
    [
      [I18n.t("copy-rules-copy-size"), sizeText(task)],
      [I18n.t("copy-field-per-trade-cap"), sol(perTrade, 3)],
      [
        I18n.t("copy-field-per-token-cap"),
        sol(perToken, 3),
        perTrade > 0
          ? I18n.t("copy-rules-token-copies", {
              count: Math.max(1, Math.floor(perToken / perTrade)),
            })
          : "",
      ],
      [I18n.t("copy-field-total-budget"), sol(task.total_budget_native, 3), budgetNote],
      [I18n.t("copy-field-slippage"), pct(task.slippage_pct, 1)],
    ],
    esc
  );
  const entry = definitionRows(
    [
      [I18n.t("copy-rules-target-size"), targetRange(task)],
      [
        I18n.t("copy-rules-repeat-buys"),
        task.buy_once_per_token
          ? I18n.t("copy-rules-repeat-first-only")
          : I18n.t("copy-rules-repeat-every"),
      ],
      [
        I18n.t("copy-rules-filter-pass"),
        requireFilter
          ? I18n.t("copy-rules-filter-required")
          : I18n.t("copy-rules-filter-not-required"),
        task.require_filter_pass == null
          ? globalRequireFilter
            ? I18n.t("copy-filter-copy-setting-required")
            : I18n.t("copy-filter-copy-setting-not-required")
          : I18n.t("copy-rules-filter-task-override"),
      ],
    ],
    esc
  );
  const exitNote = managesExits
    ? ""
    : `<p class="copy-note">${esc(I18n.t("copy-rules-exits-inactive"))}</p>`;
  return `<div class="copy-rules-grid">
    <section class="copy-card"><h4>${esc(I18n.t("copy-rules-sizing"))}</h4><dl class="copy-defs">${sizing}</dl></section>
    <section class="copy-card"><h4>${esc(I18n.t("copy-rules-entry-filters"))}</h4><dl class="copy-defs">${entry}</dl></section>
  </div>
  <section class="copy-card copy-rules-exits">
    <h4>${esc(I18n.t("copy-rules-exits"))} <span class="copy-card-sub">${esc(exitModeLabel(task.exit_mode))}</span></h4>
    ${warningList(exitWarnings(effective, task.exit_mode, { slippagePct: task.slippage_pct, feePct }), esc)}${exitNote}
    ${ruleTable(context, esc)}
  </section>`;
}

export function renderRules({ ws, defaults }, esc) {
  return `<div class="copy-panel-head"><h3>${esc(I18n.t("copy-rules-title"))}</h3><button class="btn btn-secondary btn-sm" type="button" data-ws-action="edit" data-step="exits"><i class="icon-pencil" aria-hidden="true"></i> ${esc(I18n.t("copy-action-edit-rules"))}</button></div>${rulesHtml(
    {
      task: ws,
      effective: ws.effective_policy,
      traderDefaults: ws.trader_defaults,
      managesExits: ws.policy_manages_exits,
      requireFilter: ws.effective_require_filter_pass,
      globalRequireFilter: ws.global_require_filter_pass,
      feePct: defaults?.swap_fee_pct,
    },
    esc
  )}`;
}
