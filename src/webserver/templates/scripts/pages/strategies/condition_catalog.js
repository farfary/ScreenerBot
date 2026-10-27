// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Condition catalog dialog - category rail, search and condition buttons that add a condition to the group that opened it.

import { categoryLabel, conditionDescription, conditionName } from "./condition_text.js";

/** Message ids of the data tags, by the `reads` value of a condition schema. */
export const DATA_TAG_LABELS = Object.freeze({
  candles: "strategies-data-candles",
  pool: "strategies-data-pool",
  position: "strategies-data-position",
  market: "strategies-data-market",
  clock: "strategies-data-clock",
});

/** Message ids of the line a condition shows when the strategy type cannot use it. */
const RESTRICTION_LABELS = Object.freeze({
  EXIT: "strategies-catalog-exit-only",
  ENTRY: "strategies-catalog-entry-only",
});

/** Whether a condition can be used in a strategy of `strategyType`. */
export function appliesTo(schema, strategyType) {
  return !Array.isArray(schema.applies_to) || schema.applies_to.includes(strategyType);
}

/**
 * The catalog description of a condition. A candle condition's description is closed
 * by the note that it reads candles only, written here once for every condition whose
 * schema `reads` candles rather than in each condition's message.
 */
export function catalogDescription(schema) {
  const description = conditionDescription(schema);
  return schema.reads === "candles"
    ? I18n.t("strategies-catalog-candles-note", { description }).trim()
    : description;
}

/** Catalog entries of the strategy-origin schemas, by name. */
export function catalogEntries(schemas, strategyType) {
  return Object.entries(schemas ?? {})
    .filter(([, schema]) => !schema.origin || String(schema.origin).toLowerCase() === "strategy")
    .map(([type, schema]) => ({
      type,
      icon: schema.icon || "icon-puzzle",
      category: schema.category || "General",
      categoryName: categoryLabel(schema),
      name: conditionName(schema, type),
      description: catalogDescription(schema),
      tag: schema.reads,
      applicable: appliesTo(schema, strategyType),
      only: schema.applies_to?.length === 1 ? schema.applies_to[0] : null,
    }))
    .sort((a, b) => a.name.localeCompare(b.name));
}

/** Entries of one category (`null` for all) whose name or description contains `query`. */
export function filterEntries(entries, { category = null, query = "" } = {}) {
  const needle = query.trim().toLocaleLowerCase();
  return entries.filter(
    (entry) =>
      (category === null || entry.category === category) &&
      (needle === "" ||
        entry.name.toLocaleLowerCase().includes(needle) ||
        entry.description.toLocaleLowerCase().includes(needle))
  );
}

/** Categories of the entries by name, each with its count. */
export function categoriesOf(entries) {
  const found = new Map();
  for (const entry of entries) {
    const known = found.get(entry.category);
    if (known) known.count += 1;
    else found.set(entry.category, { category: entry.category, name: entry.categoryName, count: 1 });
  }
  return [...found.values()].sort((a, b) => a.name.localeCompare(b.name));
}

export function createConditionCatalog({
  conditionSchemas,
  $,
  Utils,
  addTrackedListener,
}) {
  const escape = (text) => Utils.escapeHtml(String(text ?? ""));
  let entries = [];
  let category = null;
  let onPick = null;
  let opener = null;

  /** One condition as a button: glyph, name, description, what it reads and why it is off. */
  function renderEntry(entry) {
    const note =
      !entry.applicable && Object.hasOwn(RESTRICTION_LABELS, entry.only)
        ? `<span class="catalog-entry-note">${escape(I18n.label(RESTRICTION_LABELS, entry.only))}</span>`
        : "";
    return `
      <button type="button" class="catalog-entry" data-condition-type="${escape(entry.type)}"${entry.applicable ? "" : " disabled"}>
        <i class="catalog-entry-glyph ${escape(entry.icon)}" aria-hidden="true"></i>
        <span class="catalog-entry-main">
          <span class="catalog-entry-name">${escape(entry.name)}</span>
          <span class="catalog-entry-desc" title="${escape(entry.description)}">${escape(entry.description || I18n.t("strategies-catalog-no-description"))}</span>
          ${note}
        </span>
        <span class="catalog-entry-tag">${escape(I18n.label(DATA_TAG_LABELS, entry.tag))}</span>
      </button>`;
  }

  function renderRail() {
    const item = (value, name, count) => `
      <button type="button" class="catalog-category" data-category="${escape(value ?? "")}" aria-pressed="${category === value}">
        <span class="catalog-category-name">${escape(name)}</span>
        <span class="catalog-category-count">${count}</span>
      </button>`;
    return [
      item(null, I18n.t("strategies-catalog-all"), entries.length),
      ...categoriesOf(entries).map(({ category: value, name, count }) => item(value, name, count)),
    ].join("");
  }

  function visibleEntries() {
    return filterEntries(entries, { category, query: $("#condition-search")?.value ?? "" });
  }

  function render() {
    const rail = $("#catalog-categories");
    if (rail) rail.innerHTML = renderRail();
    const listEl = $("#catalog-list");
    if (!listEl) return;
    const shown = visibleEntries();
    listEl.innerHTML = shown.length
      ? shown.map(renderEntry).join("")
      : `<div class="catalog-empty">${escape(I18n.t("strategies-catalog-empty"))}</div>`;
  }

  function close() {
    const modal = $("#condition-catalog-modal");
    if (modal) modal.hidden = true;
    onPick = null;
    opener?.focus?.();
    opener = null;
  }

  function pick(type) {
    const handler = onPick;
    close();
    handler?.(type);
  }

  /**
   * Show the catalog for a strategy type; `handler` receives the type of the condition
   * the user picks.
   */
  function open({ strategyType, handler }) {
    entries = catalogEntries(conditionSchemas, strategyType);
    category = null;
    onPick = handler;
    opener = globalThis.document?.activeElement ?? null;
    const search = $("#condition-search");
    if (search) search.value = "";
    render();
    const modal = $("#condition-catalog-modal");
    if (modal) modal.hidden = false;
    search?.focus();
  }

  /** Wire the dialog once; its list and rail are redrawn on every search and category. */
  function bind() {
    const modal = $("#condition-catalog-modal");
    const search = $("#condition-search");
    addTrackedListener($("#close-condition-catalog"), "click", close);
    addTrackedListener(modal, "click", (event) => {
      if (event.target === modal) close();
    });
    addTrackedListener(modal, "keydown", (event) => {
      if (event.key === "Escape" && !modal.hidden) {
        event.stopPropagation();
        close();
      }
    });
    addTrackedListener(search, "input", render);
    addTrackedListener(search, "keydown", (event) => {
      if (event.key !== "Enter") return;
      const matches = visibleEntries().filter((entry) => entry.applicable);
      if (matches.length === 1) {
        event.preventDefault();
        pick(matches[0].type);
      }
    });
    addTrackedListener($("#clear-search"), "click", () => {
      if (search) search.value = "";
      render();
      search?.focus();
    });
    addTrackedListener($("#catalog-categories"), "click", (event) => {
      const button = event.target.closest("[data-category]");
      if (!button) return;
      category = button.dataset.category === "" ? null : button.dataset.category;
      render();
    });
    addTrackedListener($("#catalog-list"), "click", (event) => {
      const button = event.target.closest(".catalog-entry");
      if (button && !button.disabled) pick(button.dataset.conditionType);
    });
  }

  return { bind, open, close, renderEntry, render };
}
