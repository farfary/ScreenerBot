// Arming live is its own step: the readiness evidence from the paper book, what
// real exposure the task carries, and explicit acknowledgements before the
// confirmation phrase is sent.
import { EXIT_MODE_LABELS, definitionRows, fixed, plural, taskName } from "./format.js";
import { RULES, ruleSummary } from "./policy.js";

export function createArmGate(page) {
  const { $, Utils, api, state, on, toast, paint, dialogs } = page;
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
        !runtimeBlocked() && pending ? `${plural(pending, "acknowledgement")} left to tick` : "";
    }
  }

  function body(ws) {
    const checks = (ws.readiness?.checks || [])
      .map(
        (check) =>
          `<li class="copy-check ${check.passed ? "is-passed" : "is-failed"}"><i class="${check.passed ? "icon-circle-check" : "icon-circle-x"}" aria-hidden="true"></i><span><strong>${esc(check.label)}</strong><small>${esc(check.detail)}</small></span><span class="sr-only">${check.passed ? "passed" : "not passed"}</span></li>`
      )
      .join("");
    const size =
      ws.sizing?.kind === "ratio_of_target"
        ? `${fixed(ws.sizing.pct, 1)}% of the wallet's trade`
        : `${fixed(ws.sizing?.sol, 3)} SOL`;
    const exposure = definitionRows(
      [
        ["Per copy", size],
        ["Per-trade cap", `${fixed(ws.max_sol_per_trade, 3)} SOL`],
        ["Per-token cap", `${fixed(ws.max_sol_per_token, 3)} SOL`],
        [
          "Budget left",
          `${fixed(ws.remaining_budget_sol, 3)} of ${fixed(ws.total_budget_sol, 3)} SOL`,
        ],
        ["Slippage", `${fixed(ws.slippage_pct, 1)}%`],
        ["Exits", EXIT_MODE_LABELS[ws.exit_mode] || ws.exit_mode],
        [
          "Stop loss",
          ws.policy_manages_exits
            ? ruleSummary(RULES[0], ws.effective_policy)
            : "Wallet sells only",
        ],
      ],
      esc
    );
    const ack = (text) =>
      `<label class="copy-ack"><input type="checkbox" data-ack /><span>${esc(text)}</span></label>`;
    const acks = runtimeBlocked()
      ? '<p class="copy-warning" role="alert"><i class="icon-triangle-alert" aria-hidden="true"></i>Live execution is unavailable right now; see the last check.</p>'
      : [
          ack(
            `Real SOL: this task can spend up to ${fixed(ws.remaining_budget_sol, 3)} SOL from your wallet, at most ${fixed(ws.max_sol_per_trade, 3)} SOL per copy.`
          ),
          ack(
            "Live copies pay real network fees and slippage; paper results do not promise live results."
          ),
          ws.readiness?.ready
            ? ""
            : ack("Some readiness checks have not passed. Arm this task anyway."),
        ].join("");
    return `<p class="copy-arm-lead">${esc(`“${taskName(ws)}” will copy this wallet's trades with real swaps from your wallet.`)}</p>
      <h4>Readiness from the paper book</h4><ul class="copy-checks">${checks}</ul>
      <h4>Exposure</h4><dl class="copy-defs">${exposure}</dl>
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
          detail: "The live confirmation could not be loaded",
        });
      await api.setMode(task.id, "live", confirmation);
      dialogs.hide("copy-arm");
      toast("warning", "Live copying armed", taskName(task));
      await page.reload();
    } catch (failure) {
      if (error) error.textContent = failure.detail || "Live copying could not be armed";
      sync();
    }
  }

  return { setup, open };
}
