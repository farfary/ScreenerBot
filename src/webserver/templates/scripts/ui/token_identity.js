// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Token identity — the ONE way the dashboard turns a mint into something a human
 * can read: logo, symbol, name and the FULL mint address.
 *
 * Every surface that shows an asset (transaction details, balances, ATA rows,
 * activity, dialogs) resolves through here so they cannot disagree. Three rules
 * this module encodes:
 *
 *   1. SOL is not a token we look up — it is rendered from the official Solana
 *      brand logomark shipped with the binary (`/assets/solana/`). The wSOL mint
 *      and native SOL are the SAME asset to a user, so both resolve to it.
 *   2. A mint address is NEVER cropped and never wraps. Shortening a mint is what
 *      makes two different tokens look identical, and a wrapped one cannot be read
 *      at a glance; the address keeps one line and its size fits that line.
 *   3. Identity lookups are cache-first and batched (`/api/tokens/identities`),
 *      which is DB-only on the server — never the external-fetching token detail
 *      route, which a dialog must not trigger for every mint a swap touched.
 */
import * as Utils from "../core/utils.js";

export const SOL_MINT = "So11111111111111111111111111111111111111112";

/** Official Solana brand assets embedded in the binary (solana.com/branding). */
export const SOLANA_ASSETS = {
  logoMark: "/assets/solana/solanaLogoMark.svg",
  wordMark: "/assets/solana/solanaWordMark.svg",
  logo: "/assets/solana/solanaLogo.svg",
  verticalLogo: "/assets/solana/solanaVerticalLogo.svg",
  foundationLogo: "/assets/solana/solanaFoundationLogo.svg",
};

/**
 * Assets whose identity we own rather than look up. SOL must be here: the token
 * DB knows wSOL under whatever symbol a provider gave it, and a swap's SOL leg
 * would otherwise render as an anonymous mint.
 */
const KNOWN_IDENTITIES = {
  [SOL_MINT]: { symbol: "SOL", name: "Solana", logoUrl: SOLANA_ASSETS.logoMark, decimals: 9 },
  EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v: { symbol: "USDC", name: "USD Coin", decimals: 6 },
  Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB: { symbol: "USDT", name: "Tether USD", decimals: 6 },
};

/** mint -> identity. Lives for the page session; identities do not change. */
const identityCache = new Map();
/** mint -> in-flight promise, so N chips for one mint make ONE request. */
const inFlight = new Map();

const UNKNOWN_SYMBOLS = new Set(["UNKNOWN", "NOT_FOUND"]);
const UNKNOWN_NAMES = new Set(["UNKNOWN TOKEN", "TOKEN NOT IN CACHE"]);

/** Return a displayable symbol, or null for an internal missing-value marker. */
export function resolvedTokenSymbol(value) {
  const symbol = typeof value === "string" ? value.trim() : "";
  return symbol && !UNKNOWN_SYMBOLS.has(symbol.toUpperCase()) ? symbol : null;
}

/** Return a displayable name, or null for an internal missing-value marker. */
export function resolvedTokenName(value) {
  const name = typeof value === "string" ? value.trim() : "";
  return name && !UNKNOWN_NAMES.has(name.toUpperCase()) ? name : null;
}

/** True for wSOL and for the native-SOL pseudo-mint the UI uses in transfers. */
export function isSolMint(mint) {
  return mint === SOL_MINT || mint === "SOL" || mint === "native";
}

function makeIdentity(mint, source = {}) {
  const known = KNOWN_IDENTITIES[mint] || {};
  return {
    mint,
    symbol: resolvedTokenSymbol(source.symbol) || known.symbol || null,
    name: resolvedTokenName(source.name) || known.name || null,
    // A known asset's logo (SOL) always wins: it is the brand asset, not a
    // provider's guess at what wSOL looks like.
    logoUrl: known.logoUrl || Utils.resolveTokenLogoUrl(source) || null,
    decimals: source.decimals ?? known.decimals ?? null,
  };
}

/**
 * Identity for a mint from cache/known assets only — never fetches. Always
 * returns an object so a renderer never has to null-check; an unresolved mint
 * simply has no symbol/name yet.
 */
export function getIdentity(mint) {
  if (!mint) return { mint: null, symbol: null, name: null, logoUrl: null, decimals: null };
  if (isSolMint(mint)) return makeIdentity(SOL_MINT);
  return identityCache.get(mint) || makeIdentity(mint);
}

// `/api/tokens/identities` answers at most this many mints per request
// (`MAX_IDENTITIES` in `routes/tokens/identity.rs`); larger sets are split.
const IDENTITY_BATCH_SIZE = 50;

/**
 * Resolve a batch of mints, filling the cache. Returns a Map of mint -> identity
 * for every mint asked for (unknown ones resolve to a bare identity).
 */
export async function resolveIdentities(mints) {
  const wanted = [...new Set((mints || []).filter(Boolean))];
  const missing = wanted.filter(
    (mint) => !isSolMint(mint) && !identityCache.has(mint) && !inFlight.has(mint)
  );

  if (missing.length > 0) {
    const requests = [];
    for (let start = 0; start < missing.length; start += IDENTITY_BATCH_SIZE) {
      const batch = missing.slice(start, start + IDENTITY_BATCH_SIZE);
      const request = fetchIdentities(batch);
      batch.forEach((mint) => inFlight.set(mint, request));
      requests.push(request);
    }
    try {
      await Promise.allSettled(requests);
    } finally {
      missing.forEach((mint) => inFlight.delete(mint));
    }
  }

  // Mints already being fetched by an earlier call: wait for those too.
  const pending = wanted.map((mint) => inFlight.get(mint)).filter(Boolean);
  if (pending.length > 0) await Promise.allSettled(pending);

  return new Map(wanted.map((mint) => [mint, getIdentity(mint)]));
}

/**
 * Fill the identities behind already-rendered token cells: unresolved mints are
 * fetched, and `repaint` runs only when one was, so a poll that brings no new
 * token never redraws. A DataTable passes `() => table.repaintRows()`.
 */
export function resolveTokenCells(mints, repaint) {
  const missing = [...new Set((mints || []).filter(Boolean))].filter(
    (mint) => !isSolMint(mint) && !identityCache.has(mint)
  );
  if (missing.length === 0) return;
  resolveIdentities(missing)
    .then(() => repaint?.())
    .catch(() => {});
}

async function fetchIdentities(mints) {
  try {
    const response = await fetch(
      `/api/tokens/identities?mints=${encodeURIComponent(mints.join(","))}`
    );
    if (!response.ok) return;
    const body = await response.json();
    const identities = body?.identities || {};
    mints.forEach((mint) => {
      // Cache the miss too — an unknown mint must not be re-requested by every
      // chip on the page.
      identityCache.set(mint, makeIdentity(mint, identities[mint] || {}));
    });
  } catch {
    // Offline or a failed lookup is not an error the user needs: the mint itself
    // still renders. Leave the cache empty so a later call can retry.
  }
}

/**
 * Placeholder for an asset with no logo: the first grapheme of `seed` uppercased,
 * or a bare coin glyph when there is no text. The glyph variant carries
 * `token-logo-glyph`, which drops the avatar tile so the icon stays bare.
 */
export function tokenLogoPlaceholder(seed, className) {
  const initial = Array.from(String(seed ?? "").trim())[0];
  if (initial) {
    return `<span class="${className}">${Utils.escapeHtml(initial.toUpperCase())}</span>`;
  }
  return `<span class="${className} token-logo-glyph"><i class="icon-coins" aria-hidden="true"></i></span>`;
}

/** Letter avatar for an asset with no logo — first character of symbol, else mint. */
function logoPlaceholder(identity) {
  return tokenLogoPlaceholder(identity.symbol || identity.mint, "ti-logo-fallback");
}

/**
 * Asset logo. `size` is one of xs | sm | md | lg. A broken provider image falls
 * back to the letter avatar rather than a broken-image glyph.
 *
 * A brand asset (the Solana logomark) is NOT a square token avatar: it is a 101x88
 * glyph on a transparent canvas, so it gets `ti-logo-brand` and remains inset rather
 * than being cropped edge to edge like a provider's square icon.
 */
export function renderTokenLogo(mintOrIdentity, options = {}) {
  const identity =
    typeof mintOrIdentity === "string" ? getIdentity(mintOrIdentity) : mintOrIdentity;
  const size = options.size || "sm";
  const alt = Utils.escapeHtml(identity.symbol || identity.mint || "");
  const brand = isBrandAsset(identity.logoUrl) ? " ti-logo-brand" : "";
  const inner = identity.logoUrl
    ? `<img class="token-logo-artwork" src="${Utils.escapeHtml(identity.logoUrl)}" alt="${alt}" loading="lazy" onerror="this.remove()" />${logoPlaceholder(identity)}`
    : logoPlaceholder(identity);
  return `<span class="ti-logo ti-logo-${size}${brand} token-logo-frame">${inner}</span>`;
}

/** True for the brand assets we ship ourselves (transparent, non-square glyphs). */
function isBrandAsset(logoUrl) {
  return typeof logoUrl === "string" && logoUrl.startsWith("/assets/solana/");
}

/**
 * Asset chip: logo + symbol (+ name). Use wherever an asset is named in prose or
 * a table cell. `showMint` puts symbol and name on the first line and the FULL
 * mint on its own line underneath; `plainMint` drops that mint's link and copy.
 */
export function renderTokenChip(mintOrIdentity, options = {}) {
  const identity =
    typeof mintOrIdentity === "string" ? getIdentity(mintOrIdentity) : mintOrIdentity;
  const { size = "sm", showName = true, showMint = false, plainMint = false } = options;

  const symbol = identity.symbol || I18n.t("tokens-identity-unknown-asset");
  const name = showName && identity.name && identity.name !== identity.symbol ? identity.name : "";
  const withMint = showMint && identity.mint;

  return `
    <span class="ti-chip ti-chip-${size}${withMint ? " ti-chip-addressed" : ""}" data-mint="${Utils.escapeHtml(identity.mint || "")}">
      ${renderTokenLogo(identity, { size })}
      <span class="ti-chip-text">
        <span class="ti-chip-head">
          <span class="ti-chip-symbol token-symbol-type">${Utils.escapeHtml(symbol)}</span>
          ${name ? `<span class="ti-chip-name token-name-type">${Utils.escapeHtml(name)}</span>` : ""}
        </span>
        ${withMint ? renderAddress(identity.mint, { plain: plainMint }) : ""}
      </span>
    </span>
  `;
}

/**
 * A table's token cell: logo and symbol, then the FULL mint on a second line.
 * Fields the row already carries win over the cache, which may not have them
 * yet; a name is shown only when the row supplies one.
 * Pair it with `TOKEN_CELL_MIN_WIDTH` as the column's `minWidth`.
 */
export function renderTokenCell(mint, { symbol = null, name = null, logoUrl = null } = {}) {
  if (!mint) return symbol ? Utils.escapeHtml(symbol) : "—";
  const identity = getIdentity(mint);
  return renderTokenChip(
    {
      ...identity,
      symbol: symbol || identity.symbol,
      name: name || identity.name,
      logoUrl: logoUrl || identity.logoUrl,
    },
    { showName: Boolean(name), showMint: true }
  );
}

const EXPLORER_PATHS = { token: "token", account: "account" };

/** Glyph advance, smallest font size and action width of `.ti-address` (token_identity.css). */
export const ADDRESS_GLYPH_ADVANCE = 0.61;
export const ADDRESS_FLOOR_PX = 9;
export const ADDRESS_ACTIONS_PX = 26;

/**
 * Narrowest width, in CSS pixels, that shows an address of `chars` characters on
 * one line at the floor size: a DataTable column holding addresses uses it as its
 * `minWidth` (plus the cell padding), so a resize can never squeeze one below it.
 */
export function addressFloorWidth(chars = 44, { plain = false } = {}) {
  return Math.ceil(
    chars * ADDRESS_GLYPH_ADVANCE * ADDRESS_FLOOR_PX + (plain ? 0 : ADDRESS_ACTIONS_PX)
  );
}

/** `renderTokenCell` column minimum: small logo and gap, the mint at its floor, cell padding. */
export const TOKEN_CELL_MIN_WIDTH = addressFloorWidth() + 54;

/**
 * The value markup shared by `renderAddress` and `renderSignature`: `shown` is the
 * text on screen, `value` what the explorer link and the copy action carry.
 * Copy is handled by the global `[data-copy]` delegation in core/utils.js.
 */
function valueMarkup(value, shown, { plain, path, isSignature }) {
  const safe = Utils.escapeHtml(value);
  const text = Utils.escapeHtml(shown);
  const chars = `style="--ti-address-chars: ${shown.length}"`;
  const title = shown === value ? "" : ` title="${safe}"`;
  if (plain) {
    return `<span class="ti-address ti-address-plain" ${chars}><span class="ti-address-value" dir="ltr" translate="no"${title}>${text}</span></span>`;
  }
  const copyTitle = Utils.escapeHtml(
    isSignature
      ? I18n.attr("tokens-identity-copy-signature", "title")
      : I18n.attr("tokens-identity-copy-address", "title")
  );
  const copyLabel = Utils.escapeHtml(
    isSignature
      ? I18n.attr("tokens-identity-copy-signature", "aria-label")
      : I18n.attr("tokens-identity-copy-address", "aria-label")
  );
  const linkTitle = title || ` title="${Utils.escapeHtml(I18n.t("links-view-solscan"))}"`;
  return `
    <span class="ti-address" ${chars}>
      <a href="https://solscan.io/${path}/${safe}" target="_blank" rel="noopener" class="ti-address-value" dir="ltr" translate="no"${linkTitle}>${text}</a>
      <button type="button" class="ti-address-copy" data-copy="${safe}" title="${copyTitle}" aria-label="${copyLabel}">
        <i class="icon-copy"></i>
      </button>
    </span>
  `;
}

/**
 * A Solana address (mint, account, pool, program) in FULL on one line, with copy
 * and explorer actions. This is the ONE address renderer: mints and accounts
 * differ only in which explorer page they link to.
 *
 * The value never wraps and is never cropped. `token_identity.css` sizes its font
 * from the width its line actually has (a container query over the character
 * count carried in `--ti-address-chars`), between a floor and the surface's own
 * size, so the layout only has to give the address a line of its own.
 *
 * `plain` renders the bare value without link or copy, for a row that is itself
 * the action (search results).
 */
export function renderAddress(address, options = {}) {
  if (!address) return "—";
  const path = EXPLORER_PATHS[options.explorer] || EXPLORER_PATHS.token;
  return valueMarkup(address, address, { plain: options.plain, path, isSignature: false });
}

/**
 * A transaction signature in its compact `head…tail` form, with copy and explorer
 * actions that carry the full value and a tooltip that shows it. A signature is
 * an identifier a user copies or opens, never one they read, so unlike an address
 * it is not worth a full line of its own.
 */
export function renderSignature(signature, { plain = false } = {}) {
  if (!signature) return "—";
  return valueMarkup(signature, Utils.formatSignatureCompact(signature), {
    plain,
    path: "tx",
    isSignature: true,
  });
}

/**
 * A wallet or account by its name, with the full address on the line below: the
 * table cell for anything a user names (wallets, watched wallets, imports).
 */
export function renderNamedAddress(name, address, { explorer = "account" } = {}) {
  return `<div class="ti-named-address"><span class="ti-named-address-name">${Utils.escapeHtml(name || "—")}</span>${address ? renderAddress(address, { explorer }) : ""}</div>`;
}

/** Inline "logo + symbol" for tight spots (table cells, flow rows). */
export function renderAssetInline(mintOrIdentity, options = {}) {
  const identity =
    typeof mintOrIdentity === "string" ? getIdentity(mintOrIdentity) : mintOrIdentity;
  const size = options.size || "xs";
  return `
    <span class="ti-inline">
      ${renderTokenLogo(identity, { size })}
      <span class="ti-inline-symbol token-symbol-type">${Utils.escapeHtml(identity.symbol || I18n.t("format-unknown"))}</span>
    </span>
  `;
}
