// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Condition Editor Module
 * Handles the vertical card-based condition editor for building strategies
 */

import { formatFixed } from "../../core/format.js";
import {
  categoryLabel,
  conditionDescription,
  conditionName,
  formatParamValue,
  inputUnitText,
  optionLabel,
  optionValue,
  paramDescription,
  paramLabel,
} from "./condition_text.js";

export function createConditionEditor({
  state,
  conditions,
  conditionSchemas,
  $,
  $$,
  Utils,
  announce,
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
      list.innerHTML = `<div class="empty-state"><i class="icon-puzzle"></i><p>${Utils.escapeHtml(I18n.t("strategies-conditions-empty-title"))}</p><small>${Utils.escapeHtml(I18n.t("strategies-conditions-empty-hint"))}</small></div>`;
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

  /** Icon-only action button of a condition card. */
  function iconButton(action, icon, title) {
    const text = Utils.escapeHtml(title);
    return `<button class="btn-icon" data-action="${action}" title="${text}" aria-label="${text}"><i class="${icon}"></i></button>`;
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
              <label class="toggle" data-level="item" title="${Utils.escapeHtml(c.enabled ? I18n.t("common-state-enabled") : I18n.t("common-state-disabled"))}">
                <input type="checkbox" class="toggle-enabled" ${c.enabled ? "checked" : ""}/>
                <span class="toggle-track"></span>
              </label>
            </div>
            <div class="condition-actions">
              ${iconButton("move-up", "icon-chevron-up", I18n.attr("strategies-card-move-up", "title"))}
              ${iconButton("move-down", "icon-chevron-down", I18n.attr("strategies-card-move-down", "title"))}
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
   * Build human-readable summary of condition parameters
   */
  function buildConditionSummary(c) {
    const schema = conditionSchemas?.[c.type] || {};
    const params = schema.parameters || {};
    const parts = [];
    // A lookback given as `time_value` + `time_unit` is one "Period" entry, written last.
    const hasPeriod = c.params.time_value !== undefined && c.params.time_unit !== undefined;

    Object.entries(c.params).forEach(([key, value]) => {
      if (hasPeriod && (key === "time_value" || key === "time_unit")) return;
      const spec = params[key];
      if (!spec) return;
      parts.push(
        I18n.t("strategies-summary-param", {
          label: paramLabel(spec, key),
          value: formatParamValue(value, spec),
        })
      );
    });

    if (hasPeriod) {
      // Ids are the `time_unit` values of the price change condition.
      const amount = formatFixed(Number(c.params.time_value), { decimals: 4, trim: true });
      switch (c.params.time_unit) {
        case "SECONDS":
          parts.push(I18n.t("strategies-summary-period-seconds", { amount }));
          break;
        case "MINUTES":
          parts.push(I18n.t("strategies-summary-period-minutes", { amount }));
          break;
        case "HOURS":
          parts.push(I18n.t("strategies-summary-period-hours", { amount }));
          break;
        default:
          parts.push(amount);
      }
    }

    const shown = parts.slice(0, 3);
    if (!shown.length) return I18n.t("strategies-summary-none");
    return I18n.t("strategies-summary-parts", {
      count: shown.length,
      first: shown[0],
      second: shown[1] ?? "",
      third: shown[2] ?? "",
    });
  }

  /**
   * Render parameter editor section for a condition
   */
  function renderParamEditor(c, schema, idx) {
    const entries = Object.entries(schema.parameters || {});
    if (!entries.length)
      return `<div class="param-row">${Utils.escapeHtml(I18n.t("strategies-summary-none"))}</div>`;
    // Basic approach: show all params; could gate last N as advanced in future
    const fields = entries.map(([key, spec]) => {
      const label = paramLabel(spec, key);
      const description = paramDescription(spec);
      const val = c.params[key] ?? spec.default ?? "";
      return `
        <div class="param-field">
          <label>${Utils.escapeHtml(label)}</label>
          ${renderParamInput(idx, key, spec, val)}
          ${description ? `<div class="property-description">${Utils.escapeHtml(description)}</div>` : ""}
        </div>
      `;
    });
    return `<div class="param-row">${fields.join("")}</div>`;
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
      case "enum": {
        const options = spec.options || spec.values || [];
        const optionsHtml = options
          .map((opt) => {
            const optValue = optionValue(opt);
            const selected = optValue === value ? "selected" : "";
            return `<option value="${Utils.escapeHtml(String(optValue))}" ${selected}>${Utils.escapeHtml(optionLabel(opt))}</option>`;
          })
          .join("");
        return `<select id="${id}" ${data} class="select-field" data-custom-select>${optionsHtml}</select>`;
      }
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
        Object.keys(schema.parameters || {}).forEach((k) => {
          const v = c.params[k];
          const defv = schema.parameters[k]?.default;
          params[k] = { value: v, default: defv };
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

  /**
   * Delete a condition from the editor
   */
  function deleteCondition(index) {
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
