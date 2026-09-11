// The stepped task editor's steps: Wallet, Sizing (with a cost preview), Entry
// filters, Exits (presets and every rule field, inherited values shown) and a
// Review of what the task will run under. Pure markup, collection and checks.
import { renderAddress } from "../../ui/token_identity.js";
import { SOLANA_ADDRESS_RE, fixed, segmented, sol } from "./format.js";
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
  { id: "wallet", label: "Wallet" },
  { id: "sizing", label: "Sizing" },
  { id: "entry", label: "Entry filters" },
  { id: "exits", label: "Exits" },
  { id: "review", label: "Review" },
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
    label: "My exit rules",
    help: "Your rules below sell every holding; the wallet's sells are ignored.",
  },
  {
    id: "hybrid",
    label: "Both",
    help: "Whichever comes first: the wallet sells, or one of your rules fires.",
  },
  {
    id: "mirror",
    label: "Mirror wallet sells",
    help: "Holdings are sold only when the wallet sells. Your exit rules do not run.",
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

function walletStep({ draft, mode, source }, esc) {
  const address =
    mode === "edit"
      ? `<div class="copy-field"><span>Wallet</span>${renderAddress(source.target_address, { explorer: "account" })}<small>A task's wallet is its identity. To copy another wallet with these rules, clone the task.</small></div>`
      : `<label class="copy-field"><span>Wallet address</span><input type="text" data-field="target_address" value="${esc(draft.target_address || "")}" placeholder="Solana wallet address" spellcheck="false" autocomplete="off" required /><small>${esc(
          mode === "clone"
            ? "Same rules with an empty paper book. Keep this wallet to test other rules on it, or enter another wallet."
            : "The wallet whose buys (and, if you choose, sells) this task copies."
        )}</small></label>`;
  const note =
    mode === "edit" && source.mode === "live"
      ? "This task is live: changes apply to its next real copies."
      : "Tasks run in Paper until you arm them: trades are simulated at the pool price and nothing is spent.";
  return `${address}
    <label class="copy-field"><span>Name <em>optional</em></span><input type="text" data-field="label" value="${esc(draft.label || "")}" maxlength="64" placeholder="e.g. Fast rotator" /></label>
    ${toggleRow(esc, { name: "enabled", title: "Process the wallet's trades", help: "Off keeps the task paused until you resume it.", checked: draft.enabled })}
    <p class="copy-note">${esc(note)}</p>`;
}

/** What a copy costs under the draft's sizing, before network and priority fees. */
export function costPreview(draft) {
  const amount = draft.sizing.kind === "fixed" ? draft.sizing.sol : draft.sizing.pct;
  const cap = draft.max_sol_per_trade;
  const perToken = draft.max_sol_per_token;
  const budget = draft.total_budget_sol;
  if (![amount, cap, perToken, budget].every((value) => Number.isFinite(value) && value > 0)) {
    return "<p>Enter the sizing to see what a copy costs.</p>";
  }
  const copyFor = (target) =>
    draft.sizing.kind === "fixed" ? Math.min(amount, cap) : Math.min((target * amount) / 100, cap);
  const examples = [0.1, 1, 5]
    .map(
      (target) =>
        `<li>The wallet buys ${sol(target, 1)} → you copy <strong>${sol(copyFor(target), 3)}</strong></li>`
    )
    .join("");
  const unit = draft.sizing.kind === "fixed" ? Math.min(amount, cap) : cap;
  const perTokenCopies = Math.max(1, Math.floor(perToken / unit));
  const budgetCopies = Math.floor(budget / unit);
  return `<ul>${examples}</ul><p>One token takes at most ${perTokenCopies} cop${perTokenCopies === 1 ? "y" : "ies"} of ${sol(unit, 3)}; the budget covers ${draft.sizing.kind === "fixed" ? "about" : "at least"} ${budgetCopies} of them. Network and priority fees come on top.</p>`;
}

function sizingStep({ draft, defaults }, esc) {
  const fixedKind = draft.sizing.kind === "fixed";
  const maxSlippage = defaults?.max_slippage_pct ?? null;
  return `<div class="copy-field"><span>Copy size</span>${segmented(
    "sizing-kind",
    [
      { id: "fixed", label: "Fixed amount" },
      { id: "ratio_of_target", label: "Share of the wallet's trade" },
    ],
    draft.sizing.kind,
    esc
  )}</div>
    <div class="copy-fields">
      ${numberInput(esc, { attr: "data-field", name: "sizing_amount", label: fixedKind ? "Amount per copy" : "Share of each trade", unit: fixedKind ? "SOL" : "%", value: valueAttr(fixedKind ? draft.sizing.sol : draft.sizing.pct), min: 0, required: true, help: fixedKind ? "Spent on each copied buy." : "Of the wallet's own buy, up to the per-trade cap." })}
      ${numberInput(esc, { attr: "data-field", name: "max_sol_per_trade", label: "Per-trade cap", unit: "SOL", value: valueAttr(draft.max_sol_per_trade), min: 0, required: true, help: "No single copy spends more." })}
      ${numberInput(esc, { attr: "data-field", name: "max_sol_per_token", label: "Per-token cap", unit: "SOL", value: valueAttr(draft.max_sol_per_token), min: 0, required: true, help: "Total spent on one token." })}
      ${numberInput(esc, { attr: "data-field", name: "total_budget_sol", label: "Total budget", unit: "SOL", value: valueAttr(draft.total_budget_sol), min: 0, required: true, help: "Everything this task may spend." })}
      ${numberInput(esc, { attr: "data-field", name: "slippage_pct", label: "Slippage", unit: "%", value: valueAttr(draft.slippage_pct), min: 0, max: maxSlippage, required: true, placeholder: defaults ? String(defaults.default_slippage_pct) : "" })}
    </div>
    <section class="copy-preview" aria-live="polite"><h4>What a copy costs</h4><div id="copy-editor-preview">${costPreview(draft)}</div></section>`;
}

function entryStep({ draft, defaults }, esc) {
  const global = Boolean(defaults?.require_filter_pass);
  const filterMode =
    draft.require_filter_pass == null ? "inherit" : draft.require_filter_pass ? "require" : "skip";
  const requires = draft.require_filter_pass ?? global;
  return `<div class="copy-fields">
      ${numberInput(esc, { attr: "data-field", name: "min_target_trade_sol", label: "Smallest wallet trade copied", unit: "SOL", value: valueAttr(draft.min_target_trade_sol), min: 0, placeholder: "Any", help: "Ignore the wallet's smaller buys. Leave empty for no minimum." })}
      ${numberInput(esc, { attr: "data-field", name: "max_target_trade_sol", label: "Largest wallet trade copied", unit: "SOL", value: valueAttr(draft.max_target_trade_sol), min: 0, placeholder: "Any", help: "Ignore the wallet's larger buys. Leave empty for no maximum." })}
    </div>
    ${toggleRow(esc, { name: "buy_once_per_token", title: "Buy each token once", help: "Copy only the wallet's first buy of a token; later buys of it are skipped.", checked: draft.buy_once_per_token })}
    <div class="copy-field"><span>Filtering pass</span>${segmented(
      "filter-mode",
      [
        { id: "inherit", label: `Copy setting (${global ? "required" : "not required"})` },
        { id: "require", label: "Require" },
        { id: "skip", label: "Don't require" },
      ],
      filterMode,
      esc
    )}<small>Require a token to pass your Filtering pipeline before it is copied.</small></div>
    ${requires ? '<p class="copy-warning" role="note"><i class="icon-triangle-alert" aria-hidden="true"></i>With the default Filtering setup almost every token fails, so a task that requires a pass copies nothing. Require it only when your filters pass the tokens this wallet trades.</p>' : ""}`;
}

function ruleCard(rule, { draft, defaults }, esc) {
  const overrides = draft.exit_policy_overrides[rule.group];
  const inherited = defaults?.trader_defaults;
  const state = overrides.enabled === null ? "inherit" : overrides.enabled ? "on" : "off";
  const inheritLabel = inherited
    ? `Trader default (${inherited[rule.group]?.enabled ? "on" : "off"})`
    : "Trader default";
  const seg = segmented(
    `rule-${rule.group}`,
    [
      { id: "inherit", label: inheritLabel },
      { id: "on", label: "On" },
      { id: "off", label: "Off" },
    ],
    state,
    esc,
    `${rule.title} setting`
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
          return `<label class="copy-field"><span>${esc(field.label)}</span><select data-custom-select data-rule-field="${name}"><option value=""${value === null ? " selected" : ""}>${esc(`Trader default (${fieldText(field, fallback)})`)}</option><option value="true"${value === true ? " selected" : ""}>${esc(field.text(true))}</option><option value="false"${value === false ? " selected" : ""}>${esc(field.text(false))}</option></select></label>`;
        }
        return numberInput(esc, {
          attr: "data-rule-field",
          name,
          label: field.label,
          unit: field.unit,
          value: valueAttr(overrides[field.key], field.scale),
          placeholder: valueAttr(fallback, field.scale),
          integer: field.integer,
          help: `Empty uses the Trader default: ${fieldText(field, fallback)}`,
        });
      })
      .join("")}</div>`;
  } else if (state === "inherit") {
    body = `<p class="copy-note">${esc(inherited ? `Follows the Trader: ${ruleSummary(rule, inherited)}` : "Follows the Trader's setting.")}</p>`;
  } else {
    body = '<p class="copy-note">Off for this task, whatever the Trader uses.</p>';
  }
  return `<section class="copy-rule-card"><div class="copy-rule-head"><h4>${esc(rule.title)}</h4>${seg}</div>${body}</section>`;
}

export function exitWarningsHtml({ draft, defaults }, esc) {
  const policy = effectivePolicy(defaults?.trader_defaults, draft.exit_policy_overrides);
  return exitWarnings(policy, draft.exit_mode)
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
    ...(preset === "custom" ? [{ id: "custom", label: "Custom" }] : []),
  ];
  return `<div class="copy-field"><span>Who sells</span>${segmented("exit-mode", EXIT_MODES, draft.exit_mode, esc)}<small>${esc(mode.help)}</small></div>
    <div class="copy-field"><span>Preset</span>${segmented("preset", presets, preset, esc)}<small>A preset fills every rule below; adjust any of them after.</small></div>
    <div id="copy-editor-warnings">${exitWarningsHtml(context, esc)}</div>
    <div class="copy-rule-cards${draft.exit_mode === "mirror" ? " is-inactive" : ""}">${RULES.map((rule) => ruleCard(rule, context, esc)).join("")}</div>`;
}

function reviewStep({ draft, defaults, mode, source }, esc) {
  const global = Boolean(defaults?.require_filter_pass);
  const task = {
    ...draft,
    target_address: mode === "edit" ? source.target_address : draft.target_address,
  };
  const head = `<div class="copy-review-head">${renderAddress(task.target_address, { explorer: "account" })}<p>${esc(
    `${draft.label || "Unnamed task"} · ${mode === "edit" && source.mode === "live" ? "Live" : "Paper"} · ${draft.enabled ? "processes trades once saved" : "saved paused"}`
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
      return "Enter a valid Solana wallet address.";
    }
  } else if (id === "sizing") {
    const amount = draft.sizing.kind === "fixed" ? draft.sizing.sol : draft.sizing.pct;
    if (
      ![amount, draft.max_sol_per_trade, draft.max_sol_per_token, draft.total_budget_sol].every(
        positive
      )
    ) {
      return "Every sizing value must be above zero.";
    }
    if (draft.max_sol_per_trade > draft.max_sol_per_token)
      return "The per-trade cap cannot exceed the per-token cap.";
    if (draft.max_sol_per_token > draft.total_budget_sol)
      return "The per-token cap cannot exceed the total budget.";
    const maxSlippage = defaults?.max_slippage_pct ?? Infinity;
    if (!positive(draft.slippage_pct) || draft.slippage_pct > maxSlippage) {
      return `Slippage must be above 0% and at most ${fixed(maxSlippage, 0)}%.`;
    }
  } else if (id === "entry") {
    const { min_target_trade_sol: min, max_target_trade_sol: max } = draft;
    if ([min, max].some((value) => value !== null && !(Number.isFinite(value) && value >= 0))) {
      return "Wallet trade limits must be zero or more.";
    }
    if (min !== null && max !== null && min > max)
      return "The smallest wallet trade cannot exceed the largest.";
  } else if (id === "exits") {
    return validateOverrides(draft.exit_policy_overrides);
  }
  return null;
}
