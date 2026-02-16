/**
 * Condition Editor Module
 * Handles the vertical card-based condition editor for building strategies
 */

export function createConditionEditor({
  state,
  conditions,
  conditionSchemas,
  $,
  $$,
  Utils,
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
      list.innerHTML =
        '<div class="empty-state"><i class="icon-puzzle"></i><p>No conditions yet</p><small>Use "Add Condition" to start building</small></div>';
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

  /**
   * Render a single condition card
   */
  function renderConditionCard(c, idx) {
    const schema = conditionSchemas?.[c.type] || {};
    const iconClass = schema.icon || getConditionIcon(c.type);
    const category = schema.category || "General";
    const description = schema.description || "";
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
                ${Utils.escapeHtml(c.name || c.type)}
                <span class="condition-category-badge category-${categorySlug}">${Utils.escapeHtml(category)}</span>
              </div>
              <div class="condition-description">${Utils.escapeHtml(description)}</div>
            </div>
          </div>
          <div class="card-header-right">
            <div class="condition-status">
              <label class="status-toggle" title="${c.enabled ? "Enabled" : "Disabled"}">
                <input type="checkbox" class="toggle-enabled" ${c.enabled ? "checked" : ""}/>
                <span class="status-indicator"></span>
              </label>
            </div>
            <div class="condition-actions">
              <button class="btn-icon" data-action="move-up" title="Move up"><i class="icon-chevron-up"></i></button>
              <button class="btn-icon" data-action="move-down" title="Move down"><i class="icon-chevron-down"></i></button>
              <button class="btn-icon" data-action="duplicate" title="Duplicate"><i class="icon-copy"></i></button>
              <button class="btn-icon" data-action="delete" title="Delete"><i class="icon-trash-2"></i></button>
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

    // Special handling for conditions with time period (time_value + time_unit)
    if (c.params.time_value !== undefined && c.params.time_unit !== undefined) {
      const timeValue = c.params.time_value;
      const timeUnit = c.params.time_unit;
      const unitLabel = timeUnit === "SECONDS" ? "sec" : timeUnit === "MINUTES" ? "min" : "hrs";
      const timePart = `${timeValue} ${unitLabel}`;

      // Add other parameters (skip time_value and time_unit as they're combined)
      Object.entries(c.params).forEach(([key, value]) => {
        if (key === "time_value" || key === "time_unit") return;
        const spec = params[key];
        if (!spec) return;

        const label = spec.name || key;
        const formattedValue = formatParamValueWithUnit(value, spec);
        parts.push(`${label}: ${formattedValue}`);
      });

      // Add time period last
      parts.push(`Period: ${timePart}`);
    } else {
      // Build human-readable summary based on condition type
      Object.entries(c.params).forEach(([key, value]) => {
        const spec = params[key];
        if (!spec) return;

        const label = spec.name || key;
        const formattedValue = formatParamValueWithUnit(value, spec);
        parts.push(`${label}: ${formattedValue}`);
      });
    }

    return parts.slice(0, 3).join(", ") || "No parameters";
  }

  /**
   * Format parameter value (simple version)
   */
  function formatParamValue(v) {
    if (v === undefined || v === null) return "";
    if (typeof v === "number") return String(v);
    if (typeof v === "boolean") return v ? "true" : "false";
    return String(v);
  }

  /**
   * Format parameter value with unit (for display in summary)
   */
  function formatParamValueWithUnit(value, spec) {
    if (value === undefined || value === null) return "—";

    // Handle enum types - show label instead of value
    if (spec.type === "enum" && spec.options) {
      const option = spec.options.find((opt) => {
        const optValue = typeof opt === "object" ? opt.value : opt;
        return optValue === value;
      });
      if (option) {
        return typeof option === "object" ? option.label : option;
      }
      return String(value);
    }

    // Handle boolean
    if (spec.type === "boolean") {
      return value
        ? '<i class="icon-check" style="color: var(--success);"></i> Yes'
        : '<i class="icon-x" style="color: var(--error);"></i> No';
    }

    // Handle numbers with units
    if (typeof value === "number") {
      // Percent type
      if (spec.type === "percent") {
        return `${value}%`;
      }
      // SOL type
      if (spec.type === "sol") {
        return `${value} SOL`;
      }
      // Check name for hints about unit
      const name = (spec.name || "").toLowerCase();
      if (name.includes("hour")) {
        return value === 1 ? `${value} hour` : `${value} hours`;
      }
      if (name.includes("minute")) {
        return value === 1 ? `${value} minute` : `${value} minutes`;
      }
      if (name.includes("candle") || name.includes("period") || name.includes("lookback")) {
        return value === 1 ? `${value} candle` : `${value} candles`;
      }
      if (name.includes("multiplier") || name.includes("ratio")) {
        return `${value}×`;
      }
      // Default number formatting
      return value % 1 === 0 ? String(value) : value.toFixed(2);
    }

    return String(value);
  }

  /**
   * Render parameter editor section for a condition
   */
  function renderParamEditor(c, schema, idx) {
    const entries = Object.entries(schema.parameters || {});
    if (!entries.length) return '<div class="param-row">No parameters</div>';
    // Basic approach: show all params; could gate last N as advanced in future
    const fields = entries.map(([key, spec]) => {
      const label = spec.name || key;
      const val = c.params[key] ?? spec.default ?? "";
      return `
        <div class="param-field">
          <label>${Utils.escapeHtml(label)}</label>
          ${renderParamInput(idx, key, spec, val)}
          ${spec.description ? `<div class="property-description">${Utils.escapeHtml(spec.description)}</div>` : ""}
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
      case "percent": {
        return `<div class="input-with-unit">
          <input id="${id}" ${data} type="number" value="${value}" ${min} ${max} ${step} placeholder="0">
          <span class="input-unit">%</span>
        </div>`;
      }
      case "sol": {
        return `<div class="input-with-unit">
          <input id="${id}" ${data} type="number" value="${value}" ${min} ${max} ${step} placeholder="0">
          <span class="input-unit">SOL</span>
        </div>`;
      }
      case "number": {
        // Check if we should add a unit based on the name
        const name = (spec.name || "").toLowerCase();
        let unit = "";
        if (name.includes("hour")) unit = "hrs";
        else if (name.includes("minute")) unit = "min";
        else if (name.includes("multiplier")) unit = "×";

        if (unit) {
          return `<div class="input-with-unit">
            <input id="${id}" ${data} type="number" value="${value}" ${min} ${max} ${step} placeholder="0">
            <span class="input-unit">${unit}</span>
          </div>`;
        }
        return `<input id="${id}" ${data} type="number" value="${value}" ${min} ${max} ${step} placeholder="0">`;
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
            const optValue = typeof opt === "object" ? opt.value : opt;
            const optLabel = typeof opt === "object" ? opt.label : opt;
            const selected = optValue === value ? "selected" : "";
            return `<option value="${Utils.escapeHtml(String(optValue))}" ${selected}>${Utils.escapeHtml(String(optLabel))}</option>`;
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
    if (!schema)
      return Utils.showToast({
        type: "error",
        title: "Unknown Condition",
        message: "Condition type not found",
      });

    // Auto-create strategy if none exists (first condition added)
    // Show modal to select type first
    if (!state.currentStrategy) {
      Utils.showToast({
        type: "warning",
        title: "Create Strategy First",
        message: "Click 'New Strategy' to create a strategy before adding conditions",
      });
      return;
    }

    const params = {};
    Object.entries(schema.parameters || {}).forEach(([k, p]) => {
      params[k] = p.default ?? null;
    });
    conditions.push({
      type: conditionType,
      name: schema.name || conditionType,
      enabled: true,
      params,
    });
    renderConditionsList();
    updateRuleTreeFromEditor();
    Utils.showToast({
      type: "success",
      title: "Condition Added",
      message: `${schema.name || conditionType} added to strategy`,
    });
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
        name: schema.name || cond.type,
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
    formatParamValue,
    formatParamValueWithUnit,
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
