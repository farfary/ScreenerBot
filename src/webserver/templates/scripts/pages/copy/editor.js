// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The stepped task editor: Wallet → Sizing → Entry filters → Exits → Review.
// Creating and cloning post a new task; editing patches every field except the
// wallet, which is a task's identity.
import { modeLabel, taskName } from "./format.js";
import { PRESETS, normalizeOverrides } from "./policy.js";
import {
  STEPS,
  collect,
  costPreview,
  duplicateNote,
  exitWarningsHtml,
  stepHtml,
  validate,
} from "./editor_steps.js";

/** Starting values for a new task; every one is edited before it is saved. */
const NEW_TASK = {
  sizing: { kind: "fixed", sol: 0.05 },
  max_native_per_trade: 0.1,
  max_native_per_token: 0.5,
  total_budget_native: 2,
};

export function createEditor(page) {
  const { $, Utils, api, state, on, notify, confirm, dialogs } = page;
  const esc = Utils.escapeHtml;
  let mode = "create";
  let source = null;
  let draft = null;
  let baseline = "";
  let step = 0;
  let visited = 0;
  let shownStep = null;
  const sizingMemory = { fixed: null, ratio_of_target: null };

  const context = () => ({
    draft,
    mode,
    source,
    defaults: state.defaults,
    tasks: state.overview?.tasks || [],
  });
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
      max_native_per_trade: task.max_native_per_trade,
      max_native_per_token: task.max_native_per_token,
      total_budget_native: task.total_budget_native,
      min_target_trade_native: task.min_target_trade_native ?? null,
      max_target_trade_native: task.max_target_trade_native ?? null,
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
      max_native_per_trade: draft.max_native_per_trade,
      max_native_per_token: draft.max_native_per_token,
      total_budget_native: draft.total_budget_native,
      min_target_trade_native: draft.min_target_trade_native,
      max_target_trade_native: draft.max_target_trade_native,
      buy_once_per_token: draft.buy_once_per_token,
      slippage_pct: draft.slippage_pct,
      require_filter_pass: draft.require_filter_pass,
    };
  }

  /** A new task starts from the copy default slippage once the defaults are known. */
  function fillDefaults() {
    if (mode !== "edit" && draft.slippage_pct == null) {
      draft.slippage_pct = state.defaults?.default_slippage_pct ?? null;
    }
  }

  const dirty = () => JSON.stringify(draft) !== baseline;

  /** Closing by Escape, the backdrop or the close button keeps unsaved work unless confirmed. */
  async function confirmDiscard() {
    readStep();
    if (!dirty()) return true;
    const result = await confirm({
      title:
        mode === "edit" ? I18n.t("copy-editor-discard-edit") : I18n.t("copy-editor-discard-create"),
      message:
        mode === "edit"
          ? I18n.t("copy-editor-discard-edit-message", { name: taskName(source) })
          : I18n.t("copy-editor-discard-create-message"),
      confirmLabel: I18n.t("copy-editor-discard-confirm"),
      cancelLabel: I18n.t("copy-editor-keep-editing"),
      variant: "warning",
    });
    return result.confirmed;
  }

  function start(stepId) {
    fillDefaults();
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
          ? I18n.t("copy-editor-title-edit", { name: taskName(source) })
          : mode === "clone"
            ? I18n.t("copy-editor-title-clone", { name: taskName(source) })
            : I18n.t("copy-editor-title-add");
    }
    if (sub) {
      sub.textContent =
        mode === "edit"
          ? I18n.t("copy-editor-sub-edit", { mode: modeLabel(source.mode) })
          : mode === "clone"
            ? I18n.t("copy-editor-sub-clone")
            : I18n.t("copy-editor-sub-add");
    }
    shownStep = null;
    render();
    baseline = JSON.stringify(draft);
    dialogs.show("copy-editor", { beforeClose: confirmDiscard });
    body()?.querySelector("input, select, button")?.focus();
    // The page loads the defaults at start; the editor never waits for them to open.
    if (!state.defaults) {
      void page.reloadDefaults().then(() => {
        if (!state.defaults || !dialogs.isOpen("copy-editor")) return;
        readStep();
        const untouched = !dirty();
        fillDefaults();
        if (untouched) baseline = JSON.stringify(draft);
        render();
      });
    }
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
    start("wallet");
  }

  function openEdit(task, stepId = "wallet") {
    mode = "edit";
    source = task;
    draft = fromTask(task);
    start(stepId);
  }

  function openClone(task) {
    mode = "clone";
    source = task;
    draft = {
      ...fromTask(task),
      label: `${taskName(task)} ${I18n.t("copy-editor-clone-suffix")}`,
      enabled: false,
      mode: "paper",
    };
    start("wallet");
  }

  /**
   * Show a problem on the error line above the footer. A step problem (`{ message,
   * field }` from `validate`) also marks the input that holds it invalid and focuses
   * it; a server error is a plain message.
   */
  function setError(problem) {
    const node = $("#copy-editor-error");
    const message = typeof problem === "string" ? problem : problem?.message;
    if (node) node.textContent = message || "";
    const stepNode = body();
    stepNode?.querySelectorAll('[aria-invalid="true"]').forEach((input) => {
      input.removeAttribute("aria-invalid");
      input.removeAttribute("aria-describedby");
    });
    const field = problem?.field;
    if (!field || !stepNode) return;
    const name = CSS.escape(field);
    const input = stepNode.querySelector(`[data-field="${name}"], [data-rule-field="${name}"]`);
    if (!input) return;
    input.setAttribute("aria-invalid", "true");
    input.setAttribute("aria-describedby", "copy-editor-error");
    input.focus();
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
        mode === "edit"
          ? I18n.t("copy-editor-save-edit")
          : mode === "clone"
            ? I18n.t("copy-editor-save-clone")
            : I18n.t("copy-editor-save-create");
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

  function onInput(event) {
    // An input the user corrects is no longer marked as the problem.
    if (event.target.getAttribute("aria-invalid") === "true") {
      event.target.removeAttribute("aria-invalid");
      event.target.removeAttribute("aria-describedby");
    }
    if (!draft) return;
    const id = STEPS[step].id;
    if (!["wallet", "sizing", "exits"].includes(id)) return;
    readStep();
    if (id === "wallet") {
      const note = $("#copy-editor-duplicate");
      if (note) note.innerHTML = duplicateNote(context(), esc);
    } else if (id === "sizing") {
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
        notify("success", I18n.t("copy-editor-toast-updated"), taskName(source));
      } else {
        const response = await api.create(input());
        dialogs.hide("copy-editor");
        notify(
          "success",
          mode === "clone"
            ? I18n.t("copy-editor-toast-clone")
            : I18n.t("copy-editor-toast-created"),
          taskName(response.task)
        );
        state.view = "task";
        state.tab = "overview";
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
