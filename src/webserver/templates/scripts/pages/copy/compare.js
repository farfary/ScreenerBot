// Compare view: every task's closed rounds over one date range, as curves and a
// table. Choosing a wallet opens its workspace.
import { comparisonCurves } from "./charts.js";
import {
  RANGES,
  duration,
  fixed,
  modeLabel,
  pct,
  rangeQuery,
  seconds,
  segmented,
  signedPct,
  signedSol,
  toneClass,
} from "./format.js";
import { panelMessage } from "./overview.js";

export function createCompare(page) {
  const { $, Utils, api, state, on, paint } = page;
  const esc = Utils.escapeHtml;
  const data = new Map();
  const errors = new Map();

  function setup() {
    on($("#copy-compare"), "click", (event) => {
      const range = event.target.closest('[data-seg="compare-range"] [data-seg-value]');
      if (range) {
        state.range = range.dataset.segValue;
        render();
        void refresh();
        return;
      }
      const task = event.target.closest("[data-compare-task]");
      if (task) page.select(Number(task.dataset.compareTask));
      else if (event.target.closest("[data-compare-close]")) page.openCompare();
    });
  }

  async function refresh() {
    const range = state.range;
    try {
      const response = await api.compare(rangeQuery(range));
      data.set(range, response.tasks || []);
      errors.delete(range);
    } catch (error) {
      errors.set(range, error.detail);
    }
    if (state.view === "compare") render();
  }

  function table(rows) {
    const body = [...rows]
      .sort((a, b) => b.realized_pnl_sol - a.realized_pnl_sol)
      .map(
        (row) => `<tr>
          <td><button class="copy-token-link" type="button" data-compare-task="${row.task_id}">${esc(row.name)}</button></td>
          <td><span class="copy-row-mode copy-mode-${esc(row.mode)}">${esc(modeLabel(row.mode))}</span>${row.enabled ? "" : `<small class="copy-muted"> ${esc(I18n.t("copy-paused-suffix"))}</small>`}</td>
          <td class="num">${row.rounds}</td>
          <td class="num">${esc(row.rounds ? pct(row.win_rate_pct, 0) : "—")}</td>
          <td class="num ${toneClass(row.realized_pnl_sol)}">${esc(signedSol(row.realized_pnl_sol))}</td>
          <td class="num">${esc(fixed(row.profit_factor, 2))}</td>
          <td class="num">${esc(duration(row.average_hold_seconds))}</td>
          <td class="num">${esc(seconds(row.arrival_median_ms))}</td>
          <td class="num">${esc(signedPct(row.slippage_median_pct, 2))}</td>
          <td class="num">${row.fills}</td>
          <td class="num">${row.skips}</td>
        </tr>`
      )
      .join("");
    const head = [
      ["", I18n.t("copy-table-wallet")],
      ["", I18n.t("copy-table-mode")],
      ["num", I18n.t("copy-table-rounds")],
      ["num", I18n.t("copy-metric-win-rate")],
      ["num", I18n.t("copy-table-realized")],
      ["num", I18n.t("copy-table-profit-factor")],
      ["num", I18n.t("copy-table-average-hold")],
      ["num", I18n.t("copy-metric-median-arrival")],
      ["num", I18n.t("copy-table-median-slippage")],
      ["num", I18n.t("copy-kind-fills")],
      ["num", I18n.t("copy-kind-skips")],
    ]
      .map(([cls, text]) => `<th scope="col"${cls ? ` class="${cls}"` : ""}>${esc(text)}</th>`)
      .join("");
    return `<div class="copy-table-wrap"><table class="copy-table"><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table></div>`;
  }

  function render() {
    const root = $("#copy-compare");
    if (!root) return;
    if (state.view !== "compare") {
      root.hidden = true;
      return;
    }
    root.hidden = false;
    const workspace = $("#copy-workspace");
    if (workspace) workspace.hidden = true;
    const rows = data.get(state.range);
    const error = errors.get(state.range);
    const head = `<div class="copy-panel-head"><h2>${esc(I18n.t("copy-compare-title"))}</h2><div class="copy-panel-tools">${segmented("compare-range", RANGES, state.range, esc, I18n.attr("copy-range-label", "aria-label"))}<button class="btn btn-ghost btn-sm" type="button" data-compare-close>${esc(I18n.t("copy-compare-back"))}</button></div></div>`;
    let body;
    if (!rows)
      body = error
        ? panelMessage(I18n.t("copy-compare-load-failed", { error }), esc, "is-error")
        : panelMessage(I18n.t("copy-compare-loading"), esc);
    else if (!rows.length) body = panelMessage(I18n.t("copy-compare-empty"), esc);
    else {
      body = `<section class="copy-card"><h4>${esc(I18n.t("copy-compare-curve-title"))}</h4>${comparisonCurves(
        rows.map((row) => ({ name: row.name, points: row.pnl_curve })),
        { escapeHtml: esc }
      )}</section>${table(rows)}`;
    }
    paint(root, head + body);
  }

  return { setup, refresh, render };
}
