/**
 * Table toolbar — the single chrome strip above a DataTable.
 *
 * One band, three zones, laid out by the content it is given:
 *
 *   [ lead: identity + stats ]  [ controls: search / views / filters ]  [ actions ]
 *
 * The bar is the ONLY band a table page may put above its rows: subject identity,
 * dataset stats, query controls and actions all live here, so pages never stack a
 * hand-rolled info/action band on top of the table.
 *
 * Config is declarative and additive. The minimal form stays three lines:
 *
 *   toolbar: { summary: [...], search: { enabled: true } }
 *
 * The rich form composes typed items into the `controls` / `actions` zones:
 *
 *   toolbar: {
 *     identity: { icon, title, tag, address: { value, href }, details: [...] },
 *     summary:  [ { id, label, value, variant } ],
 *     search:   { enabled, placeholder, mode, onChange, onSubmit },
 *     views:    { id, value, options: [{ value, label, icon }], onChange },
 *     filters:  [ { id, label, options } | { id, control: "switch" } ],
 *     customControls: [ { id, type: "input", placeholder } ],
 *     buttons:  [ { id, label, icon, variant, overflow } ],
 *     controls: [ item | { type: "group", items: [item, ...] } ],
 *     actions:  [ item | { type: "group", items: [item, ...] } ],
 *     settings: false | { icon, tooltip },
 *     layout: "inline" | "query-row",
 *   }
 *
 * `filters`/`customControls`/`buttons`/`views` are sugar that normalizes into the
 * same typed items as `controls`/`actions`, so a page can start minimal and grow
 * without rewriting its config.
 *
 * Item types: search · select · switch · input · segmented · button · link ·
 *             stat · text · group · divider · spacer · slot · custom
 */

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

function classNames(list) {
  return list.filter(Boolean).join(" ");
}

function escapeSelector(value) {
  if (typeof CSS !== "undefined" && typeof CSS.escape === "function") {
    return CSS.escape(value);
  }
  return value;
}

// =============================================================================
// Icons
// =============================================================================

function sanitizeIconClassNames(value = "") {
  return value
    .split(/\s+/)
    .filter((part) => part && /^[a-zA-Z0-9_-]+$/.test(part))
    .join(" ");
}

function extractIconFromHtml(value = "") {
  const trimmed = value.trim();
  if (!trimmed.startsWith("<")) {
    return "";
  }
  const match = trimmed.match(/class=["']([^"']+)["']/i);
  if (!match) {
    return "";
  }
  return sanitizeIconClassNames(match[1]);
}

function resolveIconClass(iconConfig) {
  if (!iconConfig) {
    return "";
  }
  if (typeof iconConfig === "string") {
    const trimmed = iconConfig.trim();
    if (!trimmed) {
      return "";
    }
    if (trimmed.startsWith("<")) {
      return extractIconFromHtml(trimmed);
    }
    return sanitizeIconClassNames(trimmed);
  }
  if (typeof iconConfig === "object") {
    if (iconConfig.html) {
      return extractIconFromHtml(iconConfig.html);
    }
    const candidate =
      iconConfig.class || iconConfig.className || iconConfig.name || iconConfig.icon || "";
    return sanitizeIconClassNames(candidate);
  }
  return "";
}

function renderIconMarkup(iconConfig, wrapperClass) {
  if (!iconConfig || !wrapperClass) {
    return "";
  }
  const classes = resolveIconClass(iconConfig);
  if (!classes) {
    if (typeof iconConfig === "string") {
      const trimmed = iconConfig.trim();
      return trimmed
        ? `<span class="${wrapperClass}" aria-hidden="true">${escapeHtml(trimmed)}</span>`
        : "";
    }
    if (typeof iconConfig === "object") {
      const textVariant = iconConfig.text || iconConfig.label;
      return textVariant
        ? `<span class="${wrapperClass}" aria-hidden="true">${escapeHtml(textVariant)}</span>`
        : "";
    }
    return "";
  }
  return `<span class="${wrapperClass}" aria-hidden="true"><i class="${escapeHtml(classes)}"></i></span>`;
}

// =============================================================================
// Normalization — every sugar form collapses into one typed item shape
// =============================================================================

function inferItemType(raw) {
  if (raw.type) {
    return raw.type;
  }
  if (Array.isArray(raw.items)) {
    return "group";
  }
  if (raw.control === "switch") {
    return "switch";
  }
  if (Array.isArray(raw.options)) {
    return "select";
  }
  if (raw.href) {
    return "link";
  }
  return "button";
}

/**
 * Normalize one raw config entry into a typed item.
 * Returns null for entries that cannot render.
 */
function normalizeItem(raw, index) {
  if (!raw) {
    return null;
  }
  if (typeof raw === "string") {
    if (raw === "divider" || raw === "|") {
      return { type: "divider" };
    }
    if (raw === "spacer") {
      return { type: "spacer" };
    }
    return { type: "text", text: raw };
  }

  const type = inferItemType(raw);
  const item = { ...raw, type };

  if (type === "group") {
    item.items = (raw.items || []).map((child, i) => normalizeItem(child, i)).filter(Boolean);
    if (item.items.length === 0) {
      return null;
    }
  }

  // Items that carry state or handlers must be addressable.
  if (
    !item.id &&
    (type === "button" ||
      type === "select" ||
      type === "switch" ||
      type === "input" ||
      type === "segmented" ||
      type === "stat")
  ) {
    item.id = `tt-${type}-${index}`;
  }

  return item;
}

function normalizeList(list) {
  if (!Array.isArray(list)) {
    return [];
  }
  return list.map((raw, i) => normalizeItem(raw, i)).filter(Boolean);
}

function indexItems(items, map) {
  items.forEach((item) => {
    if (item.id) {
      map.set(item.id, item);
    }
    if (item.type === "group") {
      indexItems(item.items, map);
    }
  });
}

/**
 * Build the render model: three ordered zones plus a flat id index.
 */
function normalizeConfig(config = {}) {
  const cfg = config || {};

  const identity =
    cfg.identity && (cfg.identity.title || cfg.identity.icon || cfg.identity.address)
      ? { ...cfg.identity }
      : null;

  const stats = normalizeList((cfg.summary || []).map((entry) => ({ ...entry, type: "stat" })));

  const controls = [];
  if (cfg.search && cfg.search.enabled !== false) {
    controls.push({ ...cfg.search, type: "search", id: cfg.search.id || "search" });
  }
  if (cfg.views && Array.isArray(cfg.views.options) && cfg.views.options.length > 0) {
    controls.push({ ...cfg.views, type: "segmented", id: cfg.views.id || "views" });
  }
  controls.push(...normalizeList(cfg.customControls));
  controls.push(...normalizeList(cfg.filters));
  controls.push(...normalizeList(cfg.controls));

  const actions = [];
  actions.push(...normalizeList(cfg.buttons));
  actions.push(...normalizeList(cfg.actions));

  const settings =
    cfg.settings === false
      ? null
      : {
          icon: (cfg.settings && cfg.settings.icon) || "icon-settings",
          tooltip: (cfg.settings && cfg.settings.tooltip) || "Table settings",
        };

  const index = new Map();
  indexItems(stats, index);
  indexItems(controls, index);
  indexItems(actions, index);

  const layout = cfg.layout === "query-row" ? "query-row" : "inline";

  return {
    identity,
    stats,
    controls,
    actions,
    settings,
    index,
    density: cfg.density || "auto",
    layout,
  };
}

// =============================================================================
// Item renderers
// =============================================================================

function commonAttrs(item) {
  const attrs = [];
  if (item.id) {
    attrs.push(`data-item-id="${escapeHtml(item.id)}"`);
  }
  if (item.hidden) {
    attrs.push("hidden");
  }
  return attrs.join(" ");
}

function renderStat(item) {
  const id = item.id ? ` data-summary-id="${escapeHtml(item.id)}"` : "";
  const variant = item.variant ? ` data-variant="${escapeHtml(item.variant)}"` : "";
  const tooltip = item.tooltip ? ` title="${escapeHtml(item.tooltip)}"` : "";
  const icon = renderIconMarkup(item.icon, "table-toolbar-chip__icon");
  const label = item.label
    ? `<span class="table-toolbar-chip__label">${escapeHtml(item.label)}</span>`
    : "";
  const value = `<span class="table-toolbar-chip__value">${escapeHtml(item.value ?? "-")}</span>`;
  return `<div class="table-toolbar-chip"${id}${variant}${tooltip}${item.hidden ? " hidden" : ""}>${icon}${label}${value}</div>`;
}

function renderText(item) {
  const variant = item.variant ? ` data-variant="${escapeHtml(item.variant)}"` : "";
  const id = item.id ? ` data-meta-id="${escapeHtml(item.id)}"` : "";
  const label = item.label
    ? `<span class="table-toolbar-text__label">${escapeHtml(item.label)}</span>`
    : "";
  return `<span class="table-toolbar-text"${id}${variant}${item.hidden ? " hidden" : ""}>${label}<span class="table-toolbar-text__value">${escapeHtml(
    item.text ?? item.value ?? ""
  )}</span></span>`;
}

function renderSearch(item, state = {}) {
  const placeholder = item.placeholder ? escapeHtml(item.placeholder) : "Search table...";
  const value = state.searchQuery ? escapeHtml(state.searchQuery) : "";
  const grow = item.grow === false ? ' data-grow="false"' : "";
  const ariaLabel = item.ariaLabel
    ? ` aria-label="${escapeHtml(item.ariaLabel)}"`
    : "";
  const widthStyle = item.minWidth
    ? ` style="--table-toolbar-search-min-width:${escapeHtml(item.minWidth)};"`
    : "";

  return `
    <div class="table-toolbar-search dt-search"${grow}${widthStyle} ${commonAttrs(item)}>
      <span class="table-toolbar-search__icon" aria-hidden="true"><i class="icon-search"></i></span>
      <input
        type="text"
        class="dt-search-input table-toolbar-input"
        placeholder="${placeholder}"
        ${ariaLabel}
        value="${value}"
        autocomplete="off"
        spellcheck="false"
      />
      <button type="button" class="table-toolbar-search__clear" aria-label="Clear search" hidden>
        <i class="icon-x"></i>
      </button>
    </div>
  `;
}

function selectLabelWidth(options = []) {
  const longest = options.reduce((max, option) => {
    const label = option?.label ?? option?.value ?? "";
    return Math.max(max, Array.from(String(label)).length);
  }, 0);
  const characters = Math.max(7, Math.min(longest || 7, 16));
  return `${characters * 0.5}rem`;
}

function renderSwitch(item, stateFilters = {}) {
  const currentValue = stateFilters[item.id];
  const checked =
    currentValue !== undefined ? Boolean(currentValue) : Boolean(item.defaultValue ?? false);
  const label = item.label
    ? `<span class="table-toolbar-field__label">${escapeHtml(item.label)}</span>`
    : "";
  const onLabel = item.switchLabels?.on ?? "On";
  const offLabel = item.switchLabels?.off ?? "All";

  return `
    <div class="table-toolbar-field table-toolbar-field--switch" data-filter-id="${escapeHtml(item.id)}" ${commonAttrs(item)}>
      ${label}
      <label class="toggle toggle-sm">
        <input
          type="checkbox"
          class="dt-filter"
          id="tt-filter-${escapeHtml(item.id)}"
          data-filter-id="${escapeHtml(item.id)}"
          data-filter-kind="switch"
          ${checked ? "checked" : ""}
        />
        <span class="toggle-track" aria-hidden="true"></span>
        <span class="toggle-state" data-on-label="${escapeHtml(onLabel)}" data-off-label="${escapeHtml(offLabel)}">${escapeHtml(
          checked ? onLabel : offLabel
        )}</span>
      </label>
    </div>
  `;
}

function renderSelect(item, stateFilters = {}) {
  const currentValue = stateFilters[item.id] ?? item.defaultValue ?? item.options?.[0]?.value ?? "";
  const optionsMarkup = (item.options || [])
    .map((opt) => {
      const selected = opt.value === currentValue ? " selected" : "";
      const disabled = opt.disabled ? " disabled" : "";
      return `<option value="${escapeHtml(opt.value)}"${selected}${disabled}>${escapeHtml(
        opt.label
      )}</option>`;
    })
    .join("");

  const label = item.label
    ? `<label class="table-toolbar-field__label" for="tt-filter-${escapeHtml(item.id)}">${escapeHtml(
        item.label
      )}</label>`
    : "";

  const widthTokens = [
    `--table-toolbar-select-label-width:${selectLabelWidth(item.options)}`,
  ];
  if (item.minWidth) {
    widthTokens.push(`--table-toolbar-field-min-width:${escapeHtml(item.minWidth)}`);
  }
  const widthStyle = ` style="${widthTokens.join(";")};"`;
  const dataAttrs = [`data-filter-id="${escapeHtml(item.id)}"`];
  if (item.autoApply === false) {
    dataAttrs.push('data-auto-apply="false"');
  }
  if (item.defaultValue !== undefined) {
    dataAttrs.push(`data-default-value="${escapeHtml(item.defaultValue)}"`);
  }

  return `
    <div class="table-toolbar-field"${widthStyle} data-filter-id="${escapeHtml(item.id)}" ${commonAttrs(item)}>
      ${label}
      <select class="dt-filter table-toolbar-select" id="tt-filter-${escapeHtml(
        item.id
      )}" data-custom-select ${dataAttrs.join(" ")}>
        ${optionsMarkup}
      </select>
    </div>
  `;
}

function renderInput(item, stateControls = {}) {
  const value = stateControls[item.id] ?? item.value ?? "";
  const label = item.label
    ? `<label class="table-toolbar-field__label" for="tt-control-${escapeHtml(item.id)}">${escapeHtml(
        item.label
      )}</label>`
    : "";
  const placeholder = item.placeholder ? escapeHtml(item.placeholder) : "";
  const widthStyle = item.minWidth
    ? ` style="--table-toolbar-field-min-width:${escapeHtml(item.minWidth)};"`
    : "";
  const dataAttrs = [`data-control-id="${escapeHtml(item.id)}"`];
  if (item.defaultValue !== undefined) {
    dataAttrs.push(`data-default-value="${escapeHtml(item.defaultValue)}"`);
  }
  if (item.clearable) {
    dataAttrs.push('data-clearable="true"');
  }

  return `
    <div class="table-toolbar-field"${widthStyle} ${commonAttrs(item)}>
      ${label}
      <div class="table-toolbar-search table-toolbar-search--inline" data-control-wrapper="${escapeHtml(
        item.id
      )}">
        ${item.icon ? renderIconMarkup(item.icon, "table-toolbar-search__icon") : ""}
        <input
          type="text"
          class="table-toolbar-input table-toolbar-input--text"
          id="tt-control-${escapeHtml(item.id)}"
          placeholder="${placeholder}"
          value="${escapeHtml(value)}"
          autocomplete="off"
          spellcheck="false"
          ${dataAttrs.join(" ")}
        />
        ${item.clearable ? '<button type="button" class="table-toolbar-input__clear" aria-label="Clear" hidden><i class="icon-x"></i></button>' : ""}
      </div>
    </div>
  `;
}

function renderSegmented(item) {
  const current = item.value ?? item.defaultValue ?? item.options?.[0]?.value;
  const buttons = (item.options || [])
    .map((opt) => {
      const active = opt.value === current;
      const icon = renderIconMarkup(opt.icon, "table-toolbar-seg__icon");
      const label = opt.label
        ? `<span class="table-toolbar-seg__label">${escapeHtml(opt.label)}</span>`
        : "";
      const tooltip = opt.tooltip || (!opt.label ? opt.value : "");
      const titleAttr = tooltip ? ` title="${escapeHtml(tooltip)}"` : "";
      const count =
        opt.count !== undefined && opt.count !== null
          ? `<span class="table-toolbar-seg__count">${escapeHtml(opt.count)}</span>`
          : "";
      return `<button type="button" class="table-toolbar-seg__btn${active ? " active" : ""}" data-seg-value="${escapeHtml(
        opt.value
      )}" aria-pressed="${active ? "true" : "false"}"${titleAttr}>${icon}${label}${count}</button>`;
    })
    .join("");

  const label = item.label
    ? `<span class="table-toolbar-field__label">${escapeHtml(item.label)}</span>`
    : "";

  return `
    <div class="table-toolbar-seg-wrap" ${commonAttrs(item)}>
      ${label}
      <div class="table-toolbar-seg" role="group" data-seg-id="${escapeHtml(item.id)}"${
        item.ariaLabel ? ` aria-label="${escapeHtml(item.ariaLabel)}"` : ""
      }>${buttons}</div>
    </div>
  `;
}

function buttonClassList(item, extra = []) {
  const classes = ["table-toolbar-btn", ...extra];
  if (item.variant) {
    classes.push(`table-toolbar-btn--${item.variant}`);
  }
  if (!item.label || item.iconOnly) {
    classes.push("table-toolbar-btn--icon");
  } else if (item.collapseLabel !== false) {
    // Labelled buttons shed their text in a narrow bar instead of wrapping the row.
    classes.push("table-toolbar-btn--collapsible");
  }
  if (item.classes) {
    classes.push(item.classes);
  }
  return classes;
}

function renderButton(item) {
  const icon = renderIconMarkup(item.icon, "dt-btn-icon");
  const label = item.label ? `<span class="dt-btn-label">${escapeHtml(item.label)}</span>` : "";
  const tooltip = item.tooltip || item.label;
  const titleAttr = tooltip ? ` title="${escapeHtml(tooltip)}"` : "";
  const ariaLabel = !item.label && tooltip ? ` aria-label="${escapeHtml(tooltip)}"` : "";
  const disabled = item.disabled ? " disabled" : "";
  return `<button class="${classNames(buttonClassList(item))}" type="button" data-btn-id="${escapeHtml(
    item.id
  )}"${titleAttr}${ariaLabel}${disabled}${item.hidden ? " hidden" : ""}>${icon}${label}</button>`;
}

function renderLink(item) {
  const icon = renderIconMarkup(item.icon, "dt-btn-icon");
  const label = item.label ? `<span class="dt-btn-label">${escapeHtml(item.label)}</span>` : "";
  const tooltip = item.tooltip || item.label;
  const titleAttr = tooltip ? ` title="${escapeHtml(tooltip)}"` : "";
  const ariaLabel = !item.label && tooltip ? ` aria-label="${escapeHtml(tooltip)}"` : "";
  const id = item.id ? ` data-item-id="${escapeHtml(item.id)}"` : "";
  return `<a class="${classNames(buttonClassList(item))}" href="${escapeHtml(
    item.href
  )}" target="${escapeHtml(item.target || "_blank")}" rel="noopener"${id}${titleAttr}${ariaLabel}${
    item.hidden ? " hidden" : ""
  }>${icon}${label}</a>`;
}

function renderMenuItem(item) {
  const icon = renderIconMarkup(item.icon, "table-toolbar-menu__icon");
  const variant = item.variant ? ` data-variant="${escapeHtml(item.variant)}"` : "";
  const disabled = item.disabled ? " disabled" : "";
  return `<button type="button" class="table-toolbar-menu__item" data-btn-id="${escapeHtml(
    item.id
  )}"${variant}${disabled}${item.hidden ? " hidden" : ""}>${icon}<span class="table-toolbar-menu__label">${escapeHtml(
    item.label || item.tooltip || item.id
  )}</span></button>`;
}

function renderCustom(item) {
  const id = item.id ? ` data-item-id="${escapeHtml(item.id)}"` : "";
  return `<div class="table-toolbar-custom"${id}${item.hidden ? " hidden" : ""}>${item.html || ""}</div>`;
}

function renderSlot(item) {
  return `<span class="table-toolbar-slot" data-slot="${escapeHtml(item.name || item.id || "")}"></span>`;
}

function renderIdentity(identity) {
  if (!identity) {
    return "";
  }

  const icon = renderIconMarkup(identity.icon, "table-toolbar-identity__icon");
  const title = identity.title
    ? `<span class="table-toolbar-identity__title">${escapeHtml(identity.title)}</span>`
    : "";
  const tag = identity.tag
    ? `<span class="table-toolbar-identity__tag"${
        identity.tagVariant ? ` data-variant="${escapeHtml(identity.tagVariant)}"` : ""
      }>${escapeHtml(identity.tag)}</span>`
    : "";

  const address = identity.address;
  let addressMarkup = "";
  if (address && address.value) {
    const copyBtn =
      address.copy === false
        ? ""
        : `<button type="button" class="table-toolbar-identity__act" data-toolbar-copy="${escapeHtml(
            address.value
          )}" title="Copy address" aria-label="Copy address"><i class="icon-copy"></i></button>`;
    const linkBtn = address.href
      ? `<a class="table-toolbar-identity__act" href="${escapeHtml(
          address.href
        )}" target="_blank" rel="noopener" title="${escapeHtml(
          address.linkTooltip || "Open in explorer"
        )}" aria-label="${escapeHtml(address.linkTooltip || "Open in explorer")}"><i class="icon-external-link"></i></a>`
      : "";
    addressMarkup = `
      <span class="table-toolbar-identity__address-group">
        <code class="table-toolbar-identity__address">${escapeHtml(address.value)}</code>
        ${copyBtn}
        ${linkBtn}
      </span>
    `;
  }

  const subtitle = identity.subtitle
    ? `<span class="table-toolbar-identity__subtitle">${escapeHtml(identity.subtitle)}</span>`
    : "";

  const sub =
    addressMarkup || subtitle
      ? `<div class="table-toolbar-identity__sub">${subtitle}${addressMarkup}</div>`
      : "";

  return `
    <div class="table-toolbar-identity">
      ${icon}
      <div class="table-toolbar-identity__body">
        <div class="table-toolbar-identity__line">${title}${tag}</div>
        ${sub}
      </div>
    </div>
  `;
}

// =============================================================================
// Zone rendering
// =============================================================================

function renderItem(item, state) {
  switch (item.type) {
    case "search":
      return renderSearch(item, state);
    case "select":
      return renderSelect(item, state.filters || {});
    case "switch":
      return renderSwitch(item, state.filters || {});
    case "input":
      return renderInput(item, state.customControls || {});
    case "segmented":
      return renderSegmented(item);
    case "button":
      return renderButton(item);
    case "link":
      return renderLink(item);
    case "stat":
      return renderStat(item);
    case "text":
      return renderText(item);
    case "divider":
      return '<span class="table-toolbar-divider" aria-hidden="true"></span>';
    case "spacer":
      return '<span class="table-toolbar-spacer"></span>';
    case "slot":
      return renderSlot(item);
    case "custom":
      return renderCustom(item);
    case "group":
      return renderGroup(item, state);
    default:
      return "";
  }
}

function renderGroup(group, state) {
  const inner = renderItems(group.items, state, true);
  const label = group.label
    ? `<span class="table-toolbar-cluster__label">${escapeHtml(group.label)}</span>`
    : "";
  const attached = group.attached === false ? "" : " table-toolbar-cluster--attached";
  const id = group.id ? ` data-item-id="${escapeHtml(group.id)}"` : "";
  return `<div class="table-toolbar-cluster${attached}"${id}${group.hidden ? " hidden" : ""}>${label}${inner}</div>`;
}

function renderItems(items, state, groupAdjacentSelects = false) {
  const markup = [];

  for (let index = 0; index < items.length; index += 1) {
    if (!groupAdjacentSelects || items[index].type !== "select") {
      markup.push(renderItem(items[index], state));
      continue;
    }

    const selectRun = [];
    while (index < items.length && items[index].type === "select") {
      selectRun.push(items[index]);
      index += 1;
    }
    index -= 1;

    const selects = selectRun.map((item) => renderItem(item, state)).join("");
    markup.push(
      selectRun.length > 1 ? `<div class="table-toolbar-select-group">${selects}</div>` : selects
    );
  }

  return markup.join("");
}

function renderZone(items, state, zoneClass) {
  const inner = renderItems(items, state, true);
  if (!inner) {
    return "";
  }
  return `<div class="${zoneClass}">${inner}</div>`;
}

// =============================================================================
// View
// =============================================================================

export class TableToolbarView {
  constructor(config = {}) {
    this.setConfig(config);
  }

  setConfig(config) {
    this._rawConfig = config || {};
    this.model = normalizeConfig(this._rawConfig);
  }

  /** Config setter kept as a property so callers can assign `view.config = next`. */
  set config(next) {
    this.setConfig(next);
  }

  get config() {
    return this._rawConfig;
  }

  /** Flat lookup across every zone and nested group. */
  getItem(id) {
    return this.model.index.get(id) || null;
  }

  /** True when the bar would render nothing at all. */
  isEmpty() {
    const { identity, stats, controls, actions, settings } = this.model;
    return (
      !identity && stats.length === 0 && controls.length === 0 && actions.length === 0 && !settings
    );
  }

  render(state = {}) {
    const { identity, stats, controls, actions, settings, layout } = this.model;

    if (this.isEmpty()) {
      return "";
    }

    // Actions split into the visible strip and the overflow menu.
    const inlineActions = actions.filter((item) => !item.overflow);
    const overflowActions = actions.filter((item) => item.overflow);

    const identityMarkup = renderIdentity(identity);
    const statsMarkup = stats.length
      ? `<div class="table-toolbar-summary">${stats.map(renderStat).join("")}</div>`
      : "";

    const leadInner = `${identityMarkup}${statsMarkup}`;
    const leadZone = leadInner ? `<div class="table-toolbar__lead">${leadInner}</div>` : "";

    const controlsZone = renderZone(controls, state, "table-toolbar__controls");

    const overflowMarkup = overflowActions.length
      ? `
        <div class="table-toolbar-overflow">
          <button
            type="button"
            class="table-toolbar-btn table-toolbar-btn--icon table-toolbar-overflow__trigger"
            title="More actions"
            aria-label="More actions"
            aria-haspopup="menu"
            aria-expanded="false"
          >
            <span class="dt-btn-icon" aria-hidden="true"><i class="icon-ellipsis"></i></span>
          </button>
          <div class="table-toolbar-menu" role="menu" hidden>
            ${overflowActions.map(renderMenuItem).join("")}
          </div>
        </div>
      `
      : "";

    const settingsMarkup = settings
      ? `
        <div class="dt-column-toggle table-toolbar-settings">
          <button class="dt-btn-columns table-toolbar-btn table-toolbar-btn--icon" type="button" title="${escapeHtml(
            settings.tooltip
          )}" aria-label="${escapeHtml(settings.tooltip)}">
            ${renderIconMarkup(settings.icon, "dt-btn-icon")}
          </button>
        </div>
      `
      : "";

    // Stable insertion point for the hybrid pagination-mode toggle.
    const paginationSlot = '<span class="table-toolbar-slot" data-slot="pagination-mode"></span>';

    const actionsInner = `${inlineActions
      .map((item) => renderItem(item, state))
      .join("")}${paginationSlot}${overflowMarkup}${settingsMarkup}`;
    const actionsZone = actionsInner
      ? `<div class="table-toolbar__actions">${actionsInner}</div>`
      : "";

    const density =
      this.model.density === "auto" ? (identity ? "default" : "compact") : this.model.density;

    return `
      <div class="data-table-toolbar table-toolbar" data-density="${escapeHtml(
        density
      )}" data-layout="${escapeHtml(layout)}"${
        identity ? ' data-has-identity="true"' : ""
      }>
        ${leadZone}
        ${controlsZone}
        ${actionsZone}
      </div>
    `;
  }

  // ===========================================================================
  // In-place updates — no re-render, so a polling table never loses focus
  // ===========================================================================

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
      if (item.hidden !== undefined) {
        chip.hidden = Boolean(item.hidden);
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
    });
  }

  /**
   * Patch a single addressable item in place.
   * Supported keys: hidden, disabled, busy, label, tooltip, variant, value (segmented).
   */
  static updateItem(root, id, patch = {}) {
    if (!root || !id) {
      return;
    }
    const sel = escapeSelector(id);
    const el =
      root.querySelector(`[data-btn-id="${sel}"]`) ||
      root.querySelector(`[data-item-id="${sel}"]`) ||
      root.querySelector(`.table-toolbar-field[data-filter-id="${sel}"]`) ||
      root.querySelector(`.table-toolbar-chip[data-summary-id="${sel}"]`);
    if (!el) {
      return;
    }

    if (patch.hidden !== undefined) {
      el.hidden = Boolean(patch.hidden);
    }
    if (patch.disabled !== undefined) {
      if ("disabled" in el) {
        el.disabled = Boolean(patch.disabled);
      }
      el.classList.toggle("disabled", Boolean(patch.disabled));
    }
    if (patch.busy !== undefined) {
      el.classList.toggle("is-busy", Boolean(patch.busy));
      if ("disabled" in el) {
        el.disabled = Boolean(patch.busy) || Boolean(patch.disabled);
      }
    }
    if (patch.variant !== undefined) {
      el.setAttribute("data-variant", patch.variant);
    }
    if (patch.tooltip !== undefined) {
      if (patch.tooltip) {
        el.setAttribute("title", patch.tooltip);
      } else {
        el.removeAttribute("title");
      }
    }
    if (patch.label !== undefined) {
      const labelEl = el.querySelector(".dt-btn-label, .table-toolbar-menu__label");
      if (labelEl) {
        labelEl.textContent = patch.label;
      }
    }
  }

  static setSegmentValue(root, segId, value) {
    if (!root || !segId) return;
    const seg = root.querySelector(`.table-toolbar-seg[data-seg-id="${escapeSelector(segId)}"]`);
    if (!seg) return;
    seg.querySelectorAll(".table-toolbar-seg__btn").forEach((btn) => {
      const active = btn.dataset.segValue === String(value);
      btn.classList.toggle("active", active);
      btn.setAttribute("aria-pressed", active ? "true" : "false");
    });
  }

  static setSegmentCounts(root, segId, counts = {}) {
    if (!root || !segId) return;
    const seg = root.querySelector(`.table-toolbar-seg[data-seg-id="${escapeSelector(segId)}"]`);
    if (!seg) return;
    Object.entries(counts).forEach(([value, count]) => {
      const btn = seg.querySelector(
        `.table-toolbar-seg__btn[data-seg-value="${escapeSelector(value)}"]`
      );
      if (!btn) return;
      let countEl = btn.querySelector(".table-toolbar-seg__count");
      if (count === null || count === undefined) {
        countEl?.remove();
        return;
      }
      if (!countEl) {
        countEl = document.createElement("span");
        countEl.className = "table-toolbar-seg__count";
        btn.appendChild(countEl);
      }
      countEl.textContent = String(count);
    });
  }

  static updateIdentity(root, identity = {}) {
    if (!root) return;
    const block = root.querySelector(".table-toolbar-identity");
    if (!block) return;

    if (identity.title !== undefined) {
      const titleEl = block.querySelector(".table-toolbar-identity__title");
      if (titleEl) {
        titleEl.textContent = identity.title;
      }
    }
    if (identity.tag !== undefined) {
      const tagEl = block.querySelector(".table-toolbar-identity__tag");
      if (tagEl) {
        tagEl.textContent = identity.tag;
        tagEl.hidden = !identity.tag;
      }
    }
    if (identity.subtitle !== undefined) {
      const subEl = block.querySelector(".table-toolbar-identity__subtitle");
      if (subEl) {
        subEl.textContent = identity.subtitle;
        subEl.hidden = !identity.subtitle;
      }
    }
    if (identity.address !== undefined) {
      const value = identity.address?.value ?? "";
      const codeEl = block.querySelector(".table-toolbar-identity__address");
      if (codeEl) {
        codeEl.textContent = value;
      }
      const copyBtn = block.querySelector("[data-toolbar-copy]");
      if (copyBtn) {
        copyBtn.dataset.toolbarCopy = value;
      }
      const linkEl = block.querySelector(".table-toolbar-identity__act[href]");
      if (linkEl && identity.address?.href) {
        linkEl.setAttribute("href", identity.address.href);
      }
    }
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
    const select = root.querySelector(`.dt-filter[data-filter-id="${escapeSelector(filterId)}"]`);
    if (select) {
      if (select.type === "checkbox") {
        const checked = Boolean(value);
        select.checked = checked;
        const status = select.closest(".toggle")?.querySelector(".toggle-state");
        if (status) {
          const onLabel = status.dataset.onLabel || "On";
          const offLabel = status.dataset.offLabel || "All";
          status.textContent = checked ? onLabel : offLabel;
        }
      } else {
        select.value = value ?? "";
      }
    }
  }

  static setSelectOptions(root, selectId, options = [], value) {
    if (!root || !selectId) return;
    const select = root.querySelector(`.dt-filter[data-filter-id="${escapeSelector(selectId)}"]`);
    if (!select || select.type === "checkbox") return;

    select.innerHTML = options
      .map((opt) => {
        const selected = opt.value === value ? " selected" : "";
        const disabled = opt.disabled ? " disabled" : "";
        return `<option value="${escapeHtml(opt.value)}"${selected}${disabled}>${escapeHtml(
          opt.label
        )}</option>`;
      })
      .join("");
    select.value = value ?? "";
    select
      .closest(".table-toolbar-field")
      ?.style.setProperty("--table-toolbar-select-label-width", selectLabelWidth(options));
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
