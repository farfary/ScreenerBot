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
 *   2. A mint address is NEVER cropped. Shortening a mint is what makes two
 *      different tokens look identical; the address is always rendered in full
 *      (it wraps rather than truncates).
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
    const request = fetchIdentities(missing);
    missing.forEach((mint) => inFlight.set(mint, request));
    try {
      await request;
    } finally {
      missing.forEach((mint) => inFlight.delete(mint));
    }
  }

  // Mints already being fetched by an earlier call: wait for those too.
  const pending = wanted.map((mint) => inFlight.get(mint)).filter(Boolean);
  if (pending.length > 0) await Promise.allSettled(pending);

  return new Map(wanted.map((mint) => [mint, getIdentity(mint)]));
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

/** Letter avatar for an asset with no logo — first character of symbol, else mint. */
function logoPlaceholder(identity) {
  const seed = identity.symbol || identity.mint || "?";
  return `<span class="ti-logo-fallback">${Utils.escapeHtml(seed.charAt(0).toUpperCase())}</span>`;
}

/**
 * Asset logo. `size` is one of xs | sm | md | lg. A broken provider image falls
 * back to the letter avatar rather than a broken-image glyph.
 *
 * A brand asset (the Solana logomark) is NOT a square token avatar: it is a 101x88
 * glyph on a transparent canvas, so it gets `ti-logo-brand` — inset and letterboxed
 * inside the circle instead of cropped edge to edge like a provider's square icon.
 */
export function renderTokenLogo(mintOrIdentity, options = {}) {
  const identity =
    typeof mintOrIdentity === "string" ? getIdentity(mintOrIdentity) : mintOrIdentity;
  const size = options.size || "sm";
  const alt = Utils.escapeHtml(identity.symbol || identity.mint || "");
  const brand = isBrandAsset(identity.logoUrl) ? " ti-logo-brand" : "";
  const inner = identity.logoUrl
    ? `<img src="${Utils.escapeHtml(identity.logoUrl)}" alt="${alt}" loading="lazy" onerror="this.remove()" />${logoPlaceholder(identity)}`
    : logoPlaceholder(identity);
  return `<span class="ti-logo ti-logo-${size}${brand}">${inner}</span>`;
}

/** True for the brand assets we ship ourselves (transparent, non-square glyphs). */
function isBrandAsset(logoUrl) {
  return typeof logoUrl === "string" && logoUrl.startsWith("/assets/solana/");
}

/**
 * Asset chip: logo + symbol (+ name). Use wherever an asset is named in prose or
 * a table cell. `showMint` appends the FULL mint underneath.
 */
export function renderTokenChip(mintOrIdentity, options = {}) {
  const identity =
    typeof mintOrIdentity === "string" ? getIdentity(mintOrIdentity) : mintOrIdentity;
  const { size = "sm", showName = true, showMint = false } = options;

  const symbol = identity.symbol || "Unknown asset";
  const name = showName && identity.name && identity.name !== identity.symbol ? identity.name : "";

  return `
    <span class="ti-chip ti-chip-${size}" data-mint="${Utils.escapeHtml(identity.mint || "")}">
      ${renderTokenLogo(identity, { size })}
      <span class="ti-chip-text">
        <span class="ti-chip-symbol token-symbol-type">${Utils.escapeHtml(symbol)}</span>
        ${name ? `<span class="ti-chip-name token-name-type">${Utils.escapeHtml(name)}</span>` : ""}
        ${showMint && identity.mint ? renderAddress(identity.mint) : ""}
      </span>
    </span>
  `;
}

const EXPLORER_PATHS = { token: "token", account: "account", tx: "tx" };

/**
 * A Solana address (mint, account, pool, program) or a signature, in FULL, with copy
 * and explorer actions. This is the ONE address renderer — mints, accounts and
 * signatures differ only in which explorer page they link to, so they must not be
 * three near-identical snippets that drift apart in styling. Never shortened: it wraps.
 * Copy is handled by the global `[data-copy]` delegation in core/utils.js.
 */
export function renderAddress(address, options = {}) {
  if (!address) return "—";
  const explorer = EXPLORER_PATHS[options.explorer] || EXPLORER_PATHS.token;
  const safe = Utils.escapeHtml(address);
  const label = options.explorer === "tx" ? "signature" : "address";
  return `
    <span class="ti-address">
      <a href="https://solscan.io/${explorer}/${safe}" target="_blank" rel="noopener" class="ti-address-value" title="View on Solscan">${safe}</a>
      <button type="button" class="ti-address-copy" data-copy="${safe}" title="Copy ${label}" aria-label="Copy ${label}">
        <i class="icon-copy"></i>
      </button>
    </span>
  `;
}

/** Inline "logo + symbol" for tight spots (table cells, flow rows). */
export function renderAssetInline(mintOrIdentity, options = {}) {
  const identity =
    typeof mintOrIdentity === "string" ? getIdentity(mintOrIdentity) : mintOrIdentity;
  const size = options.size || "xs";
  return `
    <span class="ti-inline">
      ${renderTokenLogo(identity, { size })}
      <span class="ti-inline-symbol token-symbol-type">${Utils.escapeHtml(identity.symbol || "Unknown")}</span>
    </span>
  `;
}
