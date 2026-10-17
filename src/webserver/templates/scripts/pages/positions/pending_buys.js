// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Which finished buys of the Positions table already have a position. A buy shows as a
// pending row until its position loads; once that position loaded, open or closed since,
// the buy is never pending again.

/** Records the latest entry time (ms) per mint of every loaded position row. */
export const rememberEntries = (latestEntryMsByMint, rows) => {
  rows.forEach((row) => {
    const entryMs = Number(row?.entry_time) * 1000;
    if (!row?.mint || !Number.isFinite(entryMs)) return;
    const known = latestEntryMsByMint.get(row.mint);
    if (!Number.isFinite(known) || entryMs > known) {
      latestEntryMsByMint.set(row.mint, entryMs);
    }
  });
};

/**
 * True when the finished buy `info` ({ kind, startedAt }) already has its position: a row
 * of its mint entered at or after the buy started was loaded. An in-progress buy is
 * always pending.
 */
export const buyHasLoadedPosition = (info, latestEntryMs) =>
  info.kind !== "active" &&
  Number.isFinite(info.startedAt) &&
  Number.isFinite(latestEntryMs) &&
  latestEntryMs >= info.startedAt;
