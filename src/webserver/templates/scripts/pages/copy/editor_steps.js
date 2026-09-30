// The stepped task editor's steps: Wallet, Sizing (with a cost preview), Entry
// filters, Exits (presets and every rule field, inherited values shown) and a
// Review of what the task will run under. Pure markup, collection and checks.
import { escapeHtml } from "../../core/utils.js";
import { renderAddress } from "../../ui/token_identity.js";
import {
  SOLANA_ADDRESS_RE,
  exitModeLabel,
  modeLabel,
  pct,
  segmented,
  sol,
  taskName,
} from "./format.js";
import {
  PRESETS,
  RULES,
  effectivePolicy,
  exitWarnings,
  fieldText,
  matchPreset,
  ruleSummary,
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
  "max_sol_per_trade",
  "max_sol_per_token",
  "total_budget_sol",
  "slippage_pct",
  "min_target_trade_sol",
  "max_target_trade_sol",
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
    required = false,
    integer = false,
  }
) {
  return `<label class="copy-field"><span>${esc(label)}</span><span class="copy-number-row"><input type="number" ${attr}="${esc(name)}" value="${esc(value)}" placeholder="${esc(placeholder)}" step="${integer ? "1" : "any"}"${min != null ? ` min="${min}"` : ""}${max != null ? ` max="${max}"` : ""} inputmode="decimal"${required ? " required" : ""} /><span class="input-unit">${esc(unit)}</span></span>${help ? `<small>${esc(help)}</small>` : ""}</label>`;
}

function toggleRow(esc, { name, title, help, checked }) {
  return `<label class="copy-switch-row"><span class="copy-field-text"><strong>${esc(title)}</strong><small>${esc(help)}</small></span><span class="toggle"><input type="checkbox" data-field="${name}"${checked ? " checked" : ""} /><span class="toggle-track"></span></span></label>`;
}

/** The tasks already copying the draft's wallet: another task copies the same trades again. */
export function duplicateNote({ draft, mode, source, tasks }, esc) {
  const address = mode === "edit" ? null : draft.target_address;
  const others = address
    ? (tasks || []).filter((task) => task.target_address === address && task.id !== source?.id)
    : [];
  if (!others.length) return "";
  const tasksText = others
    .map((task) => I18n.t("copy-task-ref", { name: taskName(task), mode: modeLabel(task.mode) }))
    .join(", ");
  return `<p class="copy-warning" role="note"><i class="icon-triangle-alert" aria-hidden="true"></i>${esc(I18n.t("copy-editor-duplicate", { tasks: tasksText }))}</p>`;
}

function walletStep(context, esc) {
  const { draft, mode, source } = context;
  const address =
    mode === "edit"
      ? `<div class="copy-field"><span>${esc(I18n.t("copy-editor-wallet"))}</span>${renderAddress(source.target_address, { explorer: "account" })}<small>${esc(I18n.t("copy-editor-wallet-identity"))}</small></div>`
      : `<label class="copy-field"><span>${esc(I18n.t("copy-editor-address-label"))}</span><input type="text" data-field="target_address" value="${esc(draft.target_address || "")}" placeholder="${esc(I18n.t("copy-editor-address-placeholder"))}" spellcheck="false" autocomplete="off" required /><small>${esc(
          mode === "clone"
            ? I18n.t("copy-editor-address-help-clone")
            : I18n.t("copy-editor-address-help-create")
        )}</small></label><div id="copy-editor-duplicate">${duplicateNote(context, esc)}</div>`;
  const note =
    mode === "edit" && source.mode === "live"
      ? I18n.t("copy-editor-note-live")
      : I18n.t("copy-editor-note-paper");
  return `${address}
    <label class="copy-field"><span>${I18n.markup("copy-editor-name-label")}</span><input type="text" data-field="label" value="${esc(draft.label || "")}" maxlength="64" placeholder="${esc(I18n.t("copy-editor-name-placeholder"))}" /></label>
    ${toggleRow(esc, { name: "enabled", title: I18n.t("copy-editor-enabled-title"), help: I18n.t("copy-editor-enabled-help"), checked: draft.enabled })}
    <p class="copy-note">${esc(note)}</p>`;
}

/** What a copy costs under the draft's sizing, before network and priority fees. */
export function costPreview(draft) {
  const amount = draft.sizing.kind === "fixed" ? draft.sizing.sol : draft.sizing.pct;
  const cap = draft.max_sol_per_trade;
  const perToken = draft.max_sol_per_token;
  const budget = draft.total_budget_sol;
  if (![amount, cap, perToken, budget].every((value) => Number.isFinite(value) && value > 0)) {
    return `<p>${escapeHtml(I18n.t("copy-editor-preview-empty"))}</p>`;
  }
  const copyFor = (target) =>
    draft.sizing.kind === "fixed" ? Math.min(amount, cap) : Math.min((target * amount) / 100, cap);
  const examples = [0.1, 1, 5]
    .map(
      (target) =>
        `<li>${I18n.markup("copy-editor-preview-example", { target: sol(target, 1), copy: sol(copyFor(target), 3) })}</li>`
    )
    .join("");
  const unit = draft.sizing.kind === "fixed" ? Math.min(amount, cap) : cap;
  const perTokenCopies = Math.max(1, Math.floor(perToken / unit));
  const budgetCopies = Math.floor(budget / unit);
  const perTokenText = draft.buy_once_per_token
    ? I18n.t("copy-editor-preview-once", { size: sol(unit, 3) })
    : I18n.t("copy-editor-preview-token-cap", { count: perTokenCopies, size: sol(unit, 3) });
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
  const minSol = defaults?.min_trade_size_sol ?? 0;
  const solUnit = I18n.t("copy-unit-sol");
  const copySize = I18n.t("copy-editor-copy-size");
  return `<div class="copy-field"><span>${esc(copySize)}</span>${segmented(
    "sizing-kind",
    [
      { id: "fixed", label: I18n.t("copy-editor-sizing-fixed") },
      { id: "ratio_of_target", label: I18n.t("copy-editor-sizing-ratio") },
    ],
    draft.sizing.kind,
    esc,
    copySize
  )}</div>
    <div class="copy-fields">
      ${numberInput(esc, { attr: "data-field", name: "sizing_amount", label: fixedKind ? I18n.t("copy-editor-amount-fixed") : I18n.t("copy-editor-amount-ratio"), unit: fixedKind ? solUnit : "%", value: valueAttr(fixedKind ? draft.sizing.sol : draft.sizing.pct), min: fixedKind ? minSol : 0, required: true, help: fixedKind ? I18n.t("copy-editor-amount-help-fixed", { minimum: sol(minSol, 3) }) : I18n.t("copy-editor-amount-help-ratio") })}
      ${numberInput(esc, { attr: "data-field", name: "max_sol_per_trade", label: I18n.t("copy-field-per-trade-cap"), unit: solUnit, value: valueAttr(draft.max_sol_per_trade), min: minSol, required: true, help: I18n.t("copy-editor-help-trade-cap") })}
      ${numberInput(esc, { attr: "data-field", name: "max_sol_per_token", label: I18n.t("copy-field-per-token-cap"), unit: solUnit, value: valueAttr(draft.max_sol_per_token), min: minSol, required: true, help: I18n.t("copy-editor-help-token-cap") })}
      ${numberInput(esc, { attr: "data-field", name: "total_budget_sol", label: I18n.t("copy-field-total-budget"), unit: solUnit, value: valueAttr(draft.total_budget_sol), min: minSol, required: true, help: I18n.t("copy-editor-help-budget") })}
      ${numberInput(esc, { attr: "data-field", name: "slippage_pct", label: I18n.t("copy-field-slippage"), unit: "%", value: valueAttr(draft.slippage_pct), min: defaults?.min_slippage_pct ?? 0, max: maxSlippage, required: true, placeholder: defaults ? String(defaults.default_slippage_pct) : "" })}
    </div>
    <section class="copy-preview" aria-live="polite"><h4>${esc(I18n.t("copy-editor-preview-title"))}</h4><div id="copy-editor-preview">${costPreview(draft)}</div></section>`;
}

function entryStep({ draft, defaults }, esc) {
  const global = Boolean(defaults?.require_filter_pass);
  const filterMode =
    draft.require_filter_pass == null ? "inherit" : draft.require_filter_pass ? "require" : "skip";
  const requires = draft.require_filter_pass ?? global;
  const solUnit = I18n.t("copy-unit-sol");
  const filterLabel = I18n.t("copy-rules-filter-pass");
  return `<div class="copy-fields">
      ${numberInput(esc, { attr: "data-field", name: "min_target_trade_sol", label: I18n.t("copy-editor-target-min"), unit: solUnit, value: valueAttr(draft.min_target_trade_sol), min: 0, placeholder: I18n.t("copy-editor-any"), help: I18n.t("copy-editor-target-min-help") })}
      ${numberInput(esc, { attr: "data-field", name: "max_target_trade_sol", label: I18n.t("copy-editor-target-max"), unit: solUnit, value: valueAttr(draft.max_target_trade_sol), min: 0, placeholder: I18n.t("copy-editor-any"), help: I18n.t("copy-editor-target-max-help") })}
    </div>
    ${toggleRow(esc, { name: "buy_once_per_token", title: I18n.t("copy-editor-buy-once-title"), help: I18n.t("copy-editor-buy-once-help"), checked: draft.buy_once_per_token })}
    <div class="copy-field"><span>${esc(filterLabel)}</span>${segmented(
      "filter-mode",
      [
        {
          id: "inherit",
          label: global
            ? I18n.t("copy-filter-copy-setting-required")
            : I18n.t("copy-filter-copy-setting-not-required"),
        },
        { id: "require", label: I18n.t("copy-editor-filter-require") },
        { id: "skip", label: I18n.t("copy-editor-filter-skip") },
      ],
      filterMode,
      esc,
      filterLabel
    )}<small>${esc(I18n.t("copy-editor-filter-help"))}</small></div>
    ${requires ? `<p class="copy-warning" role="note"><i class="icon-triangle-alert" aria-hidden="true"></i>${esc(I18n.t("copy-editor-filter-warning"))}</p>` : ""}`;
}

function ruleCard(rule, { draft, defaults }, esc) {
  const overrides = draft.exit_policy_overrides[rule.group];
  const inherited = defaults?.trader_defaults;
  const state = overrides.enabled === null ? "inherit" : overrides.enabled ? "on" : "off";
  const inheritLabel = inherited
    ? I18n.t("copy-editor-inherit-value", {
        value: inherited[rule.group]?.enabled ? I18n.t("copy-rule-on") : I18n.t("copy-rule-off"),
      })
    : I18n.t("copy-editor-rule-inherit");
  const seg = segmented(
    `rule-${rule.group}`,
    [
      { id: "inherit", label: inheritLabel },
      { id: "on", label: I18n.t("copy-rule-on") },
      { id: "off", label: I18n.t("copy-rule-off") },
    ],
    state,
    esc,
    I18n.t("copy-editor-rule-aria", { rule: rule.title })
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
        });
      })
      .join("")}</div>`;
  } else if (state === "inherit") {
    body = `<p class="copy-note">${esc(inherited ? I18n.t("copy-editor-rule-follows", { summary: ruleSummary(rule, inherited) }) : I18n.t("copy-editor-rule-follows-plain"))}</p>`;
  } else {
    body = `<p class="copy-note">${esc(I18n.t("copy-editor-rule-off-note"))}</p>`;
  }
  return `<section class="copy-rule-card"><div class="copy-rule-head"><h4>${esc(rule.title)}</h4>${seg}</div>${body}</section>`;
}

export function exitWarningsHtml({ draft, defaults }, esc) {
  const policy = effectivePolicy(defaults?.trader_defaults, draft.exit_policy_overrides);
  return exitWarnings(policy, draft.exit_mode, {
    slippagePct: draft.slippage_pct,
    feePct: defaults?.swap_fee_pct,
  })
    .map(
      (text) =>
        `<p class="copy-warning" role="note"><i class="icon-triangle-alert" aria-hidden="true"></i>${esc(text)}</p>`
    )
    .join("");
}

function exitsStep(context, esc) {
  const { draft } = context;
  const mode = EXIT_MODES.find((item) => item.id === draft.exit_mode) || EXIT_MODES[0];
  const preset = matchPreset(draft.exit_policy_overrides);
  const presets = [
    ...PRESETS.map(({ id, label }) => ({ id, label })),
    ...(preset === "custom" ? [{ id: "custom", label: I18n.t("copy-preset-custom") }] : []),
  ];
  const whoSells = I18n.t("copy-editor-who-sells");
  const presetLabel = I18n.t("copy-editor-preset");
  const mirrorNote = `<p class="copy-note">${esc(I18n.t("copy-editor-mirror-note", { mine: exitModeLabel("buy_only"), both: I18n.t("copy-editor-exit-both") }))}</p>`;
  return `<div class="copy-field"><span>${esc(whoSells)}</span>${segmented("exit-mode", EXIT_MODES, draft.exit_mode, esc, whoSells)}<small>${esc(mode.help)}</small></div>
    <div class="copy-field"><span>${esc(presetLabel)}</span>${segmented("preset", presets, preset, esc, presetLabel)}<small>${esc(I18n.t("copy-editor-preset-help"))}</small></div>
    <div id="copy-editor-warnings">${exitWarningsHtml(context, esc)}</div>
    ${draft.exit_mode === "mirror" ? mirrorNote : ""}
    <div class="copy-rule-cards${draft.exit_mode === "mirror" ? " is-inactive" : ""}">${RULES.map((rule) => ruleCard(rule, context, esc)).join("")}</div>`;
}

function reviewStep({ draft, defaults, mode, source }, esc) {
  const global = Boolean(defaults?.require_filter_pass);
  const task = {
    ...draft,
    target_address: mode === "edit" ? source.target_address : draft.target_address,
  };
  const head = `<div class="copy-review-head">${renderAddress(task.target_address, { explorer: "account" })}<p>${esc(
    I18n.t("copy-editor-review-head", {
      name: draft.label || I18n.t("copy-editor-unnamed"),
      mode: modeLabel(mode === "edit" && source.mode === "live" ? "live" : "paper"),
      status: draft.enabled
        ? I18n.t("copy-editor-review-processes")
        : I18n.t("copy-editor-review-paused"),
    })
  )}</p></div>`;
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

/** The first problem with a step, in the server's own terms, or null. */
export function validate(id, draft, { mode, defaults }) {
  if (id === "wallet") {
    if (mode !== "edit" && !SOLANA_ADDRESS_RE.test(draft.target_address || "")) {
      return I18n.t("copy-editor-error-address");
    }
  } else if (id === "sizing") {
    const amount = draft.sizing.kind === "fixed" ? draft.sizing.sol : draft.sizing.pct;
    if (
      ![amount, draft.max_sol_per_trade, draft.max_sol_per_token, draft.total_budget_sol].every(
        positive
      )
    ) {
      return I18n.t("copy-editor-error-sizing");
    }
    const minSol = defaults?.min_trade_size_sol ?? 0;
    if (draft.sizing.kind === "fixed" && draft.sizing.sol < minSol)
      return I18n.t("copy-editor-error-min-copy", { minimum: sol(minSol, 3) });
    if (draft.max_sol_per_trade < minSol)
      return I18n.t("copy-editor-error-min-cap", { minimum: sol(minSol, 3) });
    if (draft.max_sol_per_trade > draft.max_sol_per_token)
      return I18n.t("copy-editor-error-trade-cap");
    if (draft.max_sol_per_token > draft.total_budget_sol)
      return I18n.t("copy-editor-error-token-cap");
    const minSlippage = defaults?.min_slippage_pct ?? 0;
    const maxSlippage = defaults?.max_slippage_pct ?? Infinity;
    if (
      !positive(draft.slippage_pct) ||
      draft.slippage_pct < minSlippage ||
      draft.slippage_pct > maxSlippage
    ) {
      return I18n.t("copy-editor-error-slippage", {
        min: pct(minSlippage, 1),
        max: pct(maxSlippage, 0),
      });
    }
  } else if (id === "entry") {
    const { min_target_trade_sol: min, max_target_trade_sol: max } = draft;
    if ([min, max].some((value) => value !== null && !(Number.isFinite(value) && value >= 0))) {
      return I18n.t("copy-editor-error-target-limits");
    }
    if (min !== null && max !== null && min > max) return I18n.t("copy-editor-error-target-order");
  } else if (id === "exits") {
    return validateOverrides(draft.exit_policy_overrides);
  }
  return null;
}
