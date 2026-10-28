// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for the entry source of a position.
 *
 * Ids are the values of `PositionOrigin::kind` (src/positions/types.rs): `auto`, `copy`,
 * `manual` and `external`, the last for a round derived from wallet history.
 */

/** Message key of each source's label; keep in step with `PositionOrigin::kind`. */
export const ORIGIN_KIND_LABELS = Object.freeze({
  auto: "positions-filter-origin-auto",
  copy: "positions-filter-origin-copy",
  manual: "positions-origin-manual",
  external: "positions-origin-wallet",
});

/** Label of a position's entry source `kind`. */
export function originKindText(kind) {
  return I18n.label(ORIGIN_KIND_LABELS, kind);
}
