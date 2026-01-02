/**
 * Dropdown Menu Component
 * Reusable dropdown for power menu, trader controls, etc.
 */

export class Dropdown {
  constructor(options = {}) {
    this.trigger = options.trigger; // Button element that triggers dropdown
    this.items = options.items || []; // Array of menu items
    this.onSelect = options.onSelect || (() => {}); // Callback when item selected
    this.align = options.align || "right"; // 'left' or 'right'
    this.isOpen = false;
    this.dropdownEl = null;

    // Store bound handlers for cleanup
    this._triggerListener = null;
    this._documentClickListener = null;
    this._documentKeydownListener = null;
    this._itemListeners = [];

    this._init();
  }

  _init() {
    if (!this.trigger) return;

    // Add aria attributes
    this.trigger.setAttribute("aria-haspopup", "true");
    this.trigger.setAttribute("aria-expanded", "false");

    // Create dropdown menu
    this._createDropdown();

    // Attach event listeners with stored references
    this._triggerListener = (e) => {
      e.stopPropagation();
      this.toggle();
    };
    this.trigger.addEventListener("click", this._triggerListener);

    // Close on outside click
    this._documentClickListener = (e) => {
      if (this.isOpen && !this.dropdownEl.contains(e.target)) {
        this.close();
      }
    };
    document.addEventListener("click", this._documentClickListener);

    // Close on escape and handle arrows
    this._documentKeydownListener = (e) => {
      if (!this.isOpen) return;

      if (e.key === "Escape") {
        this.close();
        this.trigger.focus();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        this._moveFocus(1);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        this._moveFocus(-1);
      } else if (e.key === "Enter") {
        const focused = this.dropdownEl.querySelector(".dropdown-item:focus");
        if (focused) {
          focused.click();
        }
      }
    };
    document.addEventListener("keydown", this._documentKeydownListener);
  }

  _moveFocus(direction) {
    const items = Array.from(this.dropdownEl.querySelectorAll(".dropdown-item:not(.disabled)"));
    const current = document.activeElement;
    let index = items.indexOf(current);

    if (index === -1) {
      index = direction > 0 ? 0 : items.length - 1;
    } else {
      index = (index + direction + items.length) % items.length;
    }

    if (items[index]) {
      items[index].focus();
    }
  }

  _createDropdown() {
    this.dropdownEl = document.createElement("div");
    this.dropdownEl.className = "dropdown-menu";
    this.dropdownEl.setAttribute("data-align", this.align);

    this.items.forEach((item) => {
      if (item.divider) {
        const divider = document.createElement("div");
        divider.className = "dropdown-divider";
        this.dropdownEl.appendChild(divider);
        return;
      }

      const itemEl = document.createElement("button");
      itemEl.className = "dropdown-item";
      itemEl.type = "button";

      if (item.icon) {
        const icon = document.createElement("span");
        icon.className = "icon";
        icon.textContent = item.icon;
        itemEl.appendChild(icon);
      }

      const text = document.createElement("span");
      text.className = "label";
      text.textContent = item.label;
      itemEl.appendChild(text);

      if (item.badge) {
        const badge = document.createElement("span");
        badge.className = "badge";
        if (item.badgeVariant) {
          badge.setAttribute("data-variant", item.badgeVariant);
        }
        badge.textContent = item.badge;
        itemEl.appendChild(badge);
      }

      if (item.disabled) {
        itemEl.classList.add("disabled");
        itemEl.disabled = true;
      }

      if (item.danger) {
        itemEl.classList.add("danger");
      }

      const itemHandler = (e) => {
        e.stopPropagation();
        if (!item.disabled) {
          this.onSelect(item.id, item);
          if (!item.keepOpen) {
            this.close();
          }
        }
      };

      itemEl.addEventListener("click", itemHandler);
      this._itemListeners.push({ element: itemEl, handler: itemHandler });

      this.dropdownEl.appendChild(itemEl);
    });

    // Position dropdown relative to trigger
    const wrapper = document.createElement("div");
    wrapper.style.position = "relative";
    wrapper.style.display = "inline-block";
    this.trigger.parentNode.insertBefore(wrapper, this.trigger);
    wrapper.appendChild(this.trigger);
    wrapper.appendChild(this.dropdownEl);
  }

  toggle() {
    if (this.isOpen) {
      this.close();
    } else {
      this.open();
    }
  }

  open() {
    this.isOpen = true;
    this.dropdownEl.classList.add("open");
    this.trigger.setAttribute("aria-expanded", "true");
    this.trigger.classList.add("active");

    // Focus first item after animation
    setTimeout(() => {
      const firstItem = this.dropdownEl.querySelector(".dropdown-item:not(.disabled)");
      if (firstItem) firstItem.focus();
    }, 50);
  }

  close() {
    this.isOpen = false;
    this.dropdownEl.classList.remove("open");
    this.trigger.setAttribute("aria-expanded", "false");
    this.trigger.classList.remove("active");
  }

  destroy() {
    // Remove document-level listeners
    if (this._documentClickListener) {
      document.removeEventListener("click", this._documentClickListener);
      this._documentClickListener = null;
    }

    if (this._documentKeydownListener) {
      document.removeEventListener("keydown", this._documentKeydownListener);
      this._documentKeydownListener = null;
    }

    // Remove trigger listener
    if (this._triggerListener && this.trigger) {
      this.trigger.removeEventListener("click", this._triggerListener);
      this._triggerListener = null;
    }

    // Remove all item listeners
    this._itemListeners.forEach(({ element, handler }) => {
      if (element) {
        element.removeEventListener("click", handler);
      }
    });
    this._itemListeners = [];

    // Remove DOM
    if (this.dropdownEl) {
      this.dropdownEl.remove();
      this.dropdownEl = null;
    }

    // Clear references
    this.trigger = null;
    this.items = null;
    this.onSelect = null;
    this.isOpen = false;
  }
}
