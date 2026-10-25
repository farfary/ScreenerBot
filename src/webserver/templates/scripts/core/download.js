// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// File downloads - saves a blob or a JSON value to the user's disk under a given file name.

/**
 * Save `blob` as `filename` through a temporary anchor.
 * @param {string} filename
 * @param {Blob} blob
 */
export function downloadBlob(filename, blob) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  // The browser reads the blob after the click returns.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/**
 * Save `value` as pretty-printed JSON named `filename`.
 * @param {string} filename
 * @param {unknown} value
 */
export function downloadJson(filename, value) {
  downloadBlob(filename, new Blob([JSON.stringify(value, null, 2)], { type: "application/json" }));
}
