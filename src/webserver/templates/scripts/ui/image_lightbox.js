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

  const symbolHtml = symbol
    ? `<div class="lightbox-token-symbol">${Utils.escapeHtml(symbol)}</div>`
    : "";
  const nameHtml = name
    ? `<div class="lightbox-token-name">${Utils.escapeHtml(name)}</div>`
    : "";

  const statsHtml = (stats || [])
    .filter((s) => s && s.value)
    .map(
      (s) => `
        <div class="lightbox-stat">
          <div class="stat-label">${Utils.escapeHtml(s.label)}</div>
          <div class="stat-value">${Utils.escapeHtml(s.value)}</div>
        </div>`
    )
    .join("");
  const footerHtml = statsHtml ? `<div class="lightbox-footer">${statsHtml}</div>` : "";

  lightbox.innerHTML = `
    <div class="lightbox-backdrop"></div>
    <div class="lightbox-container">
      <div class="lightbox-header">
        ${symbolHtml}
        ${nameHtml}
        <button class="lightbox-close" type="button" title="Close (ESC)">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>
      <div class="lightbox-body">
        <div class="lightbox-image-wrapper">
          <img src="${Utils.escapeHtml(imageUrl)}" alt="Token logo" class="lightbox-image" />
        </div>
      </div>
      ${footerHtml}
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

  return close;
}
