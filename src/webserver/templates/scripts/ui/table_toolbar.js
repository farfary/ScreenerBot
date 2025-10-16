function escapeHtml(value) {
  if (value === null || value === undefined) {
    return "";
  }
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function renderMeta(metaItems = []) {
  if (!Array.isArray(metaItems) || metaItems.length === 0) {
    return "";
  }

  const items = metaItems
    .map((item) => {
      const variant = item.variant ? ` data-variant="${escapeHtml(item.variant)}"` : "";
      const id = item.id ? ` data-meta-id="${escapeHtml(item.id)}"` : "";
      return `<span class="table-toolbar-meta__item"${id}${variant}>${escapeHtml(
        item.text ?? ""
      )}</span>`;
    })
    .join("");

  return `<div class="table-toolbar-meta">${items}</div>`;
}

function renderSummary(summaryItems = []) {
  if (!Array.isArray(summaryItems) || summaryItems.length === 0) {
    return "";
  }

  const chips = summaryItems
    .map((item) => {
      const id = item.id ? ` data-summary-id="${escapeHtml(item.id)}"` : "";
      const variant = item.variant ? ` data-variant="${escapeHtml(item.variant)}"` : "";
      const icon = item.icon
        ? `<span class="table-toolbar-chip__icon">${escapeHtml(item.icon)}</span>`
        : "";
      const label = item.label
        ? `<span class="table-toolbar-chip__label">${escapeHtml(item.label)}</span>`
        : "";
      const value = `<span class="table-toolbar-chip__value">${escapeHtml(
        item.value ?? "-"
      )}</span>`;
      const tooltip = item.tooltip ? ` title="${escapeHtml(item.tooltip)}"` : "";
      return `<div class="table-toolbar-chip"${id}${variant}${tooltip}>${icon}${label}${value}</div>`;
    })
    .join("");

  return `<div class="table-toolbar-summary">${chips}</div>`;
}

function classNames(list) {
  return list.filter(Boolean).join(" ");
}

function escapeSelector(value) {
  if (typeof CSS !== "undefined" && typeof CSS.escape === "function") {
    return CSS.escape(value);
  }
  return value;
}

function renderButtons(buttons = []) {
  if (!Array.isArray(buttons) || buttons.length === 0) {
    return "";
  }

  return buttons
    .map((btn) => {
  const classes = ["dt-btn", "table-toolbar-btn"];
      if (btn.variant) {
        classes.push(`table-toolbar-btn--${btn.variant}`);
      }
      if (!btn.label || btn.iconOnly) {
        classes.push("table-toolbar-btn--icon");
      }
      if (btn.classes) {
        classes.push(btn.classes);
      }
      const icon = btn.icon
        ? `<span class="dt-btn-icon">${escapeHtml(btn.icon)}</span>`
        : "";
      const label = btn.label
        ? `<span class="dt-btn-label">${escapeHtml(btn.label)}</span>`
        : "";
      const tooltip = btn.tooltip ? btn.tooltip : btn.label;
      const titleAttr = tooltip ? ` title="${escapeHtml(tooltip)}"` : "";
      const ariaLabel = !btn.label && tooltip ? ` aria-label="${escapeHtml(tooltip)}"` : "";
      const dataId = btn.id ? ` data-btn-id="${escapeHtml(btn.id)}"` : "";
  return `<button class="${classNames(classes)}" type="button"${dataId}${titleAttr}${ariaLabel}>${icon}${label}</button>`;
    })
    .join("");
}

function renderSearch(searchConfig = {}, state = {}) {
  if (!searchConfig || searchConfig.enabled === false) {
    return "";
  }

  const placeholder = searchConfig.placeholder
    ? escapeHtml(searchConfig.placeholder)
    : "Search table...";
  const value = state.searchQuery ? escapeHtml(state.searchQuery) : "";

  return `
    <div class="table-toolbar-search dt-search">
      <input
        type="text"
        class="dt-search-input table-toolbar-input"
        placeholder="${placeholder}"
        value="${value}"
        autocomplete="off"
        spellcheck="false"
      />
      <span class="table-toolbar-search__icon" aria-hidden="true">🔍</span>
      <button type="button" class="table-toolbar-search__clear" aria-label="Clear search" hidden>
        ✕
      </button>
    </div>
  `;
}

function renderFilter(filter, stateFilters = {}) {
  const currentValue = stateFilters[filter.id] ?? filter.defaultValue ?? filter.options?.[0]?.value ?? "";
  const optionsMarkup = (filter.options || [])
    .map((opt) => {
      const selected = opt.value === currentValue ? " selected" : "";
      const disabled = opt.disabled ? " disabled" : "";
      return `<option value="${escapeHtml(opt.value)}"${selected}${disabled}>${escapeHtml(
        opt.label
      )}</option>`;
    })
    .join("");

  const label = filter.label
    ? `<label class="table-toolbar-field__label" for="tt-filter-${escapeHtml(filter.id)}">${escapeHtml(
        filter.label
      )}</label>`
    : "";

  const widthStyle = filter.minWidth ? ` style="min-width:${escapeHtml(filter.minWidth)};"` : "";
  const dataAttrs = [`data-filter-id="${escapeHtml(filter.id)}"`];
  if (filter.autoApply === false) {
    dataAttrs.push("data-auto-apply=\"false\"");
  }
  if (filter.defaultValue !== undefined) {
    dataAttrs.push(`data-default-value="${escapeHtml(filter.defaultValue)}"`);
  }

  return `
    <div class="table-toolbar-field"${widthStyle}>
      ${label}
      <select class="dt-filter table-toolbar-select" id="tt-filter-${escapeHtml(
        filter.id
      )}" ${dataAttrs.join(" ")}>
        ${optionsMarkup}
      </select>
    </div>
  `;
}

function renderCustomControl(control, stateControls = {}) {
  if (control.type !== "input") {
    return "";
  }

  const value = stateControls[control.id] ?? control.value ?? "";
  const label = control.label
    ? `<label class="table-toolbar-field__label" for="tt-control-${escapeHtml(control.id)}">${escapeHtml(
        control.label
      )}</label>`
    : "";
  const placeholder = control.placeholder ? escapeHtml(control.placeholder) : "";
  const widthStyle = control.minWidth ? ` style="min-width:${escapeHtml(control.minWidth)};"` : "";
  const dataAttrs = [`data-control-id="${escapeHtml(control.id)}"`];
  if (control.defaultValue !== undefined) {
    dataAttrs.push(`data-default-value="${escapeHtml(control.defaultValue)}"`);
  }
  if (control.clearable) {
    dataAttrs.push("data-clearable=\"true\"");
  }

  const input = `
    <div class="table-toolbar-search table-toolbar-search--inline" data-control-wrapper="${escapeHtml(
      control.id
    )}">
      <input
        type="text"
        class="table-toolbar-input table-toolbar-input--text"
        id="tt-control-${escapeHtml(control.id)}"
        placeholder="${placeholder}"
        value="${escapeHtml(value)}"
        autocomplete="off"
        spellcheck="false"
        ${dataAttrs.join(" ")}
      />
      ${control.clearable ? '<button type="button" class="table-toolbar-input__clear" aria-label="Clear">✕</button>' : ""}
    </div>
  `;

  return `
    <div class="table-toolbar-field"${widthStyle}>
      ${label}
      ${input}
    </div>
  `;
}

export class TableToolbarView {
  constructor(config = {}) {
    this.config = config || {};
  }

  render(state = {}) {
    const titleConfig = this.config.title || {};
    const titleIcon = titleConfig.icon
      ? `<span class="table-toolbar-title__icon">${escapeHtml(titleConfig.icon)}</span>`
      : "";
    const titleText = titleConfig.text
      ? `<span class="table-toolbar-title__text">${escapeHtml(titleConfig.text)}</span>`
      : "";
    const titleTag = titleConfig.tag
      ? `<span class="table-toolbar-title__tag">${escapeHtml(titleConfig.tag)}</span>`
      : "";
    const titleMeta = renderMeta(titleConfig.meta);

    const metaSection = renderMeta(this.config.meta);
    const summarySection = renderSummary(this.config.summary);
    const searchSection = renderSearch(this.config.search, state);
    const filtersSection = (this.config.filters || [])
      .map((filter) => renderFilter(filter, state.filters || {}))
      .join("");
    const customControlsSection = (this.config.customControls || [])
      .map((control) => renderCustomControl(control, state.customControls || {}))
      .join("");
    const buttonsSection = renderButtons(this.config.buttons);

    const settingsButton = this.config.settings === false
      ? ""
      : `
        <div class="dt-column-toggle table-toolbar-settings">
          <button class="dt-btn dt-btn-columns table-toolbar-btn table-toolbar-btn--icon" type="button" title="${escapeHtml(
            (this.config.settings && this.config.settings.tooltip) || "Table settings"
          )}" aria-label="Table settings">
            <span class="dt-btn-icon">${escapeHtml(
              (this.config.settings && this.config.settings.icon) || "⚙️"
            )}</span>
          </button>
          <div class="dt-column-menu" style="display:none;"></div>
        </div>
      `;

    const controlsPresent =
      searchSection || filtersSection || customControlsSection;

    return `
      <div class="data-table-toolbar table-toolbar">
        <div class="table-toolbar__row table-toolbar__row--main">
          <div class="table-toolbar-title">
            ${titleIcon}
            <div class="table-toolbar-title__text-block">
              <div class="table-toolbar-title__line">
                ${titleText}
                ${titleTag}
              </div>
              ${titleMeta}
            </div>
          </div>
          <div class="table-toolbar-actions">
            ${metaSection}
            ${buttonsSection}
            ${settingsButton}
          </div>
        </div>
        ${controlsPresent || summarySection ? `
          <div class="table-toolbar__row table-toolbar__row--controls">
            <div class="table-toolbar-controls">
              ${searchSection}
              ${customControlsSection}
              ${filtersSection}
            </div>
            ${summarySection}
          </div>
        ` : ""}
      </div>
    `;
  }

  static updateSummary(root, summaryItems = []) {
    if (!root || !summaryItems || summaryItems.length === 0) {
      return;
    }
    summaryItems.forEach((item) => {
      if (!item || !item.id) return;
      const chip = root.querySelector(
        `.table-toolbar-chip[data-summary-id="${escapeSelector(item.id)}"]`
      );
      if (!chip) {
        return;
      }
      if (item.variant) {
        chip.setAttribute("data-variant", item.variant);
      }
      if (item.tooltip !== undefined) {
        if (item.tooltip) {
          chip.setAttribute("title", item.tooltip);
        } else {
          chip.removeAttribute("title");
        }
      }
      const valueEl = chip.querySelector(".table-toolbar-chip__value");
      if (valueEl) {
        valueEl.textContent = item.value ?? "-";
      }
      if (item.label !== undefined) {
        const labelEl = chip.querySelector(".table-toolbar-chip__label");
        if (labelEl) {
          labelEl.textContent = item.label;
        }
      }
      if (item.icon !== undefined) {
        const iconEl = chip.querySelector(".table-toolbar-chip__icon");
        if (iconEl) {
          iconEl.textContent = item.icon;
        }
      }
    });
  }

  static updateMeta(root, metaItems = []) {
    if (!root || !metaItems || metaItems.length === 0) {
      return;
    }
    metaItems.forEach((item) => {
      if (!item || !item.id) return;
      const metaEl = root.querySelector(
        `.table-toolbar-meta__item[data-meta-id="${escapeSelector(item.id)}"]`
      );
      if (!metaEl) {
        return;
      }
      if (item.variant) {
        metaEl.setAttribute("data-variant", item.variant);
      }
      metaEl.textContent = item.text ?? "";
    });
  }

  static setSearchValue(root, value) {
    if (!root) return;
    const input = root.querySelector(".dt-search-input");
    if (input) {
      input.value = value ?? "";
      const clearBtn = root.querySelector(".table-toolbar-search__clear");
      if (clearBtn) {
        clearBtn.hidden = !(value ?? "").length;
      }
    }
  }

  static setFilterValue(root, filterId, value) {
    if (!root || !filterId) return;
    const select = root.querySelector(
      `.dt-filter[data-filter-id="${escapeSelector(filterId)}"]`
    );
    if (select) {
      select.value = value ?? "";
    }
  }

  static setCustomControlValue(root, controlId, value) {
    if (!root || !controlId) return;
    const input = root.querySelector(
      `.table-toolbar-input[data-control-id="${escapeSelector(controlId)}"]`
    );
    if (input) {
      input.value = value ?? "";
      const wrapper = input.closest("[data-control-wrapper]");
      if (wrapper) {
        const clearBtn = wrapper.querySelector(".table-toolbar-input__clear");
        if (clearBtn) {
          clearBtn.hidden = !(value ?? "").length;
        }
      }
    }
  }
}
