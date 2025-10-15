// DOM utilities

// Select single element by ID or selector
export function $(selector) {
  if (!selector) return null;
  if (selector.startsWith("#")) {
    return document.getElementById(selector.slice(1));
  }
  return document.querySelector(selector);
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
