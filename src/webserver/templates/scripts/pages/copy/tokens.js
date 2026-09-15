// Token identities for the Copy Trading panels: each mint is looked up once, and
// the panel repaints when names and logos arrive.
import { escapeHtml } from "../../core/utils.js";
import { getIdentity, renderAssetInline, resolveIdentities } from "../../ui/token_identity.js";
import { shortAddress } from "./format.js";

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

/** The symbols more than one of these mints resolves to. */
export function sharedSymbols(mints) {
  const counts = new Map();
  new Set((mints || []).filter(Boolean)).forEach((mint) => {
    const symbol = getIdentity(mint).symbol;
    if (symbol) counts.set(symbol, (counts.get(symbol) || 0) + 1);
  });
  return new Set([...counts].filter(([, count]) => count > 1).map(([symbol]) => symbol));
}

/**
 * A mint's inline identity; without metadata it reads as its short mint, not
 * "Unknown". A symbol in `shared` names several tokens, so the mint follows it.
 */
export function tokenInline(mint, shared = null) {
  const identity = getIdentity(mint);
  const inline = renderAssetInline(
    identity.symbol ? identity : { ...identity, symbol: shortAddress(mint) }
  );
  return identity.symbol && shared?.has(identity.symbol)
    ? `${inline}<small class="copy-token-mint">${escapeHtml(shortAddress(mint))}</small>`
    : inline;
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
