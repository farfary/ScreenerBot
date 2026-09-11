// Token identities for the Copy Trading panels: each mint is looked up once, and
// the panel repaints when names and logos arrive.
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

/** A mint's inline identity; without metadata it reads as its short mint, not "Unknown". */
export function tokenInline(mint) {
  const identity = getIdentity(mint);
  return renderAssetInline(
    identity.symbol ? identity : { ...identity, symbol: shortAddress(mint) }
  );
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
