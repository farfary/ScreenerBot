// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// DOM utilities

// Select single element by ID or selector
export function $(selector) {
  if (!selector) return null;
  if (selector.startsWith("#")) {
    return document.getElementById(selector.slice(1));
  }
  return document.querySelector(selector);
}

// Sign of the inline direction: 1 in LTR, -1 in RTL
export function dirSign() {
  return document.documentElement.dir === "rtl" ? -1 : 1;
}

// Distance scrolled from the inline start edge (Chromium reports negative scrollLeft in RTL)
export function scrollStart(el) {
  return Math.abs(el.scrollLeft);
}

// Select all elements by selector
export function $$(selector) {
  if (!selector) return [];
  return Array.from(document.querySelectorAll(selector));
}

// Get element by ID (shorthand)
export function el(id) {
  return document.getElementById(id);
}

// Add event listener
export function on(element, event, handler, options) {
  if (!element || !event || typeof handler !== "function") return;
  element.addEventListener(event, handler, options);
}

// Remove event listener
export function off(element, event, handler, options) {
  if (!element || !event || typeof handler !== "function") return;
  element.removeEventListener(event, handler, options);
}

// Toggle classes based on map (e.g., cls(el, {active: true, hidden: false}))
export function cls(element, classMap) {
  if (!element || !classMap) return;
  Object.entries(classMap).forEach(([className, shouldAdd]) => {
    if (shouldAdd) {
      element.classList.add(className);
    } else {
      element.classList.remove(className);
    }
  });
}

// Create element with attributes and content
export function create(tag, attributes = {}, content = "") {
  const el = document.createElement(tag);
  Object.entries(attributes).forEach(([key, value]) => {
    if (key === "class" || key === "className") {
      el.className = value;
    } else if (key.startsWith("data-")) {
      el.setAttribute(key, value);
    } else {
      el[key] = value;
    }
  });
  if (content) {
    if (typeof content === "string") {
      el.innerHTML = content;
    } else if (content instanceof Node) {
      el.appendChild(content);
    }
  }
  return el;
}

// Replace a button's content with an icon and a text label. The label is set as
// a text node, so it never passes through markup.
export function setIconLabel(button, iconClass, label) {
  const icon = document.createElement("i");
  icon.className = iconClass;
  button.replaceChildren(icon, " ", label);
}

const LOGO_FALLBACK_ATTR = "data-logo-fallback";

/**
 * Attribute for a logo `<img>` that should give way to a letter placeholder when
 * the image fails to load. `placeholderClass` must be a static class name, never an
 * external value. The placeholder shows the first character of the image's `alt`.
 */
export function logoFallbackAttr(placeholderClass) {
  return `${LOGO_FALLBACK_ATTR}="${placeholderClass}"`;
}

// `error` does not bubble, so the listener runs in the capture phase on the document.
if (typeof document !== "undefined" && !window.__logoFallbackInstalled) {
  window.__logoFallbackInstalled = true;
  document.addEventListener(
    "error",
    (event) => {
      const img = event.target;
      if (img?.tagName !== "IMG" || !img.hasAttribute(LOGO_FALLBACK_ATTR)) return;
      const parent = img.parentElement;
      if (!parent) return;
      const placeholder = document.createElement("div");
      placeholder.className = img.getAttribute(LOGO_FALLBACK_ATTR);
      placeholder.textContent = img.alt.charAt(0);
      parent.replaceChildren(placeholder);
    },
    true
  );
}

// Show/hide element
export function show(element) {
  if (!element) return;
  element.style.display = "";
}

export function hide(element) {
  if (!element) return;
  element.style.display = "none";
}

export function isVisible(element) {
  if (!element) return false;
  return element.style.display !== "none" && element.offsetParent !== null;
}
