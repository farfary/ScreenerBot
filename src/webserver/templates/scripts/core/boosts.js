/**
 * Boosts - which tokens their teams paid to promote, and how strongly.
 *
 * A boost is bought on screenerbot.io: a confirmed payment promotes a mint for a
 * fixed window, and enough active boosts unlock the GOLDEN tier. The website owns
 * the money and the thresholds; the app reads the standing from `/api/boosts`,
 * which is the backend's single cached view of that feed.
 *
 * This module is the ONE client-side answer to "is this token boosted, and how do
 * I show it": the featured row, the featured dialog and every token table read it.
 * Two definitions would mean the same token reads gold on one surface and plain on
 * another, which is the exact thing a paying owner would notice first.
 *
 * A boost buys VISIBILITY, never a recommendation, so the treatment is a mark on a
 * normal row - never a change to a number, a score, or a sort the user chose.
 */

const FEED_URL = "/api/boosts";
/** Matches the backend cache window; a paid boost appears within a minute. */
const TTL_MS = 60 * 1000;

/** Fired on `window` whenever the boost map changes. Surfaces re-mark their rows. */
export const BOOSTS_CHANGED_EVENT = "screenerbot:boosts-changed";

/** mint -> { boosts, golden } */
let boostMap = new Map();
let fetchedAt = 0;
let inflight = null;

/**
 * The boost tier of a token OBJECT that already carries its own standing (a
 * featured card from `/api/featured/*`). Returns `"golden"`, `"boosted"` or null.
 *
 * Reading the object rather than the shared map matters: a featured card is
 * rendered from one snapshot, and a card that says `boosts: 500` must render gold
 * even if the separate table feed has not landed yet.
 */
export function boostTier(token) {
  if (!token) return null;
  const boosts = Number(token.boosts) || 0;
  if (boosts <= 0) return null;
  return token.golden ? "golden" : "boosted";
}

/** The boost tier of a mint, from the shared feed. Sync - safe inside `rowClass`. */
export function boostTierForMint(mint) {
  if (!mint) return null;
  return boostTier(boostMap.get(mint));
}

/** A mint's active boost count, or 0 when it is not boosted. */
export function boostCountForMint(mint) {
  if (!mint) return 0;
  return boostMap.get(mint)?.boosts ?? 0;
}

/**
 * The DataTable `rowClass` fragment for a mint. Empty string for organic rows, so
 * it composes with whatever else a page's `rowClass` returns.
 */
export function boostRowClass(mint) {
  const tier = boostTierForMint(mint);
  if (!tier) return "";
  return tier === "golden" ? "boosted-row golden-row" : "boosted-row";
}

/**
 * Render a boost count the way the packs are sold ("10x", "500x").
 */
export function formatBoostCount(boosts) {
  const count = Number(boosts) || 0;
  return count > 0 ? `${count}x` : "";
}

/** True when the two maps disagree on any mint or any standing. */
function differs(next) {
  if (next.size !== boostMap.size) return true;
  for (const [mint, standing] of next) {
    const current = boostMap.get(mint);
    if (!current || current.boosts !== standing.boosts || current.golden !== standing.golden) {
      return true;
    }
  }
  return false;
}

/**
 * Load the boost feed, at most once per TTL and never twice concurrently.
 *
 * A failing feed is not an error the user should see - the app works offline and
 * a boost is decoration on top of it. The last known map simply stands.
 */
export function loadBoosts({ force = false } = {}) {
  const now = Date.now();
  if (!force && fetchedAt && now - fetchedAt < TTL_MS) return Promise.resolve(boostMap);
  if (inflight) return inflight;

  inflight = (async () => {
    try {
      const response = await fetch(FEED_URL);
      if (!response.ok) return boostMap;
      const data = await response.json();
      if (!data || data.success === false || !Array.isArray(data.tokens)) return boostMap;

      const next = new Map();
      data.tokens.forEach((token) => {
        const mint = token?.mint;
        const boosts = Number(token?.boosts) || 0;
        if (!mint || boosts <= 0) return;
        next.set(mint, { boosts, golden: Boolean(token.golden) });
      });

      fetchedAt = Date.now();
      if (differs(next)) {
        boostMap = next;
        window.dispatchEvent(new CustomEvent(BOOSTS_CHANGED_EVENT));
      }
      return boostMap;
    } catch {
      return boostMap;
    } finally {
      inflight = null;
    }
  })();

  return inflight;
}

/**
 * Kick off a load without waiting for it.
 *
 * Callers paint first and re-mark on `BOOSTS_CHANGED_EVENT`: awaiting a fetch
 * before a page's first render is what leaves a panel blank for the whole request.
 */
export function ensureBoosts() {
  void loadBoosts();
}
