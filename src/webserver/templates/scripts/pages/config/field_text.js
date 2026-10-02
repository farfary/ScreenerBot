// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Display text of configuration fields, read from the localization catalog.
 *
 * `/api/config/metadata` carries each field's catalog key (`config-<section>-
 * <field>...`). The label is the message value; `hint`, `unit`, `placeholder`
 * and `subject` are message attributes. The metadata carries category and
 * impact as ids; their names are `config-category-<id>` and
 * `config-impact-<id>` messages.
 *
 * Every id built here lives in the `config-` namespace, which the Rust test
 * `config_catalog_covers_fields` keeps complete.
 */

/** Label of a field, by its catalog key. */
export function fieldLabel(key) {
  return I18n.t(key); // l10n-dynamic: config-
}

function fieldAttribute(key, name) {
  return I18n.attr(key, name) ?? undefined; // l10n-dynamic: config-
}

/** Description of a field, or `undefined`. */
export function fieldHint(key) {
  return fieldAttribute(key, "hint");
}

/** Unit of a field's value, or `undefined`. */
export function fieldUnit(key) {
  return fieldAttribute(key, "unit");
}

/**
 * Name of the parameter a lower bound shares with its upper bound ("Liquidity" for
 * "Min Liquidity" and "Max Liquidity"), or `undefined` for a field without a pair.
 */
export function fieldSubject(key) {
  return fieldAttribute(key, "subject");
}

/** Input placeholder of a field, or `undefined`. */
export function fieldPlaceholder(key) {
  return fieldAttribute(key, "placeholder");
}

/** Display name of a top-level config section, from its id (`sol_price`). */
export function sectionLabel(sectionId) {
  return I18n.t("config-section-" + sectionId.toLowerCase().replaceAll("_", "-")); // l10n-dynamic: config-
}

/** Whether a top-level config section has a display name in the catalog. */
export function hasSectionLabel(sectionId) {
  return I18n.has("config-section-" + sectionId.toLowerCase().replaceAll("_", "-")); // l10n-dynamic: config-
}

/** Display name of a category, from its id. */
export function categoryLabel(id) {
  return I18n.t("config-category-" + id); // l10n-dynamic: config-
}

/** Display name of an impact level (`critical`, `high`, `medium`, `low`). */
export function impactLabel(impact) {
  return I18n.t("config-impact-" + String(impact).toLowerCase()); // l10n-dynamic: config-
}
