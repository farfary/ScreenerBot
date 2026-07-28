/**
 * Image Lightbox — zoom a token logo or banner into a centered overlay.
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
 * @param {"logo"|"banner"} [opts.mediaType] - Controls the image frame geometry.
 * @returns {(() => void)|undefined}
 */
export function showImageLightbox({
  imageUrl,
  symbol = "",
  name = "",
  stats = [],
  mediaType = "logo",
} = {}) {
  if (!imageUrl) return undefined;

  const lightbox = document.createElement("div");
  lightbox.className = "image-lightbox";
  lightbox.dataset.state = "opening";
  const normalizedMediaType = mediaType === "banner" ? "banner" : "logo";

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

  // The toolbar is a direct child of the lightbox (NOT inside .lightbox-stage):
  // the stage has a transform animation, and a transformed ancestor becomes the
  // containing block for position:fixed children — so a toolbar inside it would
  // ride over the image during the animation, then snap to the viewport corner
  // once the transform clears. Keeping it outside pins it to the viewport always.
  lightbox.innerHTML = `
    <div class="lightbox-backdrop"></div>
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
    <div class="lightbox-stage" data-media-type="${normalizedMediaType}">
      <div class="lightbox-image-wrapper">
        <img src="${Utils.escapeHtml(imageUrl)}" alt="${Utils.escapeHtml(symbol || name || "Image")}" class="lightbox-image" draggable="false" />
      </div>
      ${captionHtml}
    </div>
  `;

  document.body.appendChild(lightbox);
  const previousBodyOverflow = document.body.style.overflow;
  document.body.style.overflow = "hidden";
  requestAnimationFrame(() => {
    if (closed) return;
    lightbox.classList.add("active");
    lightbox.dataset.state = "open";
  });

  const image = lightbox.querySelector(".lightbox-image");
  const view = { scale: 1, x: 0, y: 0 };
  const drag = {
    active: false,
    moved: false,
    pointerId: 0,
    startX: 0,
    startY: 0,
    originX: 0,
    originY: 0,
  };
  let closed = false;

  const renderView = () => {
    image.style.transform = `translate3d(${view.x}px, ${view.y}px, 0) scale(${view.scale})`;
  };

  const wheelHandler = (event) => {
    event.preventDefault();
    event.stopPropagation();
    const delta = Math.abs(event.deltaY) >= Math.abs(event.deltaX) ? event.deltaY : event.deltaX;
    if (delta === 0) return;

    const nextScale = Math.min(5, Math.max(1, view.scale * Math.exp(-delta * 0.002)));
    if (nextScale <= 1.001) {
      view.scale = 1;
      view.x = 0;
      view.y = 0;
      renderView();
      return;
    }

    const pointerX = event.clientX - window.innerWidth / 2;
    const pointerY = event.clientY - window.innerHeight / 2;
    const ratio = nextScale / view.scale;
    view.x = pointerX - (pointerX - view.x) * ratio;
    view.y = pointerY - (pointerY - view.y) * ratio;
    view.scale = nextScale;
    renderView();
  };

  const pointerDownHandler = (event) => {
    if (view.scale <= 1) return;
    event.stopPropagation();
    image.setPointerCapture(event.pointerId);
    Object.assign(drag, {
      active: true,
      moved: false,
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      originX: view.x,
      originY: view.y,
    });
    image.dataset.dragging = "true";
  };

  const pointerMoveHandler = (event) => {
    if (!drag.active || drag.pointerId !== event.pointerId) return;
    event.stopPropagation();
    const deltaX = event.clientX - drag.startX;
    const deltaY = event.clientY - drag.startY;
    if (Math.hypot(deltaX, deltaY) > 3) drag.moved = true;
    view.x = drag.originX + deltaX;
    view.y = drag.originY + deltaY;
    renderView();
  };

  const endDrag = (event) => {
    if (!drag.active || (event && drag.pointerId !== event.pointerId)) return;
    drag.active = false;
    if (event && image.hasPointerCapture(event.pointerId)) {
      image.releasePointerCapture(event.pointerId);
    }
    image.dataset.dragging = "false";
  };

  const escapeHandler = (e) => {
    if (e.key === "Escape") close();
  };
  document.addEventListener("keydown", escapeHandler);

  const close = () => {
    if (closed) return;
    closed = true;
    lightbox.classList.remove("active");
    lightbox.dataset.state = "closing";
    document.removeEventListener("keydown", escapeHandler);
    lightbox.removeEventListener("wheel", wheelHandler);
    image.removeEventListener("pointerdown", pointerDownHandler);
    image.removeEventListener("pointermove", pointerMoveHandler);
    image.removeEventListener("pointerup", endDrag);
    image.removeEventListener("pointercancel", endDrag);
    document.body.style.overflow = previousBodyOverflow;
    setTimeout(
      () => lightbox.remove(),
      window.matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 140
    );
  };

  lightbox.addEventListener("wheel", wheelHandler, { passive: false });
  image.addEventListener("pointerdown", pointerDownHandler);
  image.addEventListener("pointermove", pointerMoveHandler);
  image.addEventListener("pointerup", endDrag);
  image.addEventListener("pointercancel", endDrag);
  image.addEventListener("click", (event) => {
    event.stopPropagation();
    if (drag.moved) {
      drag.moved = false;
      return;
    }
    close();
  });
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
  const ext = (
    url.split(/[?#]/)[0].match(/\.(png|jpe?g|gif|webp|svg)$/i)?.[1] || "png"
  ).toLowerCase();
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
