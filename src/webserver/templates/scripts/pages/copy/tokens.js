// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Token identities for the Copy Trading panels: each mint is looked up once, and
// the panel repaints when names and logos arrive.
import { getIdentity, renderTokenChip, resolveIdentities } from "../../ui/token_identity.js";

const requested = new Set();

export function ensureIdentities(mints, onResolved) {
  const missing = [...new Set((mints || []).filter(Boolean))].filter(
    (mint) => !requested.has(mint)
  );
  if (!missing.length) return;
  missing.forEach((mint) => requested.add(mint));
  resolveIdentities(missing)
    .then(() => onResolved?.())
    .catch(() => missing.forEach((mint) => requested.delete(mint)));
}

/**
 * A mint's identity for a table or event row: logo and symbol, then the FULL
 * mint on its own line, so two tokens sharing a symbol are told apart without
 * cropping either address. The mint is plain: the row's button is the action.
 */
export function tokenWithMint(mint) {
  return renderTokenChip(getIdentity(mint), { showName: false, showMint: true, plainMint: true });
}

export function openTokenDetails(mint) {
  if (!mint) return;
  window.dispatchEvent(new CustomEvent("screenerbot:open-token-details", { detail: { mint } }));
}

/** Set a node's HTML only when it changed, so a 5s poll never resets scroll or focus. */
const painted = new WeakMap();
export function paint(node, html) {
  if (!node || painted.get(node) === html) return false;
  painted.set(node, html);
  node.innerHTML = html;
  return true;
}

export function forget(node) {
  if (node) painted.delete(node);
}
