// The stepped task editor: Wallet → Sizing → Entry filters → Exits → Review.
// Creating and cloning post a new task; editing patches every field except the
// wallet, which is a task's identity.
import { MODE_LABELS, taskName } from "./format.js";
import { PRESETS, normalizeOverrides } from "./policy.js";
import {
  STEPS,
  collect,
  costPreview,
  exitWarningsHtml,
  stepHtml,
  validate,
} from "./editor_steps.js";

/** Starting values for a new task; every one is edited before it is saved. */
const NEW_TASK = {
  sizing: { kind: "fixed", sol: 0.05 },
  max_sol_per_trade: 0.1,
  max_sol_per_token: 0.5,
  total_budget_sol: 2,
};

export function createEditor(page) {
  const { $, Utils, api, state, on, toast, dialogs } = page;
  const esc = Utils.escapeHtml;
  let mode = "create";
  let source = null;
  let draft = null;
  let step = 0;
  let visited = 0;
  let shownStep = null;
  const sizingMemory = { fixed: null, ratio_of_target: null };

  const context = () => ({ draft, mode, source, defaults: state.defaults });
  const body = () => $("#copy-editor-body");

  function setup() {
    on($("#copy-editor-next"), "click", next);
    on($("#copy-editor-back"), "click", back);
    on($("#copy-editor-save"), "click", save);
    on($("#copy-editor-steps"), "click", (event) => {
      const button = event.target.closest("[data-editor-step]");
      if (button) goTo(Number(button.dataset.editorStep));
    });
    on(body(), "click", onSegment);
    on(body(), "input", onInput);
    on(body(), "change", onInput);
    on(body(), "submit", (event) => event.preventDefault());
  }

  function fromTask(task) {
    return {
      target_address: task.target_address,
      label: task.label ?? null,
      enabled: Boolean(task.enabled),
      mode: task.mode || "paper",
      sizing: { ...task.sizing },
      exit_mode: task.exit_mode || "buy_only",
      exit_policy_overrides: normalizeOverrides(task.exit_policy_overrides),
      max_sol_per_trade: task.max_sol_per_trade,
      max_sol_per_token: task.max_sol_per_token,
      total_budget_sol: task.total_budget_sol,
      min_target_trade_sol: task.min_target_trade_sol ?? null,
      max_target_trade_sol: task.max_target_trade_sol ?? null,
      buy_once_per_token: Boolean(task.buy_once_per_token),
      slippage_pct: task.slippage_pct,
      require_filter_pass: task.require_filter_pass ?? null,
    };
  }

  function input() {
    return {
      target_address: mode === "edit" ? source.target_address : draft.target_address,
      label: draft.label,
      enabled: draft.enabled,
      mode: draft.mode,
      sizing: draft.sizing,
      exit_mode: draft.exit_mode,
      exit_policy_overrides: draft.exit_policy_overrides,
      max_sol_per_trade: draft.max_sol_per_trade,
      max_sol_per_token: draft.max_sol_per_token,
      total_budget_sol: draft.total_budget_sol,
      min_target_trade_sol: draft.min_target_trade_sol,
      max_target_trade_sol: draft.max_target_trade_sol,
      buy_once_per_token: draft.buy_once_per_token,
      slippage_pct: draft.slippage_pct,
      require_filter_pass: draft.require_filter_pass,
    };
  }

  async function start(stepId) {
    if (!state.defaults) await page.reloadDefaults();
    if (mode !== "edit" && draft.slippage_pct == null) {
      draft.slippage_pct = state.defaults?.default_slippage_pct ?? null;
    }
    step = Math.max(
      0,
      STEPS.findIndex((item) => item.id === stepId)
    );
    visited = mode === "edit" ? STEPS.length - 1 : step;
    sizingMemory.fixed = draft.sizing.kind === "fixed" ? draft.sizing.sol : null;
    sizingMemory.ratio_of_target =
      draft.sizing.kind === "ratio_of_target" ? draft.sizing.pct : null;
    const title = $("#copy-editor-title");
    const sub = $("#copy-editor-sub");
    if (title) {
      title.textContent =
        mode === "edit"
          ? `Edit ${taskName(source)}`
          : mode === "clone"
            ? `Clone ${taskName(source)}`
            : "Add wallet";
    }
    if (sub) {
      sub.textContent =
        mode === "edit"
          ? `${MODE_LABELS[source.mode] || source.mode} task · changes apply to its next decisions`
          : mode === "clone"
            ? "Same rules, empty paper book, starts in Paper"
            : "New tasks start in Paper";
    }
    shownStep = null;
    render();
    dialogs.show("copy-editor");
    body()?.querySelector("input, select, button")?.focus();
  }

  function openCreate(prefill = {}) {
    mode = "create";
    source = null;
    draft = {
      ...fromTask({
        ...NEW_TASK,
        target_address: "",
        label: null,
        enabled: true,
        mode: "paper",
        exit_mode: "buy_only",
        exit_policy_overrides: PRESETS[0].overrides,
        buy_once_per_token: true,
        slippage_pct: state.defaults?.default_slippage_pct ?? null,
      }),
      ...prefill,
    };
    void start("wallet");
  }

  function openEdit(task, stepId = "wallet") {
    mode = "edit";
    source = task;
    draft = fromTask(task);
    void start(stepId);
  }

  function openClone(task) {
    mode = "clone";
    source = task;
    draft = { ...fromTask(task), label: `${taskName(task)} (copy)`, enabled: false, mode: "paper" };
    void start("wallet");
  }

  function setError(text) {
    const node = $("#copy-editor-error");
    if (node) node.textContent = text || "";
  }

  function render() {
    const current = STEPS[step];
    const nav = $("#copy-editor-steps");
    if (nav) {
      nav.innerHTML = STEPS.map((item, index) => {
        const locked = index > visited;
        return `<li><button type="button" class="copy-step${index === step ? " is-current" : ""}${index < step ? " is-done" : ""}" data-editor-step="${index}"${locked ? " disabled" : ""} aria-current="${index === step ? "step" : "false"}"><span class="copy-step-index">${index + 1}</span>${esc(item.label)}</button></li>`;
      }).join("");
    }
    const node = body();
    if (node) {
      node.innerHTML = stepHtml(current.id, context(), esc);
      // A new step opens at its top, not at the previous step's scroll offset.
      if (shownStep !== step) node.scrollTop = 0;
      shownStep = step;
    }
    const last = step === STEPS.length - 1;
    const saving = last || mode === "edit";
    const backButton = $("#copy-editor-back");
    const nextButton = $("#copy-editor-next");
    const saveButton = $("#copy-editor-save");
    if (backButton) backButton.hidden = step === 0;
    if (nextButton) {
      // Next leads the flow until Save is on screen; one primary action at a time.
      nextButton.hidden = last;
      nextButton.classList.toggle("btn-primary", !saving);
      nextButton.classList.toggle("btn-secondary", saving);
    }
    if (saveButton) {
      saveButton.hidden = !saving;
      saveButton.textContent =
        mode === "edit" ? "Save changes" : mode === "clone" ? "Create clone" : "Create paper task";
    }
  }

  function readStep() {
    const node = body();
    if (node && draft) collect(node, draft);
  }

  function goTo(index) {
    if (index === step || index < 0 || index >= STEPS.length || index > visited) return;
    readStep();
    if (index > step) {
      const problem = validate(STEPS[step].id, draft, context());
      if (problem) return setError(problem);
    }
    setError("");
    step = index;
    render();
  }

  function next() {
    readStep();
    const problem = validate(STEPS[step].id, draft, context());
    if (problem) return setError(problem);
    setError("");
    step = Math.min(STEPS.length - 1, step + 1);
    visited = Math.max(visited, step);
    render();
  }

  function back() {
    readStep();
    setError("");
    step = Math.max(0, step - 1);
    render();
  }

  function onSegment(event) {
    const button = event.target.closest("[data-seg-value]");
    const group = button?.closest("[data-seg]");
    if (!button || !group) return;
    readStep();
    const value = button.dataset.segValue;
    const name = group.dataset.seg;
    if (name === "sizing-kind" && value !== draft.sizing.kind) {
      sizingMemory[draft.sizing.kind] =
        draft.sizing.kind === "fixed" ? draft.sizing.sol : draft.sizing.pct;
      draft.sizing =
        value === "fixed"
          ? { kind: "fixed", sol: sizingMemory.fixed }
          : { kind: "ratio_of_target", pct: sizingMemory.ratio_of_target };
    } else if (name === "filter-mode") {
      draft.require_filter_pass = value === "inherit" ? null : value === "require";
    } else if (name === "exit-mode") {
      draft.exit_mode = value;
    } else if (name === "preset") {
      const preset = PRESETS.find((item) => item.id === value);
      if (preset) draft.exit_policy_overrides = normalizeOverrides(preset.overrides);
    } else if (name.startsWith("rule-")) {
      const rule = draft.exit_policy_overrides[name.slice(5)];
      if (!rule) return;
      if (value === "on") rule.enabled = true;
      else
        Object.keys(rule).forEach(
          (key) => (rule[key] = key === "enabled" && value === "off" ? false : null)
        );
    } else {
      return;
    }
    render();
  }

  function onInput() {
    if (!draft) return;
    const id = STEPS[step].id;
    if (id !== "sizing" && id !== "exits") return;
    readStep();
    if (id === "sizing") {
      const preview = $("#copy-editor-preview");
      if (preview) preview.innerHTML = costPreview(draft);
    } else {
      const warnings = $("#copy-editor-warnings");
      if (warnings) warnings.innerHTML = exitWarningsHtml(context(), esc);
    }
  }

  async function save(event) {
    readStep();
    for (let index = 0; index < STEPS.length - 1; index += 1) {
      const problem = validate(STEPS[index].id, draft, context());
      if (problem) {
        step = index;
        render();
        return setError(problem);
      }
    }
    setError("");
    const button = event.currentTarget;
    button.disabled = true;
    try {
      if (mode === "edit") {
        const patch = input();
        delete patch.target_address;
        delete patch.mode;
        await api.update(source.id, patch);
        dialogs.hide("copy-editor");
        toast("success", "Task updated", taskName(source));
      } else {
        const response = await api.create(input());
        dialogs.hide("copy-editor");
        toast(
          "success",
          mode === "clone" ? "Clone created" : "Paper task created",
          taskName(response.task)
        );
        state.view = "task";
        state.selectedId = response.task?.id ?? state.selectedId;
      }
      await page.reload();
    } catch (error) {
      setError(error.detail);
    } finally {
      button.disabled = false;
    }
  }

  return { setup, openCreate, openEdit, openClone };
}
