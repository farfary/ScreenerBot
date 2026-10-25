// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Strategy rule editor - the rule tree as nested groups and condition rows with parameter validation, over the pure model in rule_tree.js.

import { formatFixed, formatList } from "../../core/format.js";
import {
  conditionName,
  formatParamValue,
  inheritedParamText,
  inputUnitText,
  optionLabel,
  optionValue,
  paramDescription,
  paramLabel,
} from "./condition_text.js";
import * as tree from "./rule_tree.js";

/** Stands in for the match select while a sentence is localized; the select takes its place. */
const MODE_MARK = "\u0001";
const ISOLATES = /[⁨⁩]/g;

export function createRuleEditor({
  state,
  conditionSchemas,
  $,
  Utils,
  renderStateView,
  announce,
  confirm,
  enhanceAllSelects,
  addTrackedListener,
  onChange,
  openCatalog,
}) {
  let root = tree.parseRules(null, conditionSchemas);
  const open = new Set();
  let pendingFocus = null;

  const escape = (text) => Utils.escapeHtml(String(text ?? ""));
  const strategyType = () => state.currentStrategy?.type;
  const list = () => $("#conditions-list");

  function problems() {
    return tree.problemsOf(root, conditionSchemas, strategyType());
  }

  /** Display name of a condition node. */
  function nameOf(node) {
    return conditionName(conditionSchemas?.[node.type], node.type);
  }

  function amount(value) {
    return formatFixed(value, { decimals: 4, trim: true });
  }

  /** Message of a problem without saying where it is. */
  function problemText(problem) {
    switch (problem.code) {
      case "rules-empty":
        return I18n.t("strategies-problem-rules-empty");
      case "group-empty":
        return I18n.t("strategies-problem-group-empty");
      case "not-applicable":
        return I18n.t("strategies-problem-not-applicable");
      case "unknown-condition":
        return I18n.t("strategies-problem-unknown-condition", { type: problem.args.type });
      case "too-deep":
        return I18n.t("strategies-problem-too-deep", { max: problem.args.max });
      case "required":
        return I18n.t("strategies-problem-required");
      case "range":
        return I18n.t("strategies-problem-range", {
          min: amount(problem.args.min),
          max: amount(problem.args.max),
        });
      case "invalid-option":
        return I18n.t("strategies-problem-invalid-option");
      default:
        return problem.code;
    }
  }

  /** Message of a problem led by the condition and parameter it belongs to. */
  function describeProblem(problem) {
    const node = tree.findNode(root, problem.nodeId);
    if (node?.kind !== "condition") return problemText(problem);
    const args = { name: nameOf(node), problem: problemText(problem) };
    if (!problem.key) return I18n.t("strategies-problem-located", args);
    const spec = conditionSchemas?.[node.type]?.parameters?.[problem.key];
    return I18n.t("strategies-problem-located-field", {
      ...args,
      field: paramLabel(spec, problem.key),
    });
  }

  /** Whether a problem belongs to a node itself rather than to one of its parameters. */
  const isNodeProblem = (problem) => !problem.key;

  // --- Rendering ---------------------------------------------------------------

  function iconButton(action, icon, title, { disabled = false, pressed = null } = {}) {
    const text = escape(title);
    const toggled = pressed === null ? "" : ` aria-pressed="${pressed}"`;
    return `<button type="button" class="btn-icon" data-action="${action}" title="${text}" aria-label="${text}"${toggled}${disabled ? " disabled" : ""}><i class="${icon}"></i></button>`;
  }

  function renderChips(node) {
    const chips = [];
    if (node.disabled) {
      chips.push(`<span class="rule-chip">${escape(I18n.t("strategies-chip-off"))}</span>`);
    }
    if (node.inverted) {
      chips.push(`<span class="rule-chip">${escape(I18n.t("strategies-chip-inverted"))}</span>`);
    }
    return chips.length ? `<span class="rule-chips">${chips.join("")}</span>` : "";
  }

  /** Switch, invert, duplicate, move and remove of a condition row or a nested group. */
  function renderActions(node, siblings) {
    const index = siblings.indexOf(node);
    const invertDisabled = !node.inverted && !tree.canInvert(root, node.id);
    return `<div class="rule-actions">
        <label class="toggle rule-switch" title="${escape(I18n.attr("strategies-row-switch", "title"))}">
          <input type="checkbox" data-action="switch" aria-label="${escape(I18n.attr("strategies-row-switch", "aria-label"))}"${node.disabled ? "" : " checked"}>
          <span class="toggle-track"></span>
        </label>
        ${iconButton("invert", "icon-flip-vertical-2", I18n.attr("strategies-invert", "title"), { pressed: node.inverted, disabled: invertDisabled })}
        ${iconButton("duplicate", "icon-copy", I18n.attr("strategies-card-duplicate", "title"))}
        ${iconButton("move-up", "icon-chevron-up", I18n.attr("strategies-card-move-up", "title"), { disabled: index <= 0 })}
        ${iconButton("move-down", "icon-chevron-down", I18n.attr("strategies-card-move-down", "title"), { disabled: index >= siblings.length - 1 })}
        ${iconButton("delete", "icon-trash-2", I18n.attr("strategies-card-delete", "title"))}
      </div>`;
  }

  /** One-line summary of a condition: every schema parameter in editor order. */
  function buildConditionSummary(node) {
    const params = conditionSchemas?.[node.type]?.parameters || {};
    // A lookback given as `time_value` + `time_unit` is one "Period" entry.
    const hasPeriod = Object.hasOwn(params, "time_value") && Object.hasOwn(params, "time_unit");

    const parts = Object.entries(params).flatMap(([key, spec]) => {
      if (hasPeriod && key === "time_unit") return [];
      if (hasPeriod && key === "time_value") {
        return [periodText(node.params.time_value, node.params.time_unit)];
      }
      const value = node.params[key];
      return [
        I18n.t("strategies-summary-param", {
          label: paramLabel(spec, key),
          value:
            value === null && spec.optional
              ? inheritedParamText(spec, state.currentStrategy?.[key])
              : formatParamValue(value, spec),
        }),
      ];
    });

    return parts.length ? formatList(parts, { type: "unit" }) : I18n.t("strategies-summary-none");
  }

  /** A lookback amount and its unit as one summary entry. */
  function periodText(value, unit) {
    // Ids are the `time_unit` values of the price change condition.
    const text = value === null ? "—" : amount(Number(value));
    switch (unit) {
      case "SECONDS":
        return I18n.t("strategies-summary-period-seconds", { amount: text });
      case "MINUTES":
        return I18n.t("strategies-summary-period-minutes", { amount: text });
      case "HOURS":
        return I18n.t("strategies-summary-period-hours", { amount: text });
      default:
        return text;
    }
  }

  /** The `<option>` list of an enum parameter, with the unset choice of an optional one. */
  function renderParamOptions(key, spec, value) {
    const options = spec.options || spec.values || [];
    const current = value ?? "";
    // An unset optional parameter is a choice of its own: the strategy supplies it.
    const inherit = spec.optional
      ? `<option value="" ${current === "" ? "selected" : ""}>${escape(inheritedParamText(spec, state.currentStrategy?.[key]))}</option>`
      : "";
    return (
      inherit +
      options
        .map((opt) => {
          const optValue = optionValue(opt);
          const selected = optValue === current ? "selected" : "";
          return `<option value="${escape(optValue)}" ${selected}>${escape(optionLabel(opt))}</option>`;
        })
        .join("")
    );
  }

  /** Input control for a single parameter; `invalid` marks it as the field in error. */
  function renderParamInput(node, key, spec, value, { invalid = false, errorId = "" } = {}) {
    const id = `param-${node.id}-${key}`;
    const data = `data-key="${key}"`;
    const flags = invalid ? ` aria-invalid="true" aria-describedby="${errorId}"` : "";
    const min = spec.min !== undefined ? `min="${spec.min}"` : "";
    const max = spec.max !== undefined ? `max="${spec.max}"` : "";
    const step = spec.step !== undefined ? `step="${spec.step}"` : "";

    switch (spec.type) {
      // A unit is written straight after its input — `ui/number_field.js` builds
      // the numeric shell around both, so no wrapper of our own is needed here.
      case "percent":
      case "sol":
      case "number": {
        const unit = inputUnitText(spec);
        const shown = value === null || value === undefined ? "" : value;
        const input = `<input id="${id}" ${data} type="number" value="${escape(shown)}" ${min} ${max} ${step} placeholder="0"${flags}>`;
        return unit ? `${input}\n          <span class="input-unit">${escape(unit)}</span>` : input;
      }
      case "boolean":
        return `<label class="toggle">
          <input id="${id}" ${data} type="checkbox" ${value ? "checked" : ""}>
          <span class="toggle-track"></span>
        </label>`;
      case "enum":
        return `<select id="${id}" ${data} class="select-field" data-custom-select${flags}>${renderParamOptions(key, spec, value)}</select>`;
      default:
        return `<input id="${id}" ${data} type="text" value="${escape(value ?? "")}"${flags}>`;
    }
  }

  /**
   * A lookback as one duration field: the amount, with its unit picked inside the
   * field. The unit select is the field's `.input-unit`, which `ui/number_field.js`
   * adopts into the numeric shell; both keep their own `data-key`, so the stored
   * `time_value` / `time_unit` pair is unchanged.
   */
  function renderPeriodInput(node, valueSpec, unitSpec, flags) {
    const amountInput = renderParamInput(node, "time_value", valueSpec, node.params.time_value, flags);
    const unitLabel = escape(paramLabel(unitSpec, "time_unit"));
    return `${amountInput}
          <span class="input-unit"><select id="param-${node.id}-time_unit" data-key="time_unit" aria-label="${unitLabel}" data-custom-select data-cs-fit-options>${renderParamOptions("time_unit", unitSpec, node.params.time_unit)}</select></span>`;
  }

  /** Parameter fields of a condition, each with the error line of its first problem. */
  function renderParamEditor(node) {
    const params = conditionSchemas?.[node.type]?.parameters || {};
    const entries = Object.entries(params);
    if (!entries.length) {
      return `<div class="param-row">${escape(I18n.t("strategies-summary-none"))}</div>`;
    }
    // A lookback given as `time_value` + `time_unit` is one duration field.
    const hasPeriod = Object.hasOwn(params, "time_value") && Object.hasOwn(params, "time_unit");
    const shown = hasPeriod ? entries.filter(([key]) => key !== "time_unit") : entries;
    const own = problems().filter((problem) => problem.nodeId === node.id && problem.key);
    const fields = shown.map(([key, spec]) => {
      const keys = hasPeriod && key === "time_value" ? ["time_value", "time_unit"] : [key];
      const problem = own.find((candidate) => keys.includes(candidate.key));
      const errorId = `param-${node.id}-${key}-error`;
      const flags = { invalid: Boolean(problem), errorId };
      const description = paramDescription(spec);
      const control =
        keys.length > 1
          ? renderPeriodInput(node, spec, params.time_unit, flags)
          : renderParamInput(node, key, spec, node.params[key], flags);
      return `
        <div class="param-field" data-field="${keys.join(" ")}">
          <label for="param-${node.id}-${key}">${escape(paramLabel(spec, key))}</label>
          ${control}
          ${description ? `<div class="property-description">${escape(description)}</div>` : ""}
          <div class="param-field-error" id="${errorId}">${problem ? escape(problemText(problem)) : ""}</div>
        </div>
      `;
    });
    return `<div class="param-row">${fields.join("")}</div>`;
  }

  function renderConditionRow(node, siblings, found = problems()) {
    const schema = conditionSchemas?.[node.type];
    const own = found.filter((problem) => problem.nodeId === node.id);
    const isOpen = open.has(node.id) && node.known;
    const classes = [
      "rule-row",
      node.disabled ? "is-off" : "",
      node.inverted ? "is-inverted" : "",
      isOpen ? "is-open" : "",
      own.length ? "has-problem" : "",
    ]
      .filter(Boolean)
      .join(" ");
    const noted = own.find(isNodeProblem);
    const label = escape(I18n.attr("strategies-row-expand", "aria-label"));
    return `
      <div class="${classes}" data-node-id="${node.id}" data-kind="condition">
        <div class="rule-head">
          <button type="button" class="rule-head-toggle" data-action="toggle" aria-expanded="${isOpen}" aria-controls="rule-body-${node.id}"${node.known ? "" : " disabled"}>
            <i class="rule-glyph ${escape(schema?.icon || "icon-puzzle")}" aria-hidden="true"></i>
            <span class="rule-name">${escape(nameOf(node))}</span>
            <span class="rule-summary summary-content">${node.known ? escape(buildConditionSummary(node)) : ""}</span>
            ${renderChips(node)}
          </button>
          ${renderActions(node, siblings)}
          <button type="button" class="btn-icon rule-chevron" data-action="toggle" tabindex="-1" aria-hidden="true" title="${label}"${node.known ? "" : " disabled"}><i class="icon-chevron-down"></i></button>
        </div>
        ${noted ? `<div class="rule-node-problem" role="alert">${escape(problemText(noted))}</div>` : ""}
        <div class="rule-body" id="rule-body-${node.id}"${isOpen ? "" : " hidden"}>${isOpen ? renderParamEditor(node) : ""}</div>
      </div>`;
  }

  /** Sentence "Match [all|any] of these ..." with the select where the mode word goes. */
  function renderMatchLine(node, sentence) {
    const [before, after = ""] = sentence.replace(ISOLATES, "").split(MODE_MARK);
    const label = escape(I18n.attr("strategies-match-mode", "aria-label"));
    const option = (value, text) =>
      `<option value="${value}"${node.match === value ? " selected" : ""}>${escape(text)}</option>`;
    const select = `<select class="rule-match-select" data-action="match" aria-label="${label}" data-custom-select data-cs-fit-options>${option("all", I18n.t("strategies-match-all"))}${option("any", I18n.t("strategies-match-any"))}</select>`;
    return `<span class="rule-match-line">${escape(before)}${select}${escape(after)}</span>`;
  }

  function renderChildren(group, found) {
    return group.children
      .map((child) =>
        child.kind === "group"
          ? renderGroup(child, group.children, found)
          : renderConditionRow(child, group.children, found)
      )
      .join("");
  }

  function renderGroup(group, siblings, found = problems()) {
    const isRoot = group === root;
    const own = found.filter((problem) => problem.nodeId === group.id);
    const classes = [
      "rule-group",
      isRoot ? "is-root" : "is-nested",
      group.disabled ? "is-off" : "",
      group.inverted ? "is-inverted" : "",
      own.length && !isRoot ? "has-problem" : "",
    ]
      .filter(Boolean)
      .join(" ");
    const sentence = isRoot
      ? I18n.t("strategies-rules-match", { mode: MODE_MARK })
      : I18n.t("strategies-group-match", { mode: MODE_MARK });
    const invert = isRoot
      ? `<button type="button" class="btn btn-sm rule-invert" data-action="invert" aria-pressed="${group.inverted}" title="${escape(I18n.attr("strategies-invert", "title"))}"${group.inverted || tree.canInvert(root, group.id) ? "" : " disabled"}><i class="icon-flip-vertical-2"></i> <span>${escape(I18n.t("strategies-invert"))}</span></button>`
      : renderActions(group, siblings);
    const noted = own.find(isNodeProblem);
    let children = renderChildren(group, found);
    if (!children && isRoot) {
      children = renderStateView({
        icon: "icon-puzzle",
        title: I18n.t("strategies-conditions-empty-title"),
        message: I18n.t("strategies-conditions-empty-hint"),
        compact: true,
      });
    }
    const add = isRoot
      ? ""
      : `<div class="rule-group-add">
          <button type="button" class="btn btn-sm dashed" data-action="add-condition"${tree.canAddCondition(root, group.id) ? "" : " disabled"}><i class="icon-plus"></i> <span>${escape(I18n.t("strategies-add-condition"))}</span></button>
          <button type="button" class="btn btn-sm dashed" data-action="add-group"${tree.canAddGroup(root, group.id) ? "" : " disabled"}><i class="icon-plus"></i> <span>${escape(I18n.t("strategies-add-group"))}</span></button>
        </div>`;
    return `
      <section class="${classes}" data-node-id="${group.id}" data-kind="group">
        <div class="rule-group-head">
          ${renderMatchLine(group, sentence)}
          ${isRoot ? "" : renderChips(group)}
          ${invert}
        </div>
        ${noted && !isRoot ? `<div class="rule-node-problem" role="alert">${escape(problemText(noted))}</div>` : ""}
        <div class="rule-children">${children}</div>
        ${add}
      </section>`;
  }

  /** The whole rule tree as markup. */
  function renderRules() {
    return renderGroup(root, [root]);
  }

  // --- DOM updates --------------------------------------------------------------

  const nodeEl = (id) => list()?.querySelector(`[data-node-id="${id}"]`);

  function syncFooter() {
    const found = problems();
    const bar = $("#rule-problems");
    if (bar) {
      bar.hidden = found.length === 0;
      if (found.length) {
        const text = I18n.t("strategies-problems", {
          count: found.length,
          first: describeProblem(found[0]),
        });
        bar.innerHTML = `<i class="icon-triangle-alert" aria-hidden="true"></i> <span>${escape(text)}</span>`;
      }
    }
    const addGroup = $("#add-group");
    if (addGroup) addGroup.disabled = !tree.canAddGroup(root, root.id);
  }

  function restoreFocus() {
    if (!pendingFocus) return;
    const { id, action, fallback } = pendingFocus;
    pendingFocus = null;
    const scope = nodeEl(id);
    const target =
      scope?.querySelector(`[data-action="${action}"]:not(:disabled)`) ??
      (fallback ? scope?.querySelector(`[data-action="${fallback}"]:not(:disabled)`) : null);
    target?.focus();
  }

  function render() {
    const container = list();
    if (!container) return;
    container.innerHTML = renderRules();
    enhanceAllSelects(container);
    restoreFocus();
    syncFooter();
  }

  /** Open or close the parameters of a condition row without redrawing the list. */
  function setOpen(id, value) {
    const node = tree.findNode(root, id);
    if (node?.kind !== "condition" || !node.known) return;
    if (value) open.add(id);
    else open.delete(id);
    const row = nodeEl(id);
    const body = row?.querySelector(":scope > .rule-body");
    if (!body) return;
    if (value && !body.firstElementChild) {
      body.innerHTML = renderParamEditor(node);
      enhanceAllSelects(body);
    }
    body.hidden = !value;
    row.classList.toggle("is-open", value);
    row.querySelector(".rule-head-toggle")?.setAttribute("aria-expanded", String(value));
  }

  /** Redraw the summary and field errors of one condition after a parameter edit. */
  function refreshNode(id) {
    const node = tree.findNode(root, id);
    const row = nodeEl(id);
    if (!node || !row) return;
    const summary = row.querySelector(".rule-summary");
    if (summary) summary.textContent = buildConditionSummary(node);
    const own = problems().filter((problem) => problem.nodeId === id);
    row.classList.toggle("has-problem", own.length > 0);
    row.querySelectorAll(".param-field").forEach((field) => {
      const keys = field.dataset.field.split(" ");
      const problem = own.find((candidate) => keys.includes(candidate.key));
      const error = field.querySelector(".param-field-error");
      if (error) error.textContent = problem ? problemText(problem) : "";
      field.querySelectorAll("[data-key]").forEach((control) => {
        if (problem && control.dataset.key === problem.key) {
          control.setAttribute("aria-invalid", "true");
          control.setAttribute("aria-describedby", error?.id ?? "");
        } else {
          control.removeAttribute("aria-invalid");
          control.removeAttribute("aria-describedby");
        }
      });
    });
    syncFooter();
  }

  /** Bring a problem into view: open its row, scroll to it and focus the field in error. */
  function revealProblem(problem) {
    const node = tree.findNode(root, problem.nodeId);
    if (!node) return;
    if (problem.key) setOpen(node.id, true);
    const el = nodeEl(node.id);
    el?.scrollIntoView?.({ block: "center" });
    const field = problem.key ? el?.querySelector(`[data-key="${problem.key}"]`) : null;
    const target = field?.closest(".param-field")?.querySelector(".custom-select") ?? field;
    (target ?? el?.querySelector("button, input, select"))?.focus?.();
  }

  /** True when the strategy can be saved; otherwise the first problem is brought into view. */
  function checkBeforeSave() {
    const found = problems();
    if (!found.length) return true;
    revealProblem(found[0]);
    return false;
  }

  // --- Edits --------------------------------------------------------------------

  function changed() {
    onChange?.();
  }

  /** Add a condition to a group from the catalog, open it and focus its first field. */
  function addConditionTo(groupId, type) {
    const schema = conditionSchemas?.[type];
    if (!schema) {
      return announce(
        "error",
        I18n.t("strategies-toast-unknown-condition"),
        I18n.attr("strategies-toast-unknown-condition", "message")
      );
    }
    if (!state.currentStrategy) {
      return announce(
        "warning",
        I18n.t("strategies-toast-create-first"),
        I18n.attr("strategies-toast-create-first", "message")
      );
    }
    const id = tree.addCondition(root, groupId, type, conditionSchemas);
    if (id === false) {
      return announce(
        "warning",
        I18n.t("strategies-toast-too-deep"),
        I18n.attr("strategies-toast-too-deep", "message", { max: tree.MAX_DEPTH })
      );
    }
    open.add(id);
    render();
    changed();
    const body = nodeEl(id)?.querySelector(".rule-body");
    const first = body?.querySelector(".param-field input, .param-field .custom-select");
    first?.focus?.();
    nodeEl(id)?.scrollIntoView?.({ block: "nearest" });
    const args = { name: conditionName(schema, type) };
    return announce(
      "success",
      I18n.t("strategies-toast-condition-added", args),
      I18n.attr("strategies-toast-condition-added", "message", args)
    );
  }

  function addGroupTo(groupId) {
    const id = tree.addGroup(root, groupId);
    if (id === false) return;
    pendingFocus = { id, action: "add-condition" };
    render();
    changed();
  }

  function conditionCount(node) {
    return node.kind === "condition"
      ? 1
      : node.children.reduce((sum, child) => sum + conditionCount(child), 0);
  }

  /** Remove a row or a group once the removal is confirmed. */
  async function removeNode(id) {
    const node = tree.findNode(root, id);
    if (!node || node === root) return;
    const isGroup = node.kind === "group";
    const { confirmed } = await confirm(
      isGroup
        ? {
            title: I18n.t("strategies-group-delete-confirm"),
            message: I18n.attr("strategies-group-delete-confirm", "message", {
              count: conditionCount(node),
            }),
            confirmLabel: I18n.t("common-action-remove"),
            cancelLabel: I18n.t("common-action-cancel"),
            variant: "danger",
          }
        : {
            title: I18n.t("strategies-card-delete-confirm"),
            message: I18n.attr("strategies-card-delete-confirm", "message", {
              name: nameOf(node),
            }),
            confirmLabel: I18n.t("common-action-remove"),
            cancelLabel: I18n.t("common-action-cancel"),
            variant: "danger",
          }
    );
    if (!confirmed || tree.findNode(root, id) !== node) return;
    tree.removeNode(root, id);
    open.delete(id);
    render();
    changed();
  }

  function moveBy(id, delta) {
    const [action, fallback] = delta < 0 ? ["move-up", "move-down"] : ["move-down", "move-up"];
    if (tree.moveNode(root, id, delta) === false) return;
    pendingFocus = { id, action, fallback };
    render();
    changed();
  }

  /** Act on a row or group button; every action leaves the focus where the user was. */
  function runAction(action, id) {
    switch (action) {
      case "toggle":
        setOpen(id, !open.has(id));
        break;
      case "invert": {
        const node = tree.findNode(root, id);
        if (node && tree.setInverted(root, id, !node.inverted) !== false) {
          pendingFocus = { id, action: "invert" };
          render();
          changed();
        }
        break;
      }
      case "duplicate": {
        const copy = tree.duplicateNode(root, id);
        if (copy !== false) {
          pendingFocus = { id: copy, action: "duplicate" };
          render();
          changed();
        }
        break;
      }
      case "move-up":
        moveBy(id, -1);
        break;
      case "move-down":
        moveBy(id, 1);
        break;
      case "delete":
        removeNode(id);
        break;
      case "add-condition":
        openCatalog?.(id);
        break;
      case "add-group":
        addGroupTo(id);
        break;
      default:
        break;
    }
  }

  function parseValue(control, spec) {
    if (control.type === "checkbox") return control.checked;
    if (spec.type === "number" || spec.type === "percent" || spec.type === "sol") {
      return control.value === "" ? null : Number(control.value);
    }
    if (spec.type === "enum" && spec.optional && control.value === "") return null;
    return control.value;
  }

  function editParam(control) {
    const id = Number(control.closest("[data-node-id]")?.dataset.nodeId);
    const node = tree.findNode(root, id);
    const key = control.dataset.key;
    const spec = conditionSchemas?.[node?.type]?.parameters?.[key];
    if (!spec || tree.setParam(root, id, key, parseValue(control, spec)) === false) return;
    refreshNode(id);
    changed();
  }

  const idOf = (element) => Number(element.closest("[data-node-id]")?.dataset.nodeId);

  /** Wire the list and the footer; the list's own markup is replaced on every render. */
  function bind() {
    const container = list();
    if (container) {
      addTrackedListener(container, "click", (event) => {
        const button = event.target.closest("button[data-action]");
        if (button && !button.disabled) runAction(button.dataset.action, idOf(button));
      });
      addTrackedListener(container, "input", (event) => {
        const control = event.target;
        if (control.dataset?.key && control.type !== "checkbox" && control.tagName !== "SELECT") {
          editParam(control);
        }
      });
      addTrackedListener(container, "change", (event) => {
        const control = event.target;
        const action = control.dataset?.action;
        if (action === "switch") {
          const id = idOf(control);
          if (tree.setDisabled(root, id, !control.checked) !== false) {
            pendingFocus = { id, action: "switch" };
            render();
            changed();
          }
        } else if (action === "match") {
          if (tree.setMatch(root, idOf(control), control.value) !== false) changed();
        } else if (
          control.dataset?.key &&
          (control.type === "checkbox" || control.tagName === "SELECT")
        ) {
          editParam(control);
        }
      });
    }
    addTrackedListener($("#add-condition"), "click", () => openCatalog?.(root.id));
    addTrackedListener($("#add-group"), "click", () => addGroupTo(root.id));
    addTrackedListener($("#rule-problems"), "click", () => {
      const [first] = problems();
      if (first) revealProblem(first);
    });
  }

  return {
    bind,
    load(rules) {
      root = tree.parseRules(rules, conditionSchemas);
      open.clear();
      pendingFocus = null;
      render();
    },
    serialize: () => tree.serializeRules(root, conditionSchemas),
    model: () => root,
    problems,
    describeProblem,
    checkBeforeSave,
    revealProblem,
    render,
    renderRules,
    renderConditionRow,
    renderGroup,
    renderParamEditor,
    buildConditionSummary,
    setOpen,
    addConditionTo,
    addGroupTo,
    removeNode,
    runAction,
  };
}
