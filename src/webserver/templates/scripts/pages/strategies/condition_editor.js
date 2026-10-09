// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Condition Editor Module
 * Handles the vertical card-based condition editor for building strategies
 */

import { formatFixed, formatList } from "../../core/format.js";
import {
  categoryLabel,
  conditionDescription,
  conditionName,
  formatParamValue,
  inheritedParamText,
  inputUnitText,
  optionLabel,
  optionValue,
  paramDescription,
  paramLabel,
  resolvedParam,
} from "./condition_text.js";

export function createConditionEditor({
  state,
  conditions,
  conditionSchemas,
  $,
  $$,
  Utils,
  renderStateView,
  announce,
  confirm,
  enhanceAllSelects,
  addTrackedListener,
  clearScope,
  CleanupScope,
}) {
  /**
   * Render the list of condition cards in the editor
   */
  function renderConditionsList() {
    const list = $("#conditions-list");
    if (!list) return;
    if (!conditions.length) {
      list.innerHTML = renderStateView({
        icon: "icon-puzzle",
        title: I18n.t("strategies-conditions-empty-title"),
        message: I18n.t("strategies-conditions-empty-hint"),
        compact: true,
      });
      return;
    }

    clearScope(CleanupScope.CONDITION_CARDS);

    list.innerHTML = conditions.map((c, idx) => renderConditionCard(c, idx)).join("");

    // Enhance native selects with custom styling
    enhanceAllSelects(list);

    // Wire card header click to expand/collapse (except when clicking on interactive elements)
    $$(".condition-card .card-header").forEach((header) => {
      const card = header.closest(".condition-card");
      const index = parseInt(card.dataset.index, 10);
      const handler = (e) => {
        // Don't toggle if clicking on checkbox, button, or action button
        if (
          e.target.closest("input") ||
          e.target.closest("button") ||
          e.target.closest(".condition-actions")
        ) {
          return;
        }
        toggleCardExpand(index);
      };
      addTrackedListener(header, "click", handler, CleanupScope.CONDITION_CARDS);
    });

    // Wire actions with cleanup tracking
    $$(".condition-card [data-action]").forEach((btn) => {
      const action = btn.dataset.action;
      const index = parseInt(btn.closest(".condition-card").dataset.index, 10);
      let handler;
      if (action === "toggle-expand") {
        handler = () => toggleCardExpand(index);
      } else if (action === "delete") {
        handler = () => deleteCondition(index);
      } else if (action === "duplicate") {
        handler = () => duplicateCondition(index);
      } else if (action === "move-up") {
        handler = () => moveCondition(index, -1);
      } else if (action === "move-down") {
        handler = () => moveCondition(index, 1);
      }
      if (handler) {
        addTrackedListener(btn, "click", handler, CleanupScope.CONDITION_CARDS);
      }
    });

    // Toggles and param inputs with cleanup tracking
    $$(".condition-card .toggle-enabled").forEach((el) => {
      const handler = (e) => {
        const idx = parseInt(el.closest(".condition-card").dataset.index, 10);
        const card = el.closest(".condition-card");
        conditions[idx].enabled = e.target.checked;

        // Update card status class
        if (e.target.checked) {
          card.classList.remove("status-disabled");
          card.classList.add("status-enabled");
        } else {
          card.classList.remove("status-enabled");
          card.classList.add("status-disabled");
        }

        updateRuleTreeFromEditor();
      };
      addTrackedListener(el, "change", handler, CleanupScope.CONDITION_CARDS);
    });

    // Param inputs with cleanup tracking
    $$(".condition-card .param-field input, .condition-card .param-field select").forEach(
      (input) => {
        const handler = () => {
          const card = input.closest(".condition-card");
          const idx = parseInt(card.dataset.index, 10);
          const key = input.dataset.key;
          const schema = conditionSchemas[conditions[idx].type];
          const spec = schema.parameters?.[key] || {};
          let value = input.value;
          if (spec.type === "number" || spec.type === "percent" || spec.type === "sol")
            value = parseFloat(value);
          if (spec.type === "boolean") value = input.checked;
          if (spec.type === "enum" && spec.optional && value === "") value = null;
          conditions[idx].params[key] = value;
          updateRuleTreeFromEditor();
          // Update summary text
          const summaryContent = card.querySelector(".summary-content");
          if (summaryContent) summaryContent.textContent = buildConditionSummary(conditions[idx]);
        };
        addTrackedListener(input, "change", handler, CleanupScope.CONDITION_CARDS);
      }
    );
  }

  /** Icon-only action button of a condition card; `enabled` false renders it disabled. */
  function iconButton(action, icon, title, enabled = true) {
    const text = Utils.escapeHtml(title);
    return `<button type="button" class="btn-icon" data-action="${action}" title="${text}" aria-label="${text}"${enabled ? "" : " disabled"}><i class="${icon}"></i></button>`;
  }

  /**
   * Render a single condition card
   */
  function renderConditionCard(c, idx) {
    const schema = conditionSchemas?.[c.type] || {};
    const iconClass = schema.icon || getConditionIcon(c.type);
    const category = schema.category || "General";
    const name = conditionName(schema, c.type);
    const description = conditionDescription(schema);
    const summary = buildConditionSummary(c);
    const body = renderParamEditor(c, schema, idx);
    const statusClass = c.enabled ? "status-enabled" : "status-disabled";
    const categorySlug = category.toLowerCase().replace(/[^a-z0-9]+/g, "-");

    return `
      <div class="condition-card ${statusClass}" data-index="${idx}" data-category="${categorySlug}">
        <div class="card-header">
          <div class="card-header-left">
            <div class="condition-icon">
              <i class="${iconClass}"></i>
            </div>
            <div class="condition-info">
              <div class="condition-name">
                ${Utils.escapeHtml(name)}
                <span class="condition-category-badge category-${categorySlug}">${Utils.escapeHtml(categoryLabel(schema))}</span>
              </div>
              <div class="condition-description">${Utils.escapeHtml(description)}</div>
            </div>
          </div>
          <div class="card-header-right">
            <div class="condition-status">
              <label class="toggle" title="${Utils.escapeHtml(c.enabled ? I18n.t("common-state-enabled") : I18n.t("common-state-disabled"))}">
                <input type="checkbox" class="toggle-enabled" ${c.enabled ? "checked" : ""}/>
                <span class="toggle-track"></span>
              </label>
            </div>
            <div class="condition-actions">
              ${iconButton("move-up", "icon-chevron-up", I18n.attr("strategies-card-move-up", "title"), idx > 0)}
              ${iconButton("move-down", "icon-chevron-down", I18n.attr("strategies-card-move-down", "title"), idx < conditions.length - 1)}
              ${iconButton("duplicate", "icon-copy", I18n.attr("strategies-card-duplicate", "title"))}
              ${iconButton("delete", "icon-trash-2", I18n.attr("strategies-card-delete", "title"))}
            </div>
            <span class="expand-indicator"><i class="icon-chevron-down"></i></span>
          </div>
        </div>
        <div class="card-summary">
          <div class="summary-content">${Utils.escapeHtml(summary)}</div>
        </div>
        <div class="card-body">${body}</div>
      </div>
    `;
  }

  /**
   * Toggle card expanded/collapsed state
   */
  function toggleCardExpand(index) {
    const card = document.querySelector(`.condition-card[data-index="${index}"]`);
    if (card) card.classList.toggle("expanded");
  }

  /**
   * One-line summary of a condition: every schema parameter in editor order, with the
   * same resolved value the editor shows (saved value, else default, else the
   * strategy's own value for an unset optional parameter).
   */
  function buildConditionSummary(c) {
    const params = conditionSchemas?.[c.type]?.parameters || {};
    // A lookback given as `time_value` + `time_unit` is one "Period" entry.
    const hasPeriod = Object.hasOwn(params, "time_value") && Object.hasOwn(params, "time_unit");

    const parts = Object.entries(params).flatMap(([key, spec]) => {
      if (hasPeriod && key === "time_unit") return [];
      if (hasPeriod && key === "time_value") {
        return [
          periodText(
            resolvedParam(c.params, "time_value", spec),
            resolvedParam(c.params, "time_unit", params.time_unit)
          ),
        ];
      }
      const value = resolvedParam(c.params, key, spec);
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
    const amount = formatFixed(Number(value), { decimals: 4, trim: true });
    switch (unit) {
      case "SECONDS":
        return I18n.t("strategies-summary-period-seconds", { amount });
      case "MINUTES":
        return I18n.t("strategies-summary-period-minutes", { amount });
      case "HOURS":
        return I18n.t("strategies-summary-period-hours", { amount });
      default:
        return amount;
    }
  }

  /**
   * Render parameter editor section for a condition
   */
  function renderParamEditor(c, schema, idx) {
    const params = schema.parameters || {};
    const entries = Object.entries(params);
    if (!entries.length)
      return `<div class="param-row">${Utils.escapeHtml(I18n.t("strategies-summary-none"))}</div>`;
    // A lookback given as `time_value` + `time_unit` is one duration field.
    const hasPeriod = Object.hasOwn(params, "time_value") && Object.hasOwn(params, "time_unit");
    const shown = hasPeriod ? entries.filter(([key]) => key !== "time_unit") : entries;
    const fields = shown.map(([key, spec]) => {
      const label = paramLabel(spec, key);
      const description = paramDescription(spec);
      const val = resolvedParam(c.params, key, spec) ?? "";
      const unit = params.time_unit;
      const control =
        hasPeriod && key === "time_value"
          ? renderPeriodInput(idx, spec, val, unit, resolvedParam(c.params, "time_unit", unit))
          : renderParamInput(idx, key, spec, val);
      return `
        <div class="param-field">
          <label>${Utils.escapeHtml(label)}</label>
          ${control}
          ${description ? `<div class="property-description">${Utils.escapeHtml(description)}</div>` : ""}
        </div>
      `;
    });
    return `<div class="param-row">${fields.join("")}</div>`;
  }

  /**
   * A lookback as one duration field: the amount, with its unit picked inside the
   * field. The unit select is the field's `.input-unit`, which `ui/number_field.js`
   * adopts into the numeric shell; both keep their own `data-key`, so the stored
   * `time_value` / `time_unit` pair is unchanged.
   */
  function renderPeriodInput(idx, valueSpec, value, unitSpec, unit) {
    const amount = renderParamInput(idx, "time_value", valueSpec, value);
    const unitLabel = Utils.escapeHtml(paramLabel(unitSpec, "time_unit"));
    return `${amount}
          <span class="input-unit"><select id="param-${idx}-time_unit" data-key="time_unit" aria-label="${unitLabel}" data-custom-select data-cs-fit-options>${renderParamOptions("time_unit", unitSpec, unit ?? "")}</select></span>`;
  }

  /** The `<option>` list of an enum parameter, with the unset choice of an optional one. */
  function renderParamOptions(key, spec, value) {
    const options = spec.options || spec.values || [];
    // An unset optional parameter is a choice of its own: the strategy supplies it.
    const inherit = spec.optional
      ? `<option value="" ${value === "" ? "selected" : ""}>${Utils.escapeHtml(inheritedParamText(spec, state.currentStrategy?.[key]))}</option>`
      : "";
    return (
      inherit +
      options
        .map((opt) => {
          const optValue = optionValue(opt);
          const selected = optValue === value ? "selected" : "";
          return `<option value="${Utils.escapeHtml(String(optValue))}" ${selected}>${Utils.escapeHtml(optionLabel(opt))}</option>`;
        })
        .join("")
    );
  }

  /**
   * Render input control for a single parameter
   */
  function renderParamInput(idx, key, spec, value) {
    const id = `param-${idx}-${key}`;
    const data = `data-key="${key}"`;
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
        const input = `<input id="${id}" ${data} type="number" value="${value}" ${min} ${max} ${step} placeholder="0">`;
        return unit ? `${input}\n          <span class="input-unit">${Utils.escapeHtml(unit)}</span>` : input;
      }
      case "boolean":
        return `<label class="toggle">
          <input id="${id}" ${data} type="checkbox" ${value ? "checked" : ""}>
          <span class="toggle-track"></span>
        </label>`;
      case "enum":
        return `<select id="${id}" ${data} class="select-field" data-custom-select>${renderParamOptions(key, spec, value)}</select>`;
      default:
        return `<input id="${id}" ${data} type="text" value="${Utils.escapeHtml(String(value))}">`;
    }
  }

  /**
   * Update the strategy's rule tree from the current editor state
   */
  function updateRuleTreeFromEditor() {
    if (!state.currentStrategy) return;
    if (conditions.length === 0) {
      state.currentStrategy.rules = null;
      return;
    }
    const condNodes = conditions
      .filter((c) => c.enabled)
      .map((c) => {
        const schema = conditionSchemas?.[c.type] || { parameters: {} };
        const params = {};
        Object.entries(schema.parameters || {}).forEach(([k, spec]) => {
          params[k] = { value: resolvedParam(c.params, k, spec), default: spec?.default };
        });
        return { condition: { type: c.type, parameters: params } };
      });
    if (condNodes.length === 1) state.currentStrategy.rules = condNodes[0];
    else state.currentStrategy.rules = { operator: "AND", conditions: condNodes };
  }

  /**
   * Add a new condition to the editor
   */
  function addCondition(conditionType) {
    const schema = conditionSchemas?.[conditionType];
    if (!schema) {
      return announce(
        "error",
        I18n.t("strategies-toast-unknown-condition"),
        I18n.attr("strategies-toast-unknown-condition", "message")
      );
    }

    // Auto-create strategy if none exists (first condition added)
    // Show modal to select type first
    if (!state.currentStrategy) {
      announce(
        "warning",
        I18n.t("strategies-toast-create-first"),
        I18n.attr("strategies-toast-create-first", "message")
      );
      return;
    }

    const params = {};
    Object.entries(schema.parameters || {}).forEach(([k, p]) => {
      params[k] = p.default ?? null;
    });
    conditions.push({
      type: conditionType,
      enabled: true,
      params,
    });
    renderConditionsList();
    updateRuleTreeFromEditor();
    const args = { name: conditionName(schema, conditionType) };
    announce(
      "success",
      I18n.t("strategies-toast-condition-added", args),
      I18n.attr("strategies-toast-condition-added", "message", args)
    );
  }

  /** Remove a condition from the editor once the removal is confirmed. */
  async function deleteCondition(index) {
    const condition = conditions[index];
    if (!condition) return;
    const args = { name: conditionName(conditionSchemas?.[condition.type], condition.type) };
    const { confirmed } = await confirm({
      title: I18n.t("strategies-card-delete-confirm"),
      message: I18n.attr("strategies-card-delete-confirm", "message", args),
      confirmLabel: I18n.t("common-action-remove"),
      cancelLabel: I18n.t("common-action-cancel"),
      variant: "danger",
    });
    if (!confirmed || conditions[index] !== condition) return;
    conditions.splice(index, 1);
    renderConditionsList();
    updateRuleTreeFromEditor();
  }

  /**
   * Move a condition up or down in the list
   */
  function moveCondition(index, delta) {
    const newIndex = index + delta;
    if (newIndex < 0 || newIndex >= conditions.length) return;
    const [item] = conditions.splice(index, 1);
    conditions.splice(newIndex, 0, item);
    renderConditionsList();
    updateRuleTreeFromEditor();
  }

  /**
   * Duplicate a condition
   */
  function duplicateCondition(index) {
    const copy = JSON.parse(JSON.stringify(conditions[index]));
    conditions.splice(index + 1, 0, copy);
    renderConditionsList();
    updateRuleTreeFromEditor();
  }

  /**
   * Parse a rule tree into the flat conditions array for editing
   */
  function parseRuleTreeToConditions(rules) {
    conditions.length = 0; // Clear array (mutable reference)
    if (!rules) return;
    const leafs = [];
    function walk(node) {
      if (!node) return;
      if (node.condition) {
        leafs.push(node.condition);
        return;
      }
      (node.conditions || []).forEach((c) => walk(c));
    }
    walk(rules);
    leafs.forEach((cond) => {
      const schema = conditionSchemas?.[cond.type] || { parameters: {} };
      const params = {};
      Object.keys(schema.parameters || {}).forEach((k) => {
        const p = cond.parameters?.[k];
        params[k] =
          p && typeof p === "object" && "value" in p
            ? p.value
            : (schema.parameters[k]?.default ?? null);
      });
      conditions.push({
        type: cond.type,
        enabled: true,
        params,
      });
    });
  }

  /**
   * Get icon for a specific condition type
   */
  function getConditionIcon(type) {
    const icons = {
      PriceChangePercent: "icon-percent",
      PriceToMa: "icon-chart-line",
      LiquidityLevel: "icon-droplet",
      PriceBreakout: "icon-rocket",
      PositionHoldingTime: "icon-hourglass",
      CandleSize: "icon-expand",
      ConsecutiveCandles: "icon-chart-candlestick",
      VolumeSpike: "icon-chart-bar",
    };
    return icons[type] || "icon-puzzle";
  }

  return {
    renderConditionsList,
    renderConditionCard,
    buildConditionSummary,
    renderParamEditor,
    renderParamInput,
    updateRuleTreeFromEditor,
    addCondition,
    deleteCondition,
    moveCondition,
    duplicateCondition,
    parseRuleTreeToConditions,
    toggleCardExpand,
    getConditionIcon,
  };
}
