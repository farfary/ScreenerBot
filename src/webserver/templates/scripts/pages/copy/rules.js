// The Rules tab and the editor's review: every field with the value that applies
// and where it comes from (a task override or the inherited Trader default).
import { EXIT_MODE_LABELS, definitionRows, fixed } from "./format.js";
import { RULES, exitWarnings, fieldText, isOverridden } from "./policy.js";

function sizeText(task) {
  if (task.sizing?.kind === "ratio_of_target")
    return `${fixed(task.sizing.pct, 1)}% of the wallet's trade`;
  return `${fixed(task.sizing?.sol, 3)} SOL per copy`;
}

function targetRange(task) {
  const min = task.min_target_trade_sol;
  const max = task.max_target_trade_sol;
  if (min == null && max == null) return "Any size";
  if (max == null) return `At least ${fixed(min, 3)} SOL`;
  if (min == null) return `At most ${fixed(max, 3)} SOL`;
  return `${fixed(min, 3)} – ${fixed(max, 3)} SOL`;
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
          ? `Task override · Trader ${fieldText(field, traderDefaults?.[rule.group]?.[field.key])}`
          : "Trader default";
        return `<tr class="${inactive ? "is-inactive" : ""}"><td>${esc(field.label)}</td><td>${esc(fieldText(field, effective?.[rule.group]?.[field.key]))}</td><td class="copy-rules-source">${esc(source)}</td></tr>`;
      })
      .join("");
    return `<tr class="copy-rules-group"><th colspan="3" scope="rowgroup">${esc(rule.title)}</th></tr>${fields}`;
  }).join("");
  return `<div class="copy-table-wrap"><table class="copy-rules"><thead><tr><th scope="col">Rule</th><th scope="col">Applies</th><th scope="col">Source</th></tr></thead><tbody>${rows}</tbody></table></div>`;
}

/** Sizing, entry and exit sections for a task-shaped object. */
export function rulesHtml(context, esc) {
  const { task, effective, managesExits, requireFilter, globalRequireFilter } = context;
  const budgetNote = Number.isFinite(Number(task.spent_sol))
    ? `${fixed(task.spent_sol, 3)} spent · ${fixed(task.remaining_budget_sol, 3)} left`
    : "";
  const perToken = Number(task.max_sol_per_token);
  const perTrade = Number(task.max_sol_per_trade);
  const sizing = definitionRows(
    [
      ["Copy size", sizeText(task)],
      ["Per-trade cap", `${fixed(perTrade, 3)} SOL`],
      [
        "Per-token cap",
        `${fixed(perToken, 3)} SOL`,
        perTrade > 0
          ? `About ${Math.max(1, Math.floor(perToken / perTrade))} full copies of one token`
          : "",
      ],
      ["Total budget", `${fixed(task.total_budget_sol, 3)} SOL`, budgetNote],
      ["Slippage", `${fixed(task.slippage_pct, 1)}%`],
    ],
    esc
  );
  const entry = definitionRows(
    [
      ["Wallet trade size", targetRange(task)],
      [
        "Repeat buys",
        task.buy_once_per_token
          ? "First buy of each token only"
          : "Every buy, up to the per-token cap",
      ],
      [
        "Filtering pass",
        requireFilter ? "Required" : "Not required",
        task.require_filter_pass == null
          ? `Copy setting (${globalRequireFilter ? "required" : "not required"})`
          : "Task override",
      ],
    ],
    esc
  );
  const exitNote = managesExits
    ? ""
    : '<p class="copy-note">Holdings are sold only when the wallet sells; the rules below do not run in this mode.</p>';
  return `<div class="copy-rules-grid">
    <section class="copy-card"><h4>Sizing</h4><dl class="copy-defs">${sizing}</dl></section>
    <section class="copy-card"><h4>Entry filters</h4><dl class="copy-defs">${entry}</dl></section>
  </div>
  <section class="copy-card copy-rules-exits">
    <h4>Exits <span class="copy-card-sub">${esc(EXIT_MODE_LABELS[task.exit_mode] || task.exit_mode)}</span></h4>
    ${warningList(exitWarnings(effective, task.exit_mode), esc)}${exitNote}
    ${ruleTable(context, esc)}
  </section>`;
}

export function renderRules({ ws }, esc) {
  return `<div class="copy-panel-head"><h3>Rules in effect</h3><button class="btn btn-secondary btn-sm" type="button" data-ws-action="edit" data-step="exits"><i class="icon-pencil" aria-hidden="true"></i> Edit rules</button></div>${rulesHtml(
    {
      task: ws,
      effective: ws.effective_policy,
      traderDefaults: ws.trader_defaults,
      managesExits: ws.policy_manages_exits,
      requireFilter: ws.effective_require_filter_pass,
      globalRequireFilter: ws.global_require_filter_pass,
    },
    esc
  )}`;
}
