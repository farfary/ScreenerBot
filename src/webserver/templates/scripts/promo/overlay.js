/**
 * Promo capture overlay layer.
 *
 * The annotation vocabulary a promotional screenshot or video needs: a pointed
 * callout anchored to real UI, a spotlight that dims everything else, a synthetic
 * cursor that travels and clicks, a lower-third caption, and a full-screen title
 * card. Everything is built from the dashboard's own surface, type and spacing
 * tokens so an annotated frame still looks like the product.
 *
 * Every function resolves on the Web Animations API's `finished` promise, so the
 * driver knows an annotation is fully on screen — it never waits out a duration
 * it guessed.
 */

const LAYER_ID = "promoCaptureLayer";
const CURSOR_ID = "promoCaptureCursor";

const registry = new Map();
let sequence = 0;

function motionScale() {
  const raw = getComputedStyle(document.documentElement).getPropertyValue("--promo-motion-scale");
  const value = Number.parseFloat(raw);
  return Number.isFinite(value) && value >= 0 ? value : 1;
}

function layer() {
  let el = document.getElementById(LAYER_ID);
  if (!el) {
    el = document.createElement("div");
    el.id = LAYER_ID;
    el.className = "promo-layer";
    el.setAttribute("aria-hidden", "true");
    document.body.appendChild(el);
  }
  return el;
}

function nextId(prefix) {
  sequence += 1;
  return `${prefix}-${sequence}`;
}

/** Await an animation, tolerating a cancel so a cleared annotation never hangs. */
async function settled(animation) {
  try {
    await animation.finished;
  } catch {
    /* cancelled by clear() — the caller only needs to stop waiting */
  }
}

function rectOf(target) {
  if (target && typeof target === "object" && "width" in target) return target;
  const el = document.querySelector(target);
  if (!el) throw new Error(`Overlay target not found: ${target}`);
  const rect = el.getBoundingClientRect();
  return { top: rect.top, left: rect.left, width: rect.width, height: rect.height };
}

function enter(el, { from = "translateY(6px)" } = {}) {
  return el.animate(
    [
      { opacity: 0, transform: from },
      { opacity: 1, transform: "translateY(0)" },
    ],
    { duration: 260 * motionScale(), easing: "cubic-bezier(0.22, 1, 0.36, 1)", fill: "both" }
  );
}

function exit(el) {
  return el.animate([{ opacity: 1 }, { opacity: 0 }], {
    duration: 180 * motionScale(),
    easing: "ease-out",
    fill: "both",
  });
}

// ---------------------------------------------------------------------------
// Callout
// ---------------------------------------------------------------------------

/**
 * Place the panel beside its target, flipping to the opposite side when the
 * preferred one would leave the viewport and clamping along the free axis. The
 * pointer is then aimed at the target's centre independently of that clamping,
 * so a callout pushed sideways still points at the thing it describes.
 */
function place(panel, rect, placement, gap = 14) {
  const panelRect = panel.getBoundingClientRect();
  const margin = 16;
  const vw = window.innerWidth;
  const vh = window.innerHeight;

  const room = {
    top: rect.top,
    bottom: vh - (rect.top + rect.height),
    left: rect.left,
    right: vw - (rect.left + rect.width),
  };
  const needed = {
    top: panelRect.height + gap + margin,
    bottom: panelRect.height + gap + margin,
    left: panelRect.width + gap + margin,
    right: panelRect.width + gap + margin,
  };

  let side = placement;
  if (room[side] < needed[side]) {
    const opposite = { top: "bottom", bottom: "top", left: "right", right: "left" }[side];
    if (room[opposite] >= needed[opposite]) side = opposite;
    else side = Object.keys(room).sort((a, b) => room[b] - needed[b] - (room[a] - needed[a]))[0];
  }

  let top;
  let left;
  if (side === "top" || side === "bottom") {
    top = side === "top" ? rect.top - panelRect.height - gap : rect.top + rect.height + gap;
    left = rect.left + rect.width / 2 - panelRect.width / 2;
    left = Math.min(Math.max(margin, left), vw - panelRect.width - margin);
  } else {
    left = side === "left" ? rect.left - panelRect.width - gap : rect.left + rect.width + gap;
    top = rect.top + rect.height / 2 - panelRect.height / 2;
    top = Math.min(Math.max(margin, top), vh - panelRect.height - margin);
  }

  panel.style.top = `${Math.round(top)}px`;
  panel.style.left = `${Math.round(left)}px`;
  panel.dataset.side = side;

  const pointer = panel.querySelector(".promo-callout-pointer");
  if (pointer) {
    if (side === "top" || side === "bottom") {
      const x = rect.left + rect.width / 2 - left;
      pointer.style.left = `${Math.min(Math.max(18, x), panelRect.width - 18)}px`;
      pointer.style.top = "";
    } else {
      const y = rect.top + rect.height / 2 - top;
      pointer.style.top = `${Math.min(Math.max(18, y), panelRect.height - 18)}px`;
      pointer.style.left = "";
    }
  }
}

/**
 * A pointed annotation panel anchored to a live element.
 *
 * @param {string|object} target  selector, or a viewport rect
 * @param {string} [eyebrow]      small label above the title (e.g. "Step 2")
 * @param {string} title
 * @param {string} [body]
 * @param {"top"|"bottom"|"left"|"right"} [placement]
 * @param {string} [tone]         "neutral" | "buy" | "sell"
 * @param {boolean} [follow]      keep the panel pinned while the target moves
 */
export async function callout({
  target,
  selector,
  eyebrow = "",
  title = "",
  body = "",
  placement = "bottom",
  tone = "neutral",
  follow = false,
  id = null,
} = {}) {
  const anchor = target || selector;
  const rect = rectOf(anchor);
  const panelId = id || nextId("callout");

  clear({ id: panelId });

  const panel = document.createElement("div");
  panel.className = "promo-callout";
  panel.dataset.tone = tone;
  panel.id = panelId;
  panel.innerHTML = `
    <span class="promo-callout-pointer" aria-hidden="true"></span>
    ${eyebrow ? '<p class="promo-callout-eyebrow"></p>' : ""}
    ${title ? '<p class="promo-callout-title"></p>' : ""}
    ${body ? '<p class="promo-callout-body"></p>' : ""}
  `;
  // Text is assigned rather than interpolated: scene copy is authored freely and
  // must never be able to inject markup into the captured frame.
  if (eyebrow) panel.querySelector(".promo-callout-eyebrow").textContent = eyebrow;
  if (title) panel.querySelector(".promo-callout-title").textContent = title;
  if (body) panel.querySelector(".promo-callout-body").textContent = body;

  layer().appendChild(panel);
  place(panel, rect, placement);

  const entry = { el: panel, kind: "callout", raf: null };
  registry.set(panelId, entry);

  if (follow && typeof anchor === "string") {
    const track = () => {
      if (!registry.has(panelId)) return;
      try {
        place(panel, rectOf(anchor), placement);
      } catch {
        /* target left the DOM; the panel stays where it was until cleared */
      }
      entry.raf = requestAnimationFrame(track);
    };
    entry.raf = requestAnimationFrame(track);
  }

  await settled(enter(panel));
  return { id: panelId, side: panel.dataset.side };
}

// ---------------------------------------------------------------------------
// Spotlight
// ---------------------------------------------------------------------------

/**
 * Dim the whole window except one rectangle. Built from a single element with a
 * huge spread box-shadow, so there is no second full-screen paint to keep in sync
 * with the cut-out and no SVG mask to scale wrong on a HiDPI capture.
 */
export async function spotlight({
  target,
  selector,
  padding = 8,
  radius = 10,
  id = "promo-spotlight",
} = {}) {
  const rect = rectOf(target || selector);
  clear({ id });

  const hole = document.createElement("div");
  hole.className = "promo-spotlight";
  hole.id = id;
  hole.style.top = `${rect.top - padding}px`;
  hole.style.left = `${rect.left - padding}px`;
  hole.style.width = `${rect.width + padding * 2}px`;
  hole.style.height = `${rect.height + padding * 2}px`;
  hole.style.borderRadius = `${radius}px`;

  layer().appendChild(hole);
  registry.set(id, { el: hole, kind: "spotlight" });

  await settled(
    hole.animate([{ opacity: 0 }, { opacity: 1 }], {
      duration: 240 * motionScale(),
      easing: "ease-out",
      fill: "both",
    })
  );
  return { id };
}

// ---------------------------------------------------------------------------
// Caption and title card
// ---------------------------------------------------------------------------

/** A lower-third (or upper-third) narration line for a recording. */
export async function caption({
  text = "",
  sub = "",
  position = "bottom",
  id = "promo-caption",
} = {}) {
  clear({ id });

  const el = document.createElement("div");
  el.className = "promo-caption";
  el.id = id;
  el.dataset.position = position;
  el.innerHTML = `<p class="promo-caption-text"></p>${sub ? '<p class="promo-caption-sub"></p>' : ""}`;
  el.querySelector(".promo-caption-text").textContent = text;
  if (sub) el.querySelector(".promo-caption-sub").textContent = sub;

  layer().appendChild(el);
  registry.set(id, { el, kind: "caption" });

  await settled(
    enter(el, { from: position === "bottom" ? "translateY(12px)" : "translateY(-12px)" })
  );
  return { id };
}

/** A full-screen card for the open and close of a promotional clip. */
export async function titleCard({ title = "", sub = "", id = "promo-title-card" } = {}) {
  clear({ id });

  const el = document.createElement("div");
  el.className = "promo-title-card";
  el.id = id;
  el.innerHTML = `<p class="promo-title-card-title"></p>${
    sub ? '<p class="promo-title-card-sub"></p>' : ""
  }`;
  el.querySelector(".promo-title-card-title").textContent = title;
  if (sub) el.querySelector(".promo-title-card-sub").textContent = sub;

  layer().appendChild(el);
  registry.set(id, { el, kind: "titleCard" });

  await settled(
    el.animate([{ opacity: 0 }, { opacity: 1 }], {
      duration: 420 * motionScale(),
      easing: "ease-out",
      fill: "both",
    })
  );
  return { id };
}

// ---------------------------------------------------------------------------
// Cursor
// ---------------------------------------------------------------------------

function cursorEl() {
  let el = document.getElementById(CURSOR_ID);
  if (!el) {
    el = document.createElement("div");
    el.id = CURSOR_ID;
    el.className = "promo-cursor";
    el.innerHTML = '<span class="promo-cursor-ring" aria-hidden="true"></span>';
    el.style.transform = "translate3d(-100px, -100px, 0)";
    layer().appendChild(el);
  }
  return el;
}

let cursorPosition = { x: -100, y: -100 };

export async function cursorShow({ x = null, y = null } = {}) {
  const el = cursorEl();
  if (x !== null && y !== null) {
    cursorPosition = { x, y };
    el.style.transform = `translate3d(${x}px, ${y}px, 0)`;
  }
  el.classList.add("is-visible");
  await settled(
    el.animate([{ opacity: 0 }, { opacity: 1 }], {
      duration: 180 * motionScale(),
      easing: "ease-out",
      fill: "both",
    })
  );
  return { x: cursorPosition.x, y: cursorPosition.y };
}

export async function cursorHide() {
  const el = document.getElementById(CURSOR_ID);
  if (!el) return { hidden: true };
  await settled(exit(el));
  el.classList.remove("is-visible");
  return { hidden: true };
}

/**
 * Move the promo cursor to a point or an element along an eased path. The travel
 * is one Web Animation, so the driver awaits the real end of the movement.
 */
export async function cursorTo({ selector = null, x = null, y = null, duration = 650 } = {}) {
  const el = cursorEl();
  if (!el.classList.contains("is-visible")) await cursorShow();

  let destination = { x, y };
  if (selector) {
    const rect = rectOf(selector);
    destination = { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
  }
  if (destination.x === null || destination.y === null) {
    throw new Error("cursorTo needs a selector or an x/y pair");
  }

  const from = `translate3d(${cursorPosition.x}px, ${cursorPosition.y}px, 0)`;
  const to = `translate3d(${destination.x}px, ${destination.y}px, 0)`;

  await settled(
    el.animate([{ transform: from }, { transform: to }], {
      duration: Math.max(0, duration),
      easing: "cubic-bezier(0.33, 0, 0.15, 1)",
      fill: "both",
    })
  );

  el.style.transform = to;
  cursorPosition = destination;
  return destination;
}

/** The press: the cursor dips and a ring expands from it, like a real tap. */
export async function cursorPress() {
  const el = cursorEl();
  const ring = el.querySelector(".promo-cursor-ring");
  const duration = 320 * motionScale();

  const ripple = ring.animate(
    [
      { transform: "scale(0.2)", opacity: 0.55 },
      { transform: "scale(1)", opacity: 0 },
    ],
    { duration, easing: "cubic-bezier(0.22, 1, 0.36, 1)" }
  );
  const dip = el.animate([{ scale: "1" }, { scale: "0.86" }, { scale: "1" }], {
    duration: duration * 0.6,
    easing: "ease-out",
  });

  await Promise.all([settled(ripple), settled(dip)]);
  return { pressed: true };
}

// ---------------------------------------------------------------------------
// Teardown
// ---------------------------------------------------------------------------

/** Remove one annotation, or every annotation when called without an id. */
export function clear({ id = null } = {}) {
  const remove = (key, entry) => {
    if (entry.raf) cancelAnimationFrame(entry.raf);
    entry.el.getAnimations().forEach((animation) => animation.cancel());
    entry.el.remove();
    registry.delete(key);
  };

  if (id) {
    const entry = registry.get(id);
    if (entry) remove(id, entry);
    return { cleared: id };
  }

  Array.from(registry.entries()).forEach(([key, entry]) => remove(key, entry));
  return { cleared: "all" };
}
