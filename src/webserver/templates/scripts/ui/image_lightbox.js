/**
 * Image Lightbox — zoom a token logo (or any image) into a centered overlay.
 *
 * Shared by the tokens data table (row logo click) and the token details dialog
 * (header logo click). Styles live in the global components.css (.image-lightbox
 * and friends), so the same markup works anywhere.
 */
import * as Utils from "../core/utils.js";

/**
 * Show the lightbox. Returns a close() function.
 *
 * @param {object} opts
 * @param {string} opts.imageUrl - Image to display (required; no-op if empty).
 * @param {string} [opts.symbol] - Token symbol shown in the header.
 * @param {string} [opts.name] - Token name shown in the header.
 * @param {Array<{label:string,value:string}>} [opts.stats] - Optional footer stats.
 * @returns {(() => void)|undefined}
 */
export function showImageLightbox({ imageUrl, symbol = "", name = "", stats = [] } = {}) {
  if (!imageUrl) return undefined;

  const lightbox = document.createElement("div");
  lightbox.className = "image-lightbox";

  // Minimal, image-focused: floating save/close controls, one subtle caption
  // line (symbol · name · optional stats) — no heavy header or footer.
  const metaText = (stats || [])
    .filter((s) => s && s.value)
    .map((s) => `${s.label}: ${s.value}`)
    .join("  ·  ");

  const captionParts = [
    symbol ? `<span class="lightbox-caption-symbol">${Utils.escapeHtml(symbol)}</span>` : "",
    name ? `<span class="lightbox-caption-name">${Utils.escapeHtml(name)}</span>` : "",
    metaText ? `<span class="lightbox-caption-meta">${Utils.escapeHtml(metaText)}</span>` : "",
  ].join("");
  const captionHtml = captionParts ? `<div class="lightbox-caption">${captionParts}</div>` : "";

  lightbox.innerHTML = `
    <div class="lightbox-backdrop"></div>
    <div class="lightbox-stage">
      <div class="lightbox-toolbar">
        <button class="lightbox-btn lightbox-save" type="button" title="Save image">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="7 10 12 15 17 10"></polyline>
            <line x1="12" y1="15" x2="12" y2="3"></line>
          </svg>
        </button>
        <button class="lightbox-btn lightbox-close" type="button" title="Close (ESC)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>
      <div class="lightbox-image-wrapper">
        <img src="${Utils.escapeHtml(imageUrl)}" alt="${Utils.escapeHtml(symbol || name || "Image")}" class="lightbox-image" />
      </div>
      ${captionHtml}
    </div>
  `;

  document.body.appendChild(lightbox);
  requestAnimationFrame(() => lightbox.classList.add("active"));

  const escapeHandler = (e) => {
    if (e.key === "Escape") close();
  };
  document.addEventListener("keydown", escapeHandler);

  const close = () => {
    lightbox.classList.remove("active");
    document.removeEventListener("keydown", escapeHandler);
    setTimeout(() => lightbox.remove(), 300);
  };

  lightbox.querySelector(".lightbox-close").addEventListener("click", close);
  lightbox.querySelector(".lightbox-backdrop").addEventListener("click", close);
  lightbox.querySelector(".lightbox-save").addEventListener("click", (e) => {
    e.stopPropagation();
    saveImage(imageUrl, symbol || name);
  });

  return close;
}

/**
 * Download the image. Fetches it as a blob so it saves directly; falls back to
 * opening it in a new tab if the fetch is blocked (cross-origin without CORS).
 */
async function saveImage(url, label) {
  const base = (label || "image").replace(/[^\w.-]+/g, "_") || "image";
  const ext = (url.split(/[?#]/)[0].match(/\.(png|jpe?g|gif|webp|svg)$/i)?.[1] || "png").toLowerCase();
  const filename = `${base}.${ext}`;
  try {
    const res = await fetch(url, { mode: "cors" });
    if (!res.ok) throw new Error("fetch failed");
    const blob = await res.blob();
    const objUrl = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = objUrl;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(objUrl), 1000);
  } catch {
    // Cross-origin image without CORS headers — open it so the user can save it.
    window.open(url, "_blank", "noopener");
  }
}
