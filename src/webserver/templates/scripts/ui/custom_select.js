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

    // State
    this.isOpen = false;
    this.focusedIndex = -1;
    this.selectedValue = null;
    this.searchString = "";
    this.searchTimeout = null;

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
    this._handleDocumentClick = this._handleDocumentClick.bind(this);
    this._handleOptionClick = this._handleOptionClick.bind(this);
    this._handleScrollResize = this._handleScrollResize.bind(this);
    this._handleSearchInput = this._handleSearchInput.bind(this);

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
    // eslint-disable-next-line no-undef
    if (!(selectElement instanceof HTMLSelectElement)) {
      console.warn("CustomSelect.enhance requires a <select> element");
      return null;
    }

    // Extract options from native select
    const options = Array.from(selectElement.options).map((opt) => ({
      value: opt.value,
      label: opt.textContent,
      selected: opt.selected,
      disabled: opt.disabled,
    }));

    // Create wrapper container
    const container = document.createElement("div");
    selectElement.parentNode.insertBefore(container, selectElement);

    // Hide the original select
    selectElement.style.display = "none";

    // Create custom select
    const customSelect = new CustomSelect({
      container,
      options,
      placeholder:
        selectElement.dataset.placeholder || selectElement.options[0]?.textContent || "Select...",
      id: selectElement.id ? `${selectElement.id}-custom` : null,
      name: selectElement.name,
      disabled: selectElement.disabled,
      className: selectElement.className,
      onChange: (value) => {
        // Sync value back to original select for form compatibility
        selectElement.value = value;
        selectElement.dispatchEvent(new Event("change", { bubbles: true }));
      },
      ...extraOptions,
    });

    // Store reference to original select
    customSelect._originalSelect = selectElement;

    return customSelect;
  }

  _render() {
    // Create wrapper
    this.el = document.createElement("div");
    this.el.className = `custom-select${this.className ? ` ${this.className}` : ""}`;
    this.el.tabIndex = this.disabled ? -1 : 0;
    this.el.setAttribute("role", "combobox");
    this.el.setAttribute("aria-haspopup", "listbox");
    this.el.setAttribute("aria-expanded", "false");
    if (this.id) {
      this.el.id = this.id;
    }
    if (this.disabled) {
      this.el.classList.add("disabled");
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
    arrowEl.innerHTML = `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>`;

    this.triggerEl.appendChild(this.valueEl);
    this.triggerEl.appendChild(arrowEl);

    // Create dropdown (will be appended to body when opened - portal pattern)
    this.dropdownEl = document.createElement("div");
    this.dropdownEl.className = "cs-dropdown";
    this.dropdownEl.setAttribute("role", "listbox");

    // Create search container (only shown if many options)
    this.searchContainerEl = document.createElement("div");
    this.searchContainerEl.className = "cs-search-container";
    this.searchInputEl = document.createElement("input");
    this.searchInputEl.type = "text";
    this.searchInputEl.className = "cs-search-input";
    this.searchInputEl.placeholder = "Search...";
    this.searchInputEl.autocomplete = "off";
    this.searchContainerEl.appendChild(this.searchInputEl);

    // Create options container
    this.optionsContainerEl = document.createElement("div");
    this.optionsContainerEl.className = "cs-options-container";

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
      optionEl.dataset.value = opt.value;
      optionEl.dataset.index = index;
      optionEl.textContent = opt.label;
      optionEl.setAttribute("role", "option");

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
      this.valueEl.classList.remove("placeholder");
    } else {
      this.valueEl.textContent = this.placeholder;
      this.valueEl.classList.add("placeholder");
    }
  }

  _attachEvents() {
    this.triggerEl.addEventListener("click", this._handleTriggerClick);
    this.el.addEventListener("keydown", this._handleKeyDown);
    this.optionsContainerEl.addEventListener("click", this._handleOptionClick);
    this.searchInputEl.addEventListener("input", this._handleSearchInput);
    this.searchInputEl.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "Escape") {
        // Let the main keydown handler handle these
        return;
      }
      e.stopPropagation();
    });
    document.addEventListener("click", this._handleDocumentClick);
  }

  _detachEvents() {
    this.triggerEl.removeEventListener("click", this._handleTriggerClick);
    this.el.removeEventListener("keydown", this._handleKeyDown);
    this.optionsContainerEl.removeEventListener("click", this._handleOptionClick);
    this.searchInputEl.removeEventListener("input", this._handleSearchInput);
    document.removeEventListener("click", this._handleDocumentClick);
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
    this.toggle();
  }

  _handleDocumentClick(e) {
    // Check both the main element and the portal dropdown (which is in body)
    if (this.isOpen && !this.el.contains(e.target) && !this.dropdownEl.contains(e.target)) {
      this.close();
    }
  }

  _handleOptionClick(e) {
    const optionEl = e.target.closest(".cs-option");
    if (!optionEl || optionEl.classList.contains("disabled")) return;

    const value = optionEl.dataset.value;
    this._selectValue(value);
    this.close();
  }

  _handleKeyDown(e) {
    if (this.disabled) return;

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
          this.close();
        } else {
          this.open();
        }
        break;

      case "Escape":
        if (this.isOpen) {
          e.preventDefault();
          this.close();
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

    if (index >= 0 && index < this.options.length) {
      const optionEl = this.optionsContainerEl.querySelector(`.cs-option[data-index="${index}"]`);
      if (optionEl) {
        optionEl.classList.add("focused");
        // Scroll into view
        optionEl.scrollIntoView({ block: "nearest" });
      }
    }
  }

  _selectValue(value) {
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
    if (value !== prevValue) {
      this.onChange(value);
    }
  }

  open() {
    if (this.disabled || this.isOpen) return;

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

    // Add scroll/resize listeners to reposition dropdown
    this._addScrollResizeListeners();

    // Focus search input if visible
    if (this.options.length > 10) {
      setTimeout(() => this.searchInputEl.focus(), 50);
    }
  }

  close() {
    if (!this.isOpen) return;

    this.isOpen = false;
    this.el.classList.remove("open");
    this.el.setAttribute("aria-expanded", "false");
    this.focusedIndex = -1;

    // Remove focus styling
    const focused = this.dropdownEl.querySelector(".cs-option.focused");
    if (focused) {
      focused.classList.remove("focused");
    }

    // Remove scroll/resize listeners
    this._removeScrollResizeListeners();

    // Portal pattern: remove dropdown from body
    this._removeDropdownFromBody();

    // Clear search
    this.searchInputEl.value = "";
    this._handleSearchInput({ target: this.searchInputEl });
    this.searchString = "";
    if (this.searchTimeout) {
      clearTimeout(this.searchTimeout);
    }
  }

  toggle() {
    if (this.isOpen) {
      this.close();
    } else {
      this.open();
    }
  }

  _positionDropdown() {
    const triggerRect = this.triggerEl.getBoundingClientRect();
    const dropdownHeight = this.dropdownEl.offsetHeight || 240; // max-height fallback
    const viewportHeight = window.innerHeight;
    const spaceBelow = viewportHeight - triggerRect.bottom - 8;
    const spaceAbove = triggerRect.top - 8;

    // Set fixed positioning for portal
    this.dropdownEl.style.position = "fixed";
    this.dropdownEl.style.left = `${triggerRect.left}px`;
    this.dropdownEl.style.width = `${triggerRect.width}px`;
    this.dropdownEl.style.maxHeight = `${Math.min(240, Math.max(spaceBelow, spaceAbove))}px`;

    // Decide above or below based on available space
    if (spaceBelow < dropdownHeight && spaceAbove > spaceBelow) {
      // Position above the trigger
      this.dropdownEl.style.top = "auto";
      this.dropdownEl.style.bottom = `${viewportHeight - triggerRect.top + 4}px`;
      this.el.classList.add("dropdown-above");
    } else {
      // Position below the trigger
      this.dropdownEl.style.top = `${triggerRect.bottom + 4}px`;
      this.dropdownEl.style.bottom = "auto";
      this.el.classList.remove("dropdown-above");
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
      this.dropdownEl.classList.remove("cs-portal");
      document.body.removeChild(this.dropdownEl);
    }
  }

  _addScrollResizeListeners() {
    window.addEventListener("scroll", this._handleScrollResize, true);
    window.addEventListener("resize", this._handleScrollResize);
  }

  _removeScrollResizeListeners() {
    window.removeEventListener("scroll", this._handleScrollResize, true);
    window.removeEventListener("resize", this._handleScrollResize);
  }

  _handleScrollResize() {
    if (this.isOpen) {
      this._positionDropdown();
    }
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
  setValue(value) {
    const opt = this.options.find((o) => o.value === value);
    if (opt) {
      this._selectValue(value);
    }
  }

  /**
   * Update the available options
   * @param {Array<{value: string, label: string, selected?: boolean, disabled?: boolean}>} newOptions New options array
   */
  setOptions(newOptions) {
    this.options = newOptions;

    // Check if current selection is still valid
    const stillValid = this.options.find((o) => o.value === this.selectedValue);
    if (!stillValid) {
      // Select first selected option or clear selection
      const newSelected = this.options.find((o) => o.selected);
      this.selectedValue = newSelected ? newSelected.value : null;
      this.hiddenInput.value = this.selectedValue || "";
      this.el.dataset.value = this.selectedValue || "";
    }

    this._renderOptions();
    this._updateDisplayValue();
  }

  /**
   * Enable the select
   */
  enable() {
    this.disabled = false;
    this.el.classList.remove("disabled");
    this.el.tabIndex = 0;
  }

  /**
   * Disable the select
   */
  disable() {
    this.disabled = true;
    this.el.classList.add("disabled");
    this.el.tabIndex = -1;
    this.close();
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
    // Close first (removes scroll/resize listeners and dropdown from body)
    this.close();
    this._detachEvents();

    // Ensure dropdown is removed from body if still attached
    this._removeDropdownFromBody();

    // Restore original select if enhanced
    if (this._originalSelect) {
      this._originalSelect.style.display = "";
    }

    // Remove from DOM
    if (this.el && this.el.parentNode) {
      this.el.parentNode.removeChild(this.el);
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
