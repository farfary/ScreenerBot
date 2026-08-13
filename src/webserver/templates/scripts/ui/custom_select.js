/**
 * Custom Select Component
 * Drop-in replacement for native <select> elements with enhanced styling and keyboard support.
 *
 * Features:
 * - Dark theme styling matching dashboard design
 * - Full keyboard navigation (arrows, enter, escape)
 * - Type-ahead search
 * - Click outside to close
 * - Form submission support via hidden input
 */

import { openMenu, closeMenu } from "../core/menu_manager.js";

const MAX_DROPDOWN_HEIGHT = 280;
const VIEWPORT_MARGIN = 8;
const CLOSE_ANIMATION_MS = 220;
let customSelectId = 0;

export class CustomSelect {
  /**
   * @param {Object} options Configuration options
   * @param {HTMLElement} options.container Container element to render into
   * @param {Array<{value: string, label: string, selected?: boolean, disabled?: boolean}>} options.options Select options
   * @param {string} [options.placeholder='Select...'] Placeholder text
   * @param {Function} [options.onChange] Callback when value changes
   * @param {string} [options.id] ID for the component
   * @param {string} [options.name] Name for the hidden input (form submission)
   * @param {boolean} [options.disabled=false] Whether the select is disabled
   * @param {string} [options.className] Additional CSS class for the wrapper
   */
  constructor(options = {}) {
    this.container = options.container;
    this.options = options.options || [];
    this.placeholder = options.placeholder || "Select...";
    this.onChange = options.onChange || (() => {});
    this.id = options.id || null;
    this.name = options.name || null;
    this.disabled = options.disabled || false;
    this.className = options.className || "";
    this.ariaLabel = options.ariaLabel || "";
    this.ariaLabelledBy = options.ariaLabelledBy || "";

    // State
    this.isOpen = false;
    this.focusedIndex = -1;
    this.selectedValue = null;
    this.searchString = "";
    this.searchTimeout = null;
    this._isClosing = false;
    this._openAnimationFrame = null;
    this._positionAnimationFrame = null;
    this._closeAnimationTimer = null;
    this._lastPositionSignature = "";
    this._labelListeners = [];

    // DOM elements
    this.el = null;
    this.triggerEl = null;
    this.valueEl = null;
    this.dropdownEl = null;
    this.optionsContainerEl = null;
    this.searchContainerEl = null;
    this.searchInputEl = null;
    this.noResultsEl = null;
    this.hiddenInput = null;

    // Bound handlers for cleanup
    this._handleTriggerClick = this._handleTriggerClick.bind(this);
    this._handleKeyDown = this._handleKeyDown.bind(this);
    this._handleOptionClick = this._handleOptionClick.bind(this);
    this._handleSearchInput = this._handleSearchInput.bind(this);

    // Stable descriptor for the global menu coordinator. `owns` must also cover
    // the portaled dropdown (rendered in document.body), so clicks/focus inside
    // the option list don't dismiss it.
    this._menuHandle = {
      close: (reason) =>
        this.close({
          restoreFocus: reason === "escape",
          immediate: [
            "superseded",
            "outside-pointer",
            "focus-left",
            "document-hidden",
            "navigation",
            "dialog-open",
          ].includes(reason),
        }),
      owns: (t) =>
        (this.el && this.el.contains(t)) || (this.dropdownEl && this.dropdownEl.contains(t)),
    };

    // Find initially selected option
    const selectedOpt = this.options.find((o) => o.selected);
    if (selectedOpt) {
      this.selectedValue = selectedOpt.value;
    }

    this._render();
    this._attachEvents();
  }

  /**
   * Enhance an existing native <select> element
   * @param {HTMLSelectElement} selectElement The native select to enhance
   * @param {Object} [extraOptions] Additional options to merge
   * @returns {CustomSelect} The created CustomSelect instance
   */
  static enhance(selectElement, extraOptions = {}) {
    if (!(selectElement instanceof HTMLSelectElement)) {
      console.warn("CustomSelect.enhance requires a <select> element");
      return null;
    }

    // Extract options from native select
    const options = Array.from(selectElement.options).map((opt) => ({
      value: opt.value,
      label: opt.textContent?.trim() || "",
      selected: opt.selected,
      disabled: opt.disabled,
    }));
    const associatedLabels = new Set(Array.from(selectElement.labels || []));
    const wrappingLabel = selectElement.closest("label");
    const fieldLabel = selectElement.parentElement?.querySelector(":scope > label");
    if (wrappingLabel) associatedLabels.add(wrappingLabel);
    if (fieldLabel) associatedLabels.add(fieldLabel);
    const sourceDisplay = selectElement.style.display;
    const sourceClassName = selectElement.className;
    const valueDescriptor = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value");
    const selectedIndexDescriptor = Object.getOwnPropertyDescriptor(
      HTMLSelectElement.prototype,
      "selectedIndex"
    );
    const disabledDescriptor = Object.getOwnPropertyDescriptor(
      HTMLSelectElement.prototype,
      "disabled"
    );
    const getLabelText = (label) => {
      const clone = label.cloneNode(true);
      clone.querySelectorAll("select, .custom-select-host").forEach((control) => control.remove());
      return clone.textContent?.trim() || "";
    };

    // Create wrapper container
    const container = document.createElement("div");
    container.className = "custom-select-host";
    selectElement.parentNode.insertBefore(container, selectElement);

    // Hide the original select. A bare inline `display:none` is NOT enough: page
    // logic and CSS frequently toggle `display` on a select (or its container) to
    // show/hide a field. Re-showing the enhanced source select makes it paint the
    // global `var(--select-arrow)` chevron again UNDER the wrapper's own .cs-arrow
    // -> the double/native-looking arrow. The `cs-enhanced-source` class hard-hides
    // it with `display:none !important` so it can never reappear as a second arrow
    // source. (Single-arrow-source rule — see commit 20626fd4.)
    selectElement.classList.add("cs-enhanced-source");
    selectElement.style.display = "none";

    // Create custom select
    const explicitLabelledBy = selectElement.getAttribute("aria-labelledby") || "";
    const labelledByElements = explicitLabelledBy
      .split(/\s+/)
      .filter(Boolean)
      .map((id) => document.getElementById(id))
      .filter(Boolean);
    const resolvedAriaLabel =
      selectElement.getAttribute("aria-label") ||
      labelledByElements.map((label) => label.textContent?.trim()).find(Boolean) ||
      Array.from(associatedLabels)
        .map(getLabelText)
        .find(Boolean) ||
      "";

    const customSelect = new CustomSelect({
      container,
      options,
      placeholder:
        selectElement.dataset.placeholder || selectElement.options[0]?.textContent || "Select...",
      id: selectElement.id ? `${selectElement.id}-custom` : null,
      // The native source remains the successful form control; giving the
      // helper input the same name would submit the value twice.
      name: null,
      disabled: selectElement.disabled,
      className: sourceClassName,
      ariaLabel: resolvedAriaLabel,
      ariaLabelledBy: explicitLabelledBy,
      ...extraOptions,
      onChange: (value) => {
        // Sync value back to original select for form compatibility
        valueDescriptor?.set.call(selectElement, value);
        selectElement.dispatchEvent(new Event("change", { bubbles: true }));
        extraOptions.onChange?.(value, selectElement);
      },
    });

    // Store reference to original select
    customSelect._originalSelect = selectElement;
    customSelect._sourceDisplay = sourceDisplay;
    customSelect._sourcePropertyNames = [];
    const syncSourceValue = () => {
      const index = selectedIndexDescriptor?.get.call(selectElement) ?? selectElement.selectedIndex;
      const value = index < 0 ? null : valueDescriptor.get.call(selectElement);
      customSelect.setValue(value, { emitChange: false });
    };

    // Property assignments do not produce DOM mutations. Bridge the two native
    // properties pages change after enhancement so the visible control cannot
    // lag behind its source select.
    if (valueDescriptor?.get && valueDescriptor?.set) {
      Object.defineProperty(selectElement, "value", {
        configurable: true,
        enumerable: valueDescriptor.enumerable,
        get: () => valueDescriptor.get.call(selectElement),
        set: (value) => {
          valueDescriptor.set.call(selectElement, value);
          syncSourceValue();
        },
      });
      customSelect._sourcePropertyNames.push("value");
    }
    if (selectedIndexDescriptor?.get && selectedIndexDescriptor?.set) {
      Object.defineProperty(selectElement, "selectedIndex", {
        configurable: true,
        enumerable: selectedIndexDescriptor.enumerable,
        get: () => selectedIndexDescriptor.get.call(selectElement),
        set: (index) => {
          selectedIndexDescriptor.set.call(selectElement, index);
          syncSourceValue();
        },
      });
      customSelect._sourcePropertyNames.push("selectedIndex");
    }
    if (disabledDescriptor?.get && disabledDescriptor?.set) {
      Object.defineProperty(selectElement, "disabled", {
        configurable: true,
        enumerable: disabledDescriptor.enumerable,
        get: () => disabledDescriptor.get.call(selectElement),
        set: (disabled) => {
          disabledDescriptor.set.call(selectElement, disabled);
          customSelect.setDisabled(disabledDescriptor.get.call(selectElement));
        },
      });
      customSelect._sourcePropertyNames.push("disabled");
    }

    // A native label still targets the hidden source select. Forward label
    // activation to the visible combobox so mouse and keyboard focus agree.
    associatedLabels.forEach((label) => {
      const handler = (event) => {
        if (customSelect.el?.contains(event.target)) return;
        event.preventDefault();
        customSelect.focus();
      };
      label.addEventListener("click", handler);
      customSelect._labelListeners.push({ label, handler });
    });

    const syncFromSource = () => {
      syncSourceValue();
      customSelect.setDisabled(disabledDescriptor.get.call(selectElement));
    };
    customSelect._sourceChangeHandler = syncFromSource;
    selectElement.addEventListener("change", syncFromSource);
    if (selectElement.form) {
      customSelect._sourceForm = selectElement.form;
      customSelect._sourceResetHandler = () => requestAnimationFrame(syncFromSource);
      customSelect._sourceForm.addEventListener("reset", customSelect._sourceResetHandler);
    }

    // Store reference to CustomSelect instance on original select for later access
    selectElement._customSelectInstance = customSelect;

    // Keep the custom UI in sync when the underlying <select>'s options are
    // populated or replaced AFTER enhancement. The global auto-enhancer upgrades a
    // select as soon as it enters the DOM, but many pages fill the option list
    // asynchronously (provider/category lists, etc.) — without this the custom
    // dropdown would be stuck showing the empty initial snapshot.
    if (typeof MutationObserver !== "undefined") {
      const optionObserver = new MutationObserver(() => {
        const opts = Array.from(selectElement.options).map((opt) => ({
          value: opt.value,
          label: opt.textContent?.trim() || "",
          selected: opt.selected,
          disabled: opt.disabled,
        }));
        customSelect.setOptions(opts);
        syncSourceValue();
        customSelect.setDisabled(selectElement.disabled);
      });
      optionObserver.observe(selectElement, {
        attributes: true,
        attributeFilter: ["disabled", "label", "selected", "value"],
        childList: true,
        subtree: true,
      });
      customSelect._optionObserver = optionObserver;
    }

    return customSelect;
  }

  _render() {
    const componentId = this.id || `custom-select-${++customSelectId}`;
    const listboxId = `${componentId}-listbox`;

    // Create wrapper
    this.el = document.createElement("div");
    this.el.className = `custom-select${this.className ? ` ${this.className}` : ""}`;
    this.el.tabIndex = this.disabled ? -1 : 0;
    this.el.setAttribute("role", "combobox");
    this.el.setAttribute("aria-haspopup", "listbox");
    this.el.setAttribute("aria-expanded", "false");
    this.el.setAttribute("aria-controls", listboxId);
    this.el.id = componentId;
    if (this.ariaLabelledBy) {
      this.el.setAttribute("aria-labelledby", this.ariaLabelledBy);
    } else if (this.ariaLabel) {
      this.el.setAttribute("aria-label", this.ariaLabel);
    }
    if (this.disabled) {
      this.el.classList.add("disabled");
      this.el.setAttribute("aria-disabled", "true");
    }

    // Create trigger
    this.triggerEl = document.createElement("div");
    this.triggerEl.className = "cs-trigger";

    // Create value display
    this.valueEl = document.createElement("span");
    this.valueEl.className = "cs-value";
    this._updateDisplayValue();

    // Create arrow with SVG chevron
    const arrowEl = document.createElement("span");
    arrowEl.className = "cs-arrow";
    arrowEl.innerHTML =
      '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>';

    this.triggerEl.appendChild(this.valueEl);
    this.triggerEl.appendChild(arrowEl);

    // Create dropdown (will be appended to body when opened - portal pattern)
    this.dropdownEl = document.createElement("div");
    this.dropdownEl.className = "cs-dropdown";
    this.dropdownEl.id = `${componentId}-popup`;

    // Create search container (only shown if many options)
    this.searchContainerEl = document.createElement("div");
    this.searchContainerEl.className = "cs-search-container";
    this.searchInputEl = document.createElement("input");
    this.searchInputEl.type = "text";
    this.searchInputEl.className = "cs-search-input";
    this.searchInputEl.placeholder = "Search...";
    this.searchInputEl.autocomplete = "off";
    this.searchInputEl.setAttribute("aria-label", "Filter options");
    this.searchInputEl.setAttribute("aria-controls", listboxId);
    this.searchContainerEl.appendChild(this.searchInputEl);

    // Create options container
    this.optionsContainerEl = document.createElement("div");
    this.optionsContainerEl.className = "cs-options-container";
    this.optionsContainerEl.id = listboxId;
    this.optionsContainerEl.setAttribute("role", "listbox");

    // Create no results message
    this.noResultsEl = document.createElement("div");
    this.noResultsEl.className = "cs-no-results";
    this.noResultsEl.textContent = "No results found";
    this.noResultsEl.style.display = "none";

    this.dropdownEl.appendChild(this.searchContainerEl);
    this.dropdownEl.appendChild(this.optionsContainerEl);
    this.dropdownEl.appendChild(this.noResultsEl);

    this._renderOptions();

    // Create hidden input for form submission
    this.hiddenInput = document.createElement("input");
    this.hiddenInput.type = "hidden";
    if (this.name) {
      this.hiddenInput.name = this.name;
    }
    this.hiddenInput.value = this.selectedValue || "";

    // Assemble (dropdown is NOT appended here - it uses portal pattern)
    this.el.appendChild(this.triggerEl);
    this.el.appendChild(this.hiddenInput);

    // Mount to container
    if (this.container) {
      this.container.appendChild(this.el);
    }

    // Update data attribute
    if (this.selectedValue) {
      this.el.dataset.value = this.selectedValue;
    }
  }

  _renderOptions() {
    this.optionsContainerEl.innerHTML = "";

    // Show/hide search based on option count
    if (this.options.length > 10) {
      this.searchContainerEl.style.display = "block";
    } else {
      this.searchContainerEl.style.display = "none";
    }

    this.options.forEach((opt, index) => {
      const optionEl = document.createElement("div");
      optionEl.className = "cs-option";
      optionEl.id = `${this.optionsContainerEl.id}-option-${index}`;
      optionEl.dataset.value = opt.value;
      optionEl.dataset.index = index;
      optionEl.textContent = opt.label;
      optionEl.title = opt.label; // full text on hover when a long label is ellipsized
      optionEl.setAttribute("role", "option");
      optionEl.setAttribute("aria-selected", "false");

      if (opt.value === this.selectedValue) {
        optionEl.classList.add("selected");
        optionEl.setAttribute("aria-selected", "true");
      }

      if (opt.disabled) {
        optionEl.classList.add("disabled");
        optionEl.setAttribute("aria-disabled", "true");
      }

      this.optionsContainerEl.appendChild(optionEl);
    });
  }

  _updateDisplayValue() {
    const selectedOpt = this.options.find((o) => o.value === this.selectedValue);
    if (selectedOpt) {
      this.valueEl.textContent = selectedOpt.label;
      this.valueEl.title = selectedOpt.label;
      this.valueEl.classList.remove("placeholder");
    } else {
      this.valueEl.textContent = this.placeholder;
      this.valueEl.removeAttribute("title");
      this.valueEl.classList.add("placeholder");
    }
  }

  _attachEvents() {
    this.triggerEl.addEventListener("click", this._handleTriggerClick);
    this.el.addEventListener("keydown", this._handleKeyDown);
    this.dropdownEl.addEventListener("keydown", this._handleKeyDown);
    this.optionsContainerEl.addEventListener("click", this._handleOptionClick);
    this.searchInputEl.addEventListener("input", this._handleSearchInput);
  }

  _detachEvents() {
    this.triggerEl.removeEventListener("click", this._handleTriggerClick);
    this.el.removeEventListener("keydown", this._handleKeyDown);
    this.dropdownEl.removeEventListener("keydown", this._handleKeyDown);
    this.optionsContainerEl.removeEventListener("click", this._handleOptionClick);
    this.searchInputEl.removeEventListener("input", this._handleSearchInput);
  }

  _handleSearchInput(e) {
    const query = e.target.value.toLowerCase();
    let hasResults = false;
    let firstVisibleIndex = -1;

    this.options.forEach((opt, index) => {
      const optionEl = this.optionsContainerEl.querySelector(`.cs-option[data-index="${index}"]`);
      if (!optionEl) return;

      const matches = opt.label.toLowerCase().includes(query);
      optionEl.style.display = matches ? "flex" : "none";

      if (matches) {
        hasResults = true;
        if (firstVisibleIndex === -1) firstVisibleIndex = index;
      }
    });

    this.noResultsEl.style.display = hasResults ? "none" : "block";

    if (hasResults && firstVisibleIndex !== -1) {
      this._setFocusIndex(firstVisibleIndex);
    } else {
      this._setFocusIndex(-1);
    }
  }

  _handleTriggerClick(e) {
    e.stopPropagation();
    if (this.disabled) return;
    this.el.focus({ preventScroll: true });
    this.toggle();
  }

  _handleOptionClick(e) {
    const optionEl = e.target.closest(".cs-option");
    if (!optionEl || optionEl.classList.contains("disabled")) return;

    const value = optionEl.dataset.value;
    this._selectValue(value);
    this.close({ restoreFocus: true });
  }

  _handleKeyDown(e) {
    if (this.disabled) return;

    // Printable keys, Space, Home and End belong to the search field itself.
    // Navigation keys still control the portaled listbox.
    if (
      e.target === this.searchInputEl &&
      !["Enter", "Escape", "ArrowDown", "ArrowUp", "Tab"].includes(e.key)
    ) {
      return;
    }

    switch (e.key) {
      case "Enter":
      case " ":
        e.preventDefault();
        if (this.isOpen) {
          if (this.focusedIndex >= 0) {
            const opt = this.options[this.focusedIndex];
            if (opt && !opt.disabled) {
              this._selectValue(opt.value);
            }
          }
          this.close({ restoreFocus: true });
        } else {
          this.open();
        }
        break;

      case "Escape":
        if (this.isOpen) {
          e.preventDefault();
          this.close({ restoreFocus: true });
        }
        break;

      case "ArrowDown":
        e.preventDefault();
        if (!this.isOpen) {
          this.open();
        } else {
          this._moveFocus(1);
        }
        break;

      case "ArrowUp":
        e.preventDefault();
        if (!this.isOpen) {
          this.open();
        } else {
          this._moveFocus(-1);
        }
        break;

      case "Home":
        if (this.isOpen) {
          e.preventDefault();
          this._setFocusIndex(this._findNextEnabledIndex(-1, 1));
        }
        break;

      case "End":
        if (this.isOpen) {
          e.preventDefault();
          this._setFocusIndex(this._findNextEnabledIndex(this.options.length, -1));
        }
        break;

      case "Tab":
        // Allow tab to close and move focus naturally
        if (this.isOpen) {
          this.close();
        }
        break;

      default:
        // Type-ahead search
        if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
          this._handleTypeAhead(e.key);
        }
        break;
    }
  }

  _handleTypeAhead(char) {
    // Clear previous timeout
    if (this.searchTimeout) {
      clearTimeout(this.searchTimeout);
    }

    // Append to search string
    this.searchString += char.toLowerCase();

    // Find matching option
    const matchIndex = this.options.findIndex(
      (opt) => !opt.disabled && opt.label.toLowerCase().startsWith(this.searchString)
    );

    if (matchIndex >= 0) {
      if (this.isOpen) {
        this._setFocusIndex(matchIndex);
      } else {
        this._selectValue(this.options[matchIndex].value);
      }
    }

    // Clear search string after delay
    this.searchTimeout = setTimeout(() => {
      this.searchString = "";
    }, 500);
  }

  _moveFocus(direction) {
    const nextIndex = this._findNextEnabledIndex(this.focusedIndex, direction);
    if (nextIndex >= 0) {
      this._setFocusIndex(nextIndex);
    }
  }

  _findNextEnabledIndex(startIndex, direction) {
    let index = startIndex + direction;
    while (index >= 0 && index < this.options.length) {
      const opt = this.options[index];
      const optionEl = this.optionsContainerEl.querySelector(`.cs-option[data-index="${index}"]`);
      const isVisible = optionEl && optionEl.style.display !== "none";

      if (!opt.disabled && isVisible) {
        return index;
      }
      index += direction;
    }
    return -1;
  }

  _setFocusIndex(index) {
    // Remove previous focus
    const prevFocused = this.optionsContainerEl.querySelector(".cs-option.focused");
    if (prevFocused) {
      prevFocused.classList.remove("focused");
    }

    this.focusedIndex = index;
    this.el.removeAttribute("aria-activedescendant");
    this.searchInputEl.removeAttribute("aria-activedescendant");

    if (index >= 0 && index < this.options.length) {
      const optionEl = this.optionsContainerEl.querySelector(`.cs-option[data-index="${index}"]`);
      if (optionEl) {
        optionEl.classList.add("focused");
        this.el.setAttribute("aria-activedescendant", optionEl.id);
        this.searchInputEl.setAttribute("aria-activedescendant", optionEl.id);
        // Scroll into view
        optionEl.scrollIntoView({ block: "nearest" });
      }
    }
  }

  _selectValue(value, { emitChange = true } = {}) {
    const prevValue = this.selectedValue;
    this.selectedValue = value;

    // Update hidden input
    this.hiddenInput.value = value || "";

    // Update data attribute
    this.el.dataset.value = value || "";

    // Update display
    this._updateDisplayValue();

    // Update option states
    this.optionsContainerEl.querySelectorAll(".cs-option").forEach((optEl) => {
      const isSelected = optEl.dataset.value === value;
      optEl.classList.toggle("selected", isSelected);
      optEl.setAttribute("aria-selected", isSelected ? "true" : "false");
    });

    // Fire change callback if value actually changed
    if (emitChange && value !== prevValue) {
      this.onChange(value);
    }
  }

  open() {
    if (this.disabled || this.isOpen) return;

    this._cancelCloseAnimation();
    this._isClosing = false;
    this.el.classList.remove("closing");

    // Register first so any other open menu is dismissed before this one shows.
    openMenu(this._menuHandle);
    this.isOpen = true;
    this.el.classList.add("open");
    this.el.setAttribute("aria-expanded", "true");

    // Portal pattern: append dropdown to body
    this._appendDropdownToBody();

    // Set initial focus to selected option or first enabled option
    const selectedIndex = this.options.findIndex((o) => o.value === this.selectedValue);
    if (selectedIndex >= 0) {
      this._setFocusIndex(selectedIndex);
    } else {
      this._setFocusIndex(this._findNextEnabledIndex(-1, 1));
    }

    // Position dropdown with fixed positioning
    this._positionDropdown();
    this._startPositionTracking();

    // The menu is portaled, so descendant `.open` selectors cannot animate it.
    // Start from its own hidden portal state and promote it on the next frame.
    this.dropdownEl.classList.remove("is-open");
    this._openAnimationFrame = requestAnimationFrame(() => {
      this._openAnimationFrame = null;
      if (this.isOpen) this.dropdownEl.classList.add("is-open");
    });

    // Focus search input if visible
    if (this.options.length > 10) {
      this.searchInputEl.focus({ preventScroll: true });
    }
  }

  close({ restoreFocus = false, immediate = false } = {}) {
    if (!this.isOpen && !this._isClosing) return;

    closeMenu(this._menuHandle);
    this.isOpen = false;
    this._isClosing = !immediate;
    this.el.classList.remove("open");
    this.el.classList.toggle("closing", this._isClosing);
    this.el.setAttribute("aria-expanded", "false");
    this.el.removeAttribute("aria-activedescendant");
    this.searchInputEl.removeAttribute("aria-activedescendant");
    this.focusedIndex = -1;

    // Remove focus styling
    const focused = this.dropdownEl.querySelector(".cs-option.focused");
    if (focused) {
      focused.classList.remove("focused");
    }

    // Clear search
    this.searchInputEl.value = "";
    this.optionsContainerEl.querySelectorAll(".cs-option").forEach((option) => {
      option.style.display = "";
    });
    this.noResultsEl.style.display = "none";
    this._setFocusIndex(-1);
    this.searchString = "";
    if (this.searchTimeout) {
      clearTimeout(this.searchTimeout);
      this.searchTimeout = null;
    }

    if (restoreFocus && this.el.isConnected) {
      this.el.focus({ preventScroll: true });
    }

    if (this._openAnimationFrame !== null) {
      cancelAnimationFrame(this._openAnimationFrame);
      this._openAnimationFrame = null;
    }

    this.dropdownEl.classList.remove("is-open");
    if (immediate || window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) {
      this._finishClose();
      return;
    }

    this._cancelCloseAnimation();
    this._closeAnimationTimer = setTimeout(() => this._finishClose(), CLOSE_ANIMATION_MS);
  }

  toggle() {
    if (this.isOpen) {
      this.close();
    } else {
      this.open();
    }
  }

  _positionDropdown(triggerRect = this.triggerEl?.getBoundingClientRect()) {
    if (!triggerRect || !this.dropdownEl?.isConnected) return;

    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    const style = this.dropdownEl.style;
    style.position = "fixed";
    style.right = "auto"; // JS owns horizontal placement; clear the base `right: 0`

    // The menu is the lower/upper half of the control, so it must keep the
    // trigger's exact width instead of presenting as an independent floating card.
    const dropdownWidth = Math.max(
      0,
      Math.min(triggerRect.width, viewportWidth - VIEWPORT_MARGIN * 2)
    );
    style.minWidth = `${dropdownWidth}px`;
    style.maxWidth = `${dropdownWidth}px`;
    style.width = `${dropdownWidth}px`;
    style.setProperty("--cs-control-height", `${triggerRect.height}px`);

    // Measure the resolved size after the width constraints are applied.
    const dropdownRect = this.dropdownEl.getBoundingClientRect();
    const resolvedDropdownWidth = dropdownRect.width;
    const dropdownHeight = Math.min(
      this.dropdownEl.scrollHeight || MAX_DROPDOWN_HEIGHT,
      MAX_DROPDOWN_HEIGHT
    );

    // --- HORIZONTAL: align to the trigger, then clamp fully into the viewport. ---
    let left = triggerRect.left;
    if (left + resolvedDropdownWidth > viewportWidth - VIEWPORT_MARGIN) {
      // Prefer right-aligning to the trigger when it would overflow the right edge.
      left = triggerRect.right - resolvedDropdownWidth;
    }
    left = Math.max(
      VIEWPORT_MARGIN,
      Math.min(left, viewportWidth - resolvedDropdownWidth - VIEWPORT_MARGIN)
    );
    style.left = `${left}px`;

    // --- VERTICAL: open below by default, flip above when there is more room. ---
    const spaceBelow = Math.max(0, viewportHeight - triggerRect.bottom - VIEWPORT_MARGIN);
    const spaceAbove = Math.max(0, triggerRect.top - VIEWPORT_MARGIN);
    const openAbove = spaceBelow < dropdownHeight && spaceAbove > spaceBelow;
    const availableHeight = openAbove ? spaceAbove : spaceBelow;
    style.maxHeight = `${Math.max(0, Math.min(MAX_DROPDOWN_HEIGHT, availableHeight))}px`;

    if (openAbove) {
      style.top = "auto";
      style.bottom = `${viewportHeight - triggerRect.top - 1}px`;
      this.el.classList.add("dropdown-above");
      this.dropdownEl.classList.add("cs-dropdown--above");
    } else {
      style.top = `${triggerRect.bottom - 1}px`;
      style.bottom = "auto";
      this.el.classList.remove("dropdown-above");
      this.dropdownEl.classList.remove("cs-dropdown--above");
    }
  }

  _appendDropdownToBody() {
    if (!this.dropdownEl.parentNode) {
      this.dropdownEl.classList.add("cs-portal");
      document.body.appendChild(this.dropdownEl);
    }
  }

  _removeDropdownFromBody() {
    if (this.dropdownEl && this.dropdownEl.parentNode === document.body) {
      this.dropdownEl.classList.remove("is-open", "cs-dropdown--above");
      this.dropdownEl.classList.remove("cs-portal");
      document.body.removeChild(this.dropdownEl);
    }
  }

  _startPositionTracking() {
    if (this._positionAnimationFrame !== null) return;

    const update = () => {
      this._positionAnimationFrame = null;
      if (!this.isOpen && !this._isClosing) return;
      if (!this.triggerEl?.isConnected || !this.dropdownEl?.isConnected) {
        this.close({ immediate: true });
        return;
      }

      const rect = this.triggerEl.getBoundingClientRect();
      const signature = [
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        rect.width,
        rect.height,
        window.innerWidth,
        window.innerHeight,
        this.dropdownEl.scrollHeight,
      ].join(":");
      if (signature !== this._lastPositionSignature) {
        this._lastPositionSignature = signature;
        this._positionDropdown(rect);
      }
      this._positionAnimationFrame = requestAnimationFrame(update);
    };

    this._lastPositionSignature = "";
    this._positionAnimationFrame = requestAnimationFrame(update);
  }

  _stopPositionTracking() {
    if (this._positionAnimationFrame !== null) {
      cancelAnimationFrame(this._positionAnimationFrame);
      this._positionAnimationFrame = null;
    }
    this._lastPositionSignature = "";
  }

  _cancelCloseAnimation() {
    if (this._closeAnimationTimer !== null) {
      clearTimeout(this._closeAnimationTimer);
      this._closeAnimationTimer = null;
    }
  }

  _finishClose() {
    this._cancelCloseAnimation();
    this._isClosing = false;
    this.el?.classList.remove("closing", "dropdown-above");
    this._stopPositionTracking();
    this._removeDropdownFromBody();
  }

  // Public API

  /**
   * Get the current selected value
   * @returns {string|null} The selected value
   */
  getValue() {
    return this.selectedValue;
  }

  /**
   * Set the selected value
   * @param {string} value The value to select
   */
  setValue(value, { emitChange = true } = {}) {
    const opt = this.options.find((o) => o.value === value);
    if (opt) {
      this._selectValue(value, { emitChange });
      return;
    }

    const previousValue = this.selectedValue;
    this.selectedValue = null;
    this.hiddenInput.value = "";
    delete this.el.dataset.value;
    this._updateDisplayValue();
    this.optionsContainerEl.querySelectorAll(".cs-option").forEach((optionEl) => {
      optionEl.classList.remove("selected");
      optionEl.setAttribute("aria-selected", "false");
    });
    if (emitChange && previousValue !== null) this.onChange("");
  }

  /**
   * Update the available options
   * @param {Array<{value: string, label: string, selected?: boolean, disabled?: boolean}>} newOptions New options array
   */
  setOptions(newOptions) {
    this.options = newOptions;

    // A selected option from the source select is authoritative even when the
    // previous value still exists (async option refreshes commonly change it).
    const selectedOption = this.options.find((o) => o.selected);
    const stillValid = this.options.find((o) => o.value === this.selectedValue);
    if (selectedOption || !stillValid) {
      this.selectedValue = selectedOption ? selectedOption.value : null;
      this.hiddenInput.value = this.selectedValue || "";
      if (this.selectedValue === null) delete this.el.dataset.value;
      else this.el.dataset.value = this.selectedValue;
    }

    this._renderOptions();
    this._updateDisplayValue();
    if (this.isOpen) {
      this._lastPositionSignature = "";
    }
  }

  /**
   * Enable the select
   */
  enable() {
    this.disabled = false;
    this.el.classList.remove("disabled");
    this.el.tabIndex = 0;
    this.el.removeAttribute("aria-disabled");
  }

  /**
   * Disable the select
   */
  disable() {
    this.disabled = true;
    this.el.classList.add("disabled");
    this.el.tabIndex = -1;
    this.el.setAttribute("aria-disabled", "true");
    this.close();
  }

  setDisabled(disabled) {
    if (disabled) this.disable();
    else this.enable();
  }

  /**
   * Focus the select element
   */
  focus() {
    this.el.focus();
  }

  /**
   * Destroy the component and clean up
   */
  destroy() {
    // Close first and bypass the exit animation because the owner is leaving.
    this.close({ immediate: true });
    this._finishClose();
    this._detachEvents();

    // Stop syncing from the original <select> (enhanced instances only).
    if (this._optionObserver) {
      this._optionObserver.disconnect();
      this._optionObserver = null;
    }

    this._labelListeners.forEach(({ label, handler }) => label.removeEventListener("click", handler));
    this._labelListeners = [];

    // Ensure dropdown is removed from body if still attached
    this._removeDropdownFromBody();

    // Restore original select if enhanced
    if (this._originalSelect) {
      this._originalSelect.removeEventListener("change", this._sourceChangeHandler);
      this._sourceForm?.removeEventListener("reset", this._sourceResetHandler);
      this._sourcePropertyNames.forEach((property) => delete this._originalSelect[property]);
      this._originalSelect.classList.remove("cs-enhanced-source");
      this._originalSelect.style.display = this._sourceDisplay || "";
      this._originalSelect.removeAttribute("data-enhanced");
      delete this._originalSelect._customSelectInstance;
    }

    // Remove from DOM
    if (this.el && this.el.parentNode) {
      this.el.parentNode.removeChild(this.el);
    }
    if (this.container?.classList.contains("custom-select-host")) {
      this.container.remove();
    }

    // Clear references
    this.el = null;
    this.triggerEl = null;
    this.valueEl = null;
    this.dropdownEl = null;
    this.hiddenInput = null;
    this.container = null;
    this.options = null;
    this.onChange = null;
    this._originalSelect = null;
    this._sourcePropertyNames = null;
    this._sourceDisplay = null;
    this._sourceChangeHandler = null;
    this._sourceResetHandler = null;
    this._sourceForm = null;
  }
}

/**
 * Enhance all native <select> elements within a container
 * @param {HTMLElement} [container=document] Container to search within
 * @param {string} [selector='select[data-custom-select]'] Selector for selects to enhance
 * @returns {CustomSelect[]} Array of created CustomSelect instances
 */
export function enhanceAllSelects(container = document, selector = "select[data-custom-select]") {
  const selects = container.querySelectorAll(selector);
  const instances = [];

  selects.forEach((select) => {
    // Skip if already enhanced
    if (select.dataset.enhanced === "true") return;

    const instance = CustomSelect.enhance(select);
    if (instance) {
      select.dataset.enhanced = "true";
      instances.push(instance);
    }
  });

  return instances;
}

let _globalEnhancerInstalled = false;

/**
 * Install a one-time, document-wide auto-enhancer so EVERY
 * `select[data-custom-select]` becomes a CustomSelect automatically — including
 * selects rendered later by dynamically-built pages, dialogs and repeating rows
 * (e.g. the auto-trader time rules). Previously `data-custom-select` was a no-op
 * unless a page happened to call enhanceAllSelects(), so most pages still showed
 * the native browser dropdown. With this installed, no page needs to call it.
 * Idempotent: a MutationObserver enhances only newly-added selects, and the
 * per-select `data-enhanced` guard prevents double-enhancement.
 */
export function installGlobalSelectEnhancer() {
  if (_globalEnhancerInstalled || typeof document === "undefined") return;
  _globalEnhancerInstalled = true;

  const enhanceWithin = (root) => {
    if (!root || root.nodeType !== 1) return;
    if (
      root.matches &&
      root.matches("select[data-custom-select]") &&
      root.dataset.enhanced !== "true"
    ) {
      if (CustomSelect.enhance(root)) root.dataset.enhanced = "true";
    }
    if (root.querySelectorAll) {
      root.querySelectorAll("select[data-custom-select]:not([data-enhanced])").forEach((s) => {
        if (CustomSelect.enhance(s)) s.dataset.enhanced = "true";
      });
    }
  };

  const destroyWithin = (root) => {
    if (!root || root.nodeType !== 1) return;
    const destroySelect = (select) => select._customSelectInstance?.destroy();
    if (root.matches?.("select[data-enhanced='true']")) destroySelect(root);
    root.querySelectorAll?.("select[data-enhanced='true']").forEach(destroySelect);
  };

  const initialSweep = () => enhanceWithin(document.body || document.documentElement);
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initialSweep, { once: true });
  } else {
    initialSweep();
  }

  // Enhance any select added anywhere later. Scoped to the added subtrees so it
  // stays cheap even while live tables re-render constantly.
  const observer = new MutationObserver((mutations) => {
    for (const m of mutations) {
      m.removedNodes?.forEach(destroyWithin);
      m.addedNodes?.forEach(enhanceWithin);
    }
  });
  observer.observe(document.documentElement, { childList: true, subtree: true });
}
