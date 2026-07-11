/**
 * Token Accent - derive a per-token accent hue used to tint billboard cards.
 *
 * Two sources, in order:
 *
 * 1. The token's LOGO. The dominant colour is sampled from the image on a tiny
 *    offscreen canvas. This needs CORS: the image is requested with
 *    `crossOrigin = "anonymous"`, and if the host does not send
 *    Access-Control-Allow-Origin the canvas is "tainted" and `getImageData`
 *    THROWS -- which is expected and simply falls through to (2). Never assume a
 *    third-party CDN allows this.
 *
 * 2. A hash of the mint. Most tokens have no logo at all (and some logos fail
 *    CORS), so there must always be a colour: the mint hashes to a stable hue, so
 *    a given token always looks the same, and cards stay visually distinct instead
 *    of collapsing into one grey wall.
 *
 * Only the HUE (and a coarse saturation) is used. The card applies it as a very
 * light alpha tint over the existing surface tokens, so it reads correctly in both
 * light and dark themes rather than baking in a fixed colour.
 */

// Resolved accents by cache key (logo URL, or "mint:<mint>" when there is no logo).
const accentCache = new Map();

const FALLBACK_SATURATION = 55;
const SAMPLE_SIZE = 16;

/**
 * Stable hue (0-359) derived from an arbitrary string.
 */
function hueFromString(str) {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = (hash << 5) - hash + str.charCodeAt(i);
    hash |= 0; // force 32-bit
  }
  return Math.abs(hash) % 360;
}

/**
 * Convert RGB to HSL, returning only what the tint needs.
 */
function rgbToHs(r, g, b) {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;

  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const delta = max - min;
  const lightness = (max + min) / 2;

  if (delta === 0) return { hue: 0, saturation: 0 };

  const saturation = delta / (1 - Math.abs(2 * lightness - 1));

  let hue;
  if (max === rn) hue = ((gn - bn) / delta) % 6;
  else if (max === gn) hue = (bn - rn) / delta + 2;
  else hue = (rn - gn) / delta + 4;

  hue = Math.round(hue * 60);
  if (hue < 0) hue += 360;

  return { hue, saturation: Math.round(saturation * 100) };
}

/**
 * Average the image's colourful pixels. Near-transparent, near-white and
 * near-black pixels are skipped: logos are overwhelmingly a subject on a white or
 * transparent background, and averaging those in drags every token toward the same
 * washed-out grey.
 */
function dominantAccent(imageData) {
  let r = 0;
  let g = 0;
  let b = 0;
  let count = 0;

  for (let i = 0; i < imageData.length; i += 4) {
    const alpha = imageData[i + 3];
    if (alpha < 128) continue;

    const pr = imageData[i];
    const pg = imageData[i + 1];
    const pb = imageData[i + 2];

    const max = Math.max(pr, pg, pb);
    const min = Math.min(pr, pg, pb);
    if (max > 240 && min > 240) continue; // near-white
    if (max < 24) continue; // near-black
    if (max - min < 16) continue; // greyscale, carries no hue

    r += pr;
    g += pg;
    b += pb;
    count++;
  }

  if (count === 0) return null;

  const { hue, saturation } = rgbToHs(r / count, g / count, b / count);
  if (saturation < 12) return null; // effectively colourless

  // Clamp so a neon logo cannot produce a screaming card tint.
  return { hue, saturation: Math.min(Math.max(saturation, 35), 80) };
}

/**
 * Sample the dominant accent from a logo. Resolves null on any failure
 * (CORS-tainted canvas, load error, colourless image).
 */
function accentFromImage(url) {
  return new Promise((resolve) => {
    const img = document.createElement("img");
    img.crossOrigin = "anonymous";

    img.onload = () => {
      try {
        const canvas = document.createElement("canvas");
        canvas.width = SAMPLE_SIZE;
        canvas.height = SAMPLE_SIZE;

        const ctx = canvas.getContext("2d", { willReadFrequently: true });
        ctx.drawImage(img, 0, 0, SAMPLE_SIZE, SAMPLE_SIZE);

        // Throws a SecurityError when the host sent no CORS header.
        const { data } = ctx.getImageData(0, 0, SAMPLE_SIZE, SAMPLE_SIZE);
        resolve(dominantAccent(data));
      } catch {
        resolve(null);
      }
    };

    img.onerror = () => resolve(null);
    img.src = url;
  });
}

/**
 * The colour a token falls back to when it has no usable logo.
 * @param {string} mint
 */
export function fallbackAccent(mint) {
  return { hue: hueFromString(mint || ""), saturation: FALLBACK_SATURATION };
}

/**
 * Resolve a token's accent, preferring its logo and falling back to the mint hash.
 *
 * @param {string} mint - token mint (always available; drives the fallback)
 * @param {string|null} logoUrl - validated logo URL, or null
 * @returns {Promise<{hue: number, saturation: number}>} always resolves
 */
export async function getTokenAccent(mint, logoUrl) {
  const key = logoUrl || `mint:${mint}`;

  const cached = accentCache.get(key);
  if (cached) return cached;

  let accent = null;
  if (logoUrl) {
    accent = await accentFromImage(logoUrl);
  }
  if (!accent) {
    accent = fallbackAccent(mint);
  }

  accentCache.set(key, accent);
  return accent;
}
