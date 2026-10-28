// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The stepped task editor's steps: Wallet, Sizing (with a cost preview), Entry
// filters, Exits (presets and every rule field, inherited values shown) and a
// Review of what the task will run under. Pure markup, collection and checks.
import { escapeHtml } from "../../core/utils.js";
import { formatList } from "../../core/format.js";
import { renderAddress } from "../../ui/token_identity.js";
import { SOLANA_ADDRESS_RE, exitModeLabel, settingPct, settingSol, taskName } from "./format.js";
import { tradingModeLabel } from "../../ui/trading_mode.js";
import { renderNotice } from "../../ui/notice.js";
import { renderSegmented } from "../../ui/segmented.js";
import {
  PRESETS,
  RULES,
  effectivePolicy,
  exitWarnings,
  fieldText,
  inheritNote,
  matchPreset,
  validateOverrides,
} from "./policy.js";
import { rulesHtml } from "./rules.js";

export const STEPS = [
  {
    id: "wallet",
    get label() {
      return I18n.t("copy-step-wallet");
    },
  },
  {
    id: "sizing",
    get label() {
      return I18n.t("copy-step-sizing");
    },
  },
  {
    id: "entry",
    get label() {
      return I18n.t("copy-step-entry");
    },
  },
  {
    id: "exits",
    get label() {
      return I18n.t("copy-step-exits");
    },
  },
  {
    id: "review",
    get label() {
      return I18n.t("copy-step-review");
    },
  },
];

const NUMBER_FIELDS = [
  "max_native_per_trade",
  "max_native_per_token",
  "total_budget_native",
  "slippage_pct",
  "min_target_trade_native",
  "max_target_trade_native",
];

const EXIT_MODES = [
  {
    id: "buy_only",
    get label() {
      return exitModeLabel("buy_only");
    },
    get help() {
      return I18n.t("copy-editor-exit-help-buy-only");
    },
  },
  {
    id: "hybrid",
    get label() {
      return I18n.t("copy-editor-exit-both");
    },
    get help() {
      return I18n.t("copy-editor-exit-help-hybrid");
    },
  },
  {
    id: "mirror",
    get label() {
      return exitModeLabel("mirror");
    },
    get help() {
      return I18n.t("copy-editor-exit-help-mirror");
    },
  },
];

const number = (text) => (String(text ?? "").trim() === "" ? null : Number(text));
const valueAttr = (value, scale = 1) =>
  value === null || value === undefined || value === "" ? "" : String(Number(value) / scale);

function numberInput(
  esc,
  {
    attr,
    name,
    label,
    unit,
    value,
    placeholder = "",
    min,
    max,
    help = "",
    note = "",
    required = false,
    integer = false,
  }
) {
  return `<label class="copy-field"><span>${esc(label)}</span><span class="copy-number-row"><input type="number" ${attr}="${esc(name)}" value="${esc(value)}" placeholder="${esc(placeholder)}" step="${integer ? "1" : "any"}"${min != null ? ` min="${min}"` : ""}${max != null ? ` max="${max}"` : ""} inputmode="decimal"${required ? " required" : ""} /><span class="input-unit">${esc(unit)}</span></span>${help ? `<small>${esc(help)}</small>` : ""}${note ? `<small>${esc(note)}</small>` : ""}</label>`;
}

function toggleRow(esc, { name, title, help, checked }) {
  return `<label class="copy-switch-row"><span class="copy-field-text"><strong>${esc(title)}</strong><small>${esc(help)}</small></span><span class="toggle"><input type="checkbox" data-field="${name}"${checked ? " checked" : ""} /><span class="toggle-track"></span></span></label>`;
}

/** The tasks already copying the draft's wallet: another task copies the same trades again. */
export function duplicateNote({ draft, mode, source, tasks }) {
  const address = mode === "edit" ? null : draft.target_address;
  const others = address
    ? (tasks || []).filter((task) => task.target_address === address && task.id !== source?.id)
    : [];
  if (!others.length) return "";
  const tasksText = formatList(
    others.map((task) =>
      I18n.t("copy-task-ref", { name: taskName(task), mode: tradingModeLabel(task.mode) })
    )
  );
  return renderNotice({
    variant: "warning",
    message: I18n.t("copy-editor-duplicate", { tasks: tasksText }),
  });
}

/** While copy processing is paused for every task, a saved task waits: say so and offer Resume. */
function pausedNote({ processingPaused }, esc) {
  if (!processingPaused) return "";
  return `<div class="copy-paused-note" role="note"><p class="copy-note">${esc(I18n.t("copy-editor-processing-paused"))}</p><button class="btn btn-secondary btn-sm" type="button" data-copy-resume>${esc(I18n.t("copy-toolbar-resume"))}</button></div>`;
}

function walletStep(context, esc) {
  const { draft, mode, source } = context;
  const address =
    mode === "edit"
      ? `<div class="copy-field"><span>${esc(I18n.t("copy-editor-wallet"))}</span>${renderAddress(source.target_address, { explorer: "account" })}<small>${esc(I18n.t("copy-editor-wallet-identity"))}</small></div>`
      : `<label class="copy-field"><span>${esc(I18n.t("copy-editor-address-label"))}</span><input dir="ltr" type="text" data-field="target_address" value="${esc(draft.target_address || "")}" placeholder="${esc(I18n.t("copy-editor-address-placeholder"))}" spellcheck="false" autocomplete="off" required /><small>${esc(
          mode === "clone"
            ? I18n.t("copy-editor-address-help-clone")
            : I18n.t("copy-editor-address-help-create")
        )}</small></label><div id="copy-editor-duplicate">${duplicateNote(context)}</div>`;
  const note =
    mode === "edit" && source.mode === "live"
      ? I18n.t("copy-editor-note-live")
      : I18n.t("copy-editor-note-paper-quote");
  return `${address}
    <label class="copy-field"><span>${I18n.markup("copy-editor-name-label")}</span><input type="text" data-field="label" value="${esc(draft.label || "")}" maxlength="64" placeholder="${esc(I18n.t("copy-editor-name-placeholder"))}" /></label>
    ${toggleRow(esc, { name: "enabled", title: I18n.t("copy-editor-enabled-title"), help: I18n.t("copy-editor-enabled-help"), checked: draft.enabled })}
    ${pausedNote(context, esc)}
    <p class="copy-note">${esc(note)}</p>`;
}

/** What a copy costs under the draft's sizing, before network and priority fees. */
export function costPreview(draft) {
  const amount = draft.sizing.kind === "fixed" ? draft.sizing.sol : draft.sizing.pct;
  const cap = draft.max_native_per_trade;
  const perToken = draft.max_native_per_token;
  const budget = draft.total_budget_native;
  if (![amount, cap, perToken, budget].every((value) => Number.isFinite(value) && value > 0)) {
    return `<p>${escapeHtml(I18n.t("copy-editor-preview-empty"))}</p>`;
  }
  const copyFor = (target) =>
    draft.sizing.kind === "fixed" ? Math.min(amount, cap) : Math.min((target * amount) / 100, cap);
  const examples = [0.1, 1, 5]
    .map(
      (target) =>
        `<li>${I18n.markup("copy-editor-preview-example", { target: settingSol(target), copy: settingSol(copyFor(target)) })}</li>`
    )
    .join("");
  const unit = draft.sizing.kind === "fixed" ? Math.min(amount, cap) : cap;
  const perTokenCopies = Math.max(1, Math.floor(perToken / unit));
  const budgetCopies = Math.floor(budget / unit);
  const perTokenText = draft.buy_once_per_token
    ? I18n.t("copy-editor-preview-once", { size: settingSol(unit) })
    : I18n.t("copy-editor-preview-token-cap", { count: perTokenCopies, size: settingSol(unit) });
  const summary =
    draft.sizing.kind === "fixed"
      ? I18n.t("copy-editor-preview-summary-exact", { perToken: perTokenText, count: budgetCopies })
      : I18n.t("copy-editor-preview-summary-minimum", {
          perToken: perTokenText,
          count: budgetCopies,
        });
  return `<ul>${examples}</ul><p>${escapeHtml(summary)}</p>`;
}

function sizingStep({ draft, defaults }, esc) {
  const fixedKind = draft.sizing.kind === "fixed";
  const maxSlippage = defaults?.max_slippage_pct ?? null;
  const minSol = defaults?.min_trade_size_native ?? 0;
  const solUnit = I18n.t("copy-unit-native");
  const copySize = I18n.t("copy-editor-copy-size");
  return `<div class="copy-field"><span>${esc(copySize)}</span>${renderSegmented(
    {
      name: "sizing-kind",
      options: [
        { value: "fixed", label: I18n.t("copy-editor-sizing-fixed") },
        { value: "ratio_of_target", label: I18n.t("copy-editor-sizing-ratio") },
      ],
      value: draft.sizing.kind,
      ariaLabel: copySize,
    },
    esc
  )}</div>
    <div class="copy-fields">
      ${numberInput(esc, { attr: "data-field", name: "sizing_amount", label: fixedKind ? I18n.t("copy-editor-amount-fixed") : I18n.t("copy-editor-amount-ratio"), unit: fixedKind ? solUnit : "%", value: valueAttr(fixedKind ? draft.sizing.sol : draft.sizing.pct), min: fixedKind ? minSol : 0, required: true, help: fixedKind ? I18n.t("copy-editor-amount-help-fixed", { minimum: settingSol(minSol) }) : I18n.t("copy-editor-amount-help-ratio") })}
      ${numberInput(esc, { attr: "data-field", name: "max_native_per_trade", label: I18n.t("copy-field-per-trade-cap"), unit: solUnit, value: valueAttr(draft.max_native_per_trade), min: minSol, required: true, help: I18n.t("copy-editor-help-trade-cap") })}
      ${numberInput(esc, { attr: "data-field", name: "max_native_per_token", label: I18n.t("copy-field-per-token-cap"), unit: solUnit, value: valueAttr(draft.max_native_per_token), min: minSol, required: true, help: I18n.t("copy-editor-help-token-cap") })}
      ${numberInput(esc, { attr: "data-field", name: "total_budget_native", label: I18n.t("copy-field-total-budget"), unit: solUnit, value: valueAttr(draft.total_budget_native), min: minSol, required: true, help: I18n.t("copy-editor-help-budget") })}
      ${numberInput(esc, { attr: "data-field", name: "slippage_pct", label: I18n.t("copy-field-slippage"), unit: "%", value: valueAttr(draft.slippage_pct), min: defaults?.min_slippage_pct ?? 0, max: maxSlippage, required: true, placeholder: defaults ? String(defaults.default_slippage_pct) : "" })}
    </div>
    <section class="copy-preview" aria-live="polite"><h4>${esc(I18n.t("copy-editor-preview-title"))}</h4><div id="copy-editor-preview">${costPreview(draft)}</div></section>`;
}

function entryStep({ draft, defaults }, esc) {
  const global = Boolean(defaults?.require_filter_pass);
  const globalStable = Boolean(defaults?.copy_stable_quoted);
  const stableMode =
    draft.copy_stable_quoted == null ? "inherit" : draft.copy_stable_quoted ? "on" : "off";
  const stableLabel = I18n.t("copy-rules-stable-trades");
  const filterMode =
    draft.require_filter_pass == null ? "inherit" : draft.require_filter_pass ? "require" : "skip";
  const requires = draft.require_filter_pass ?? global;
  const solUnit = I18n.t("copy-unit-native");
  const filterLabel = I18n.t("copy-rules-filter-pass");
  return `<div class="copy-fields">
      ${numberInput(esc, { attr: "data-field", name: "min_target_trade_native", label: I18n.t("copy-editor-target-min"), unit: solUnit, value: valueAttr(draft.min_target_trade_native), min: 0, placeholder: I18n.t("copy-editor-any"), help: I18n.t("copy-editor-target-min-help") })}
      ${numberInput(esc, { attr: "data-field", name: "max_target_trade_native", label: I18n.t("copy-editor-target-max"), unit: solUnit, value: valueAttr(draft.max_target_trade_native), min: 0, placeholder: I18n.t("copy-editor-any"), help: I18n.t("copy-editor-target-max-help") })}
    </div>
    ${toggleRow(esc, { name: "buy_once_per_token", title: I18n.t("copy-editor-buy-once-title"), help: I18n.t("copy-editor-buy-once-help"), checked: draft.buy_once_per_token })}
    <div class="copy-field"><span>${esc(filterLabel)}</span>${renderSegmented(
      {
        name: "filter-mode",
        options: [
          {
            value: "inherit",
            label: global
              ? I18n.t("copy-filter-copy-setting-required")
              : I18n.t("copy-filter-copy-setting-not-required"),
          },
          { value: "require", label: I18n.t("copy-editor-filter-require") },
          { value: "skip", label: I18n.t("copy-editor-filter-skip") },
        ],
        value: filterMode,
        ariaLabel: filterLabel,
      },
      esc
    )}<small>${esc(I18n.t("copy-editor-filter-help"))}</small></div>
    ${requires ? renderNotice({ variant: "warning", message: I18n.t("copy-editor-filter-warning") }) : ""}
    <div class="copy-field"><span>${esc(stableLabel)}</span>${renderSegmented(
      {
        name: "stable-mode",
        options: [
          {
            value: "inherit",
            label: globalStable
              ? I18n.t("copy-stable-copy-setting-on")
              : I18n.t("copy-stable-copy-setting-off"),
          },
          { value: "on", label: I18n.t("copy-rule-on") },
          { value: "off", label: I18n.t("copy-rule-off") },
        ],
        value: stableMode,
        ariaLabel: stableLabel,
      },
      esc
    )}<small>${esc(I18n.t("copy-editor-stable-help"))}</small></div>`;
}

function ruleCard(rule, { draft, defaults }, esc) {
  const overrides = draft.exit_policy_overrides[rule.group];
  const inherited = defaults?.trader_defaults;
  const state = overrides.enabled === null ? "inherit" : overrides.enabled ? "on" : "off";
  // The inherited switch state is spelled out in the note under the choice, so the
  // segment never repeats "On" or "Off" beside the explicit ones.
  const seg = renderSegmented(
    {
      name: `rule-${rule.group}`,
      options: [
        { value: "inherit", label: I18n.t("copy-editor-rule-inherit") },
        { value: "on", label: I18n.t("copy-rule-on") },
        { value: "off", label: I18n.t("copy-rule-off") },
      ],
      value: state,
      ariaLabel: I18n.t("copy-editor-rule-aria", { rule: rule.title }),
    },
    esc
  );
  let body;
  if (state === "on") {
    body = `<div class="copy-fields">${rule.fields
      .filter((field) => field.key !== "enabled")
      .map((field) => {
        const name = `${rule.group}.${field.key}`;
        const fallback = inherited?.[rule.group]?.[field.key];
        if (field.bool) {
          const value = overrides[field.key];
          return `<label class="copy-field"><span>${esc(field.label)}</span><select data-custom-select data-rule-field="${name}"><option value=""${value === null ? " selected" : ""}>${esc(I18n.t("copy-editor-inherit-value", { value: fieldText(field, fallback) }))}</option><option value="true"${value === true ? " selected" : ""}>${esc(field.text(true))}</option><option value="false"${value === false ? " selected" : ""}>${esc(field.text(false))}</option></select></label>`;
        }
        return numberInput(esc, {
          attr: "data-rule-field",
          name,
          label: field.label,
          unit: field.unit,
          value: valueAttr(overrides[field.key], field.scale),
          placeholder: valueAttr(fallback, field.scale),
          integer: field.integer,
          help: I18n.t("copy-editor-rule-empty-uses", { value: fieldText(field, fallback) }),
          note: field.hint,
        });
      })
      .join("")}</div>`;
  } else if (state === "inherit") {
    body = `<p class="copy-note">${esc(inheritNote(rule, inherited, draft.exit_policy_overrides))}</p>`;
  } else {
    body = `<p class="copy-note">${esc(I18n.t("copy-editor-rule-off-note"))}</p>`;
  }
  return `<section class="copy-rule-card"><div class="copy-rule-head"><h4>${esc(rule.title)}</h4>${seg}</div>${body}</section>`;
}

export function exitWarningsHtml({ draft, defaults }) {
  const policy = effectivePolicy(defaults?.trader_defaults, draft.exit_policy_overrides);
  return exitWarnings(policy, draft.exit_mode, {
    slippagePct: draft.slippage_pct,
    feePct: defaults?.swap_fee_pct,
  })
    .map((message) => renderNotice({ variant: "warning", message }))
    .join("");
}

function exitsStep(context, esc) {
  const { draft } = context;
  const mode = EXIT_MODES.find((item) => item.id === draft.exit_mode) || EXIT_MODES[0];
  const preset = matchPreset(draft.exit_policy_overrides);
  const presets = [
    ...PRESETS.map(({ id, label }) => ({ value: id, label })),
    ...(preset === "custom" ? [{ value: "custom", label: I18n.t("copy-preset-custom") }] : []),
  ];
  const exitModes = EXIT_MODES.map(({ id, label }) => ({ value: id, label }));
  const whoSells = I18n.t("copy-editor-who-sells");
  const presetLabel = I18n.t("copy-editor-preset");
  const mirrorNote = `<p class="copy-note">${esc(I18n.t("copy-editor-mirror-note", { mine: exitModeLabel("buy_only"), both: I18n.t("copy-editor-exit-both") }))}</p>`;
  return `<div class="copy-field"><span>${esc(whoSells)}</span>${renderSegmented({ name: "exit-mode", options: exitModes, value: draft.exit_mode, ariaLabel: whoSells }, esc)}<small>${esc(mode.help)}</small></div>
    <div class="copy-field"><span>${esc(presetLabel)}</span>${renderSegmented({ name: "preset", options: presets, value: preset, ariaLabel: presetLabel }, esc)}<small>${esc(I18n.t("copy-editor-preset-help"))}</small></div>
    <div id="copy-editor-warnings">${exitWarningsHtml(context)}</div>
    ${draft.exit_mode === "mirror" ? mirrorNote : ""}
    <div class="copy-rule-cards${draft.exit_mode === "mirror" ? " is-inactive" : ""}">${RULES.map((rule) => ruleCard(rule, context, esc)).join("")}</div>`;
}

function reviewStep(context, esc) {
  const { draft, defaults, mode, source, processingPaused } = context;
  const global = Boolean(defaults?.require_filter_pass);
  const globalStable = Boolean(defaults?.copy_stable_quoted);
  const task = {
    ...draft,
    target_address: mode === "edit" ? source.target_address : draft.target_address,
  };
  const head = `<div class="copy-review-head">${renderAddress(task.target_address, { explorer: "account" })}<p>${esc(
    I18n.t("copy-editor-review-head", {
      name: draft.label || I18n.t("copy-task-unnamed"),
      mode: tradingModeLabel(mode === "edit" && source.mode === "live" ? "live" : "paper"),
      status: !draft.enabled
        ? I18n.t("copy-editor-review-paused")
        : processingPaused
          ? I18n.t("copy-editor-review-waits")
          : I18n.t("copy-editor-review-processes"),
    })
  )}</p></div>${draft.enabled ? pausedNote(context, esc) : ""}`;
  return (
    head +
    rulesHtml(
      {
        task,
        effective: effectivePolicy(defaults?.trader_defaults, draft.exit_policy_overrides),
        traderDefaults: defaults?.trader_defaults,
        managesExits: draft.exit_mode !== "mirror",
        requireFilter: draft.require_filter_pass ?? global,
        globalRequireFilter: global,
        copyStable: draft.copy_stable_quoted ?? globalStable,
        globalCopyStable: globalStable,
        feePct: defaults?.swap_fee_pct,
      },
      esc
    )
  );
}

export function stepHtml(id, context, esc) {
  switch (id) {
    case "wallet":
      return walletStep(context, esc);
    case "sizing":
      return sizingStep(context, esc);
    case "entry":
      return entryStep(context, esc);
    case "exits":
      return exitsStep(context, esc);
    default:
      return reviewStep(context, esc);
  }
}

/** Read the visible step's inputs into the draft. */
export function collect(body, draft) {
  body.querySelectorAll("[data-field]").forEach((input) => {
    const field = input.dataset.field;
    if (field === "target_address") draft.target_address = input.value.trim();
    else if (field === "label") draft.label = input.value.trim() || null;
    else if (field === "enabled" || field === "buy_once_per_token") draft[field] = input.checked;
    else if (field === "sizing_amount") {
      if (draft.sizing.kind === "fixed") draft.sizing.sol = number(input.value);
      else draft.sizing.pct = number(input.value);
    } else if (NUMBER_FIELDS.includes(field)) draft[field] = number(input.value);
  });
  body.querySelectorAll("[data-rule-field]").forEach((input) => {
    const [group, key] = input.dataset.ruleField.split(".");
    const spec = RULES.find((rule) => rule.group === group)?.fields.find(
      (field) => field.key === key
    );
    if (!spec) return;
    if (spec.bool) {
      draft.exit_policy_overrides[group][key] = input.value === "" ? null : input.value === "true";
    } else {
      const value = number(input.value);
      draft.exit_policy_overrides[group][key] =
        value === null ? null : spec.scale ? value * spec.scale : value;
    }
  });
}

const positive = (value) => Number.isFinite(value) && value > 0;

/**
 * The first problem with a step, in the server's own terms, as `{ message, field }`:
 * `field` names the input that holds the problem (its `data-field` or `data-rule-field`),
 * or is null when no single input does. Null when the step is valid.
 */
export function validate(id, draft, { mode, defaults }) {
  const problem = (message, field = null) => ({ message, field });
  if (id === "wallet") {
    if (mode !== "edit" && !SOLANA_ADDRESS_RE.test(draft.target_address || "")) {
      return problem(I18n.t("copy-editor-error-address"), "target_address");
    }
  } else if (id === "sizing") {
    const amount = draft.sizing.kind === "fixed" ? draft.sizing.sol : draft.sizing.pct;
    const required = [
      ["sizing_amount", amount],
      ["max_native_per_trade", draft.max_native_per_trade],
      ["max_native_per_token", draft.max_native_per_token],
      ["total_budget_native", draft.total_budget_native],
    ];
    const missing = required.find(([, value]) => !positive(value));
    if (missing) return problem(I18n.t("copy-editor-error-sizing"), missing[0]);
    const minSol = defaults?.min_trade_size_native ?? 0;
    if (draft.sizing.kind === "fixed" && draft.sizing.sol < minSol)
      return problem(
        I18n.t("copy-editor-error-min-copy", { minimum: settingSol(minSol) }),
        "sizing_amount"
      );
    if (draft.max_native_per_trade < minSol)
      return problem(
        I18n.t("copy-editor-error-min-cap", { minimum: settingSol(minSol) }),
        "max_native_per_trade"
      );
    if (draft.max_native_per_trade > draft.max_native_per_token)
      return problem(I18n.t("copy-editor-error-trade-cap"), "max_native_per_trade");
    if (draft.max_native_per_token > draft.total_budget_native)
      return problem(I18n.t("copy-editor-error-token-cap"), "max_native_per_token");
    const minSlippage = defaults?.min_slippage_pct ?? 0;
    const maxSlippage = defaults?.max_slippage_pct ?? Infinity;
    if (
      !positive(draft.slippage_pct) ||
      draft.slippage_pct < minSlippage ||
      draft.slippage_pct > maxSlippage
    ) {
      return problem(
        I18n.t("copy-editor-error-slippage", {
          min: settingPct(minSlippage),
          max: settingPct(maxSlippage),
        }),
        "slippage_pct"
      );
    }
  } else if (id === "entry") {
    const limits = [
      ["min_target_trade_native", draft.min_target_trade_native],
      ["max_target_trade_native", draft.max_target_trade_native],
    ];
    const invalid = limits.find(
      ([, value]) => value !== null && !(Number.isFinite(value) && value >= 0)
    );
    if (invalid) return problem(I18n.t("copy-editor-error-target-limits"), invalid[0]);
    const [[, min], [, max]] = limits;
    if (min !== null && max !== null && min > max)
      return problem(I18n.t("copy-editor-error-target-order"), "min_target_trade_native");
  } else if (id === "exits") {
    return validateOverrides(draft.exit_policy_overrides);
  }
  return null;
}
