/**
 * Interface Tab Module - Theme, animations, display settings
 * Extracted from settings_dialog.js
 */
import { enhanceAllSelects } from "../custom_select.js";
import { setSoundsEnabled } from "../../core/sounds.js";

/**
 * Build Interface tab HTML
 */
export function buildInterfaceTab(settings) {
  const iface = settings?.dashboard?.interface || {};
  // Use the live DOM value as source of truth — the header toggle may have changed theme
  // without the gui config being updated (they're separate stores)
  const activeTheme = document.documentElement.getAttribute("data-theme") || iface.theme || "dark";
  const activeLogoShape =
    document.documentElement.getAttribute("data-token-logo-shape") === "rounded-square" ||
    iface.token_logo_shape === "rounded-square"
      ? "rounded-square"
      : "circle";

  return `
    <div class="settings-section">
      <h3 class="settings-section-title">Appearance</h3>
      <div class="settings-group">
        <div class="settings-field">
          <div class="settings-field-info">
            <label>Theme</label>
            <span class="settings-field-hint">Choose your preferred color scheme</span>
          </div>
          <div class="settings-field-control">
            <select id="settingTheme" class="settings-select" data-custom-select>
              <option value="dark" ${activeTheme === "dark" ? "selected" : ""}>Dark</option>
              <option value="light" ${activeTheme === "light" ? "selected" : ""}>Light</option>
            </select>
          </div>
        </div>

        <div class="settings-field settings-field--logo-shape">
          <div class="settings-field-info">
            <label id="tokenLogoShapeLabel">Token Logo Shape</label>
            <span class="settings-field-hint">Choose how token artwork appears throughout the app</span>
          </div>
          <fieldset class="logo-shape-options" aria-labelledby="tokenLogoShapeLabel">
            <label class="logo-shape-option">
              <input type="radio" name="tokenLogoShape" value="circle" ${activeLogoShape === "circle" ? "checked" : ""}>
              <img class="logo-shape-option__preview logo-shape-option__preview--circle" src="/assets/logo.png" alt="">
              <span>Circle</span>
            </label>
            <label class="logo-shape-option">
              <input type="radio" name="tokenLogoShape" value="rounded-square" ${activeLogoShape === "rounded-square" ? "checked" : ""}>
              <img class="logo-shape-option__preview logo-shape-option__preview--rounded-square" src="/assets/logo.png" alt="">
              <span>Square Rounded</span>
            </label>
          </fieldset>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Enable Animations</label>
            <span class="settings-field-hint">Smooth transitions and effects</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle">
              <input type="checkbox" id="settingAnimations" ${iface.enable_animations !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Compact Mode</label>
            <span class="settings-field-hint">Reduce padding for more content</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle">
              <input type="checkbox" id="settingCompact" ${iface.compact_mode ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title">Data & Display</h3>
      <div class="settings-group">
        <div class="settings-field">
          <div class="settings-field-info">
            <label>Refresh Interval</label>
            <span class="settings-field-hint">How often to refresh data</span>
          </div>
          <div class="settings-field-control">
            <select id="settingPolling" class="settings-select" data-custom-select>
              <option value="1000" ${iface.polling_interval_ms === 1000 ? "selected" : ""}>1 second</option>
              <option value="2000" ${iface.polling_interval_ms === 2000 ? "selected" : ""}>2 seconds</option>
              <option value="5000" ${iface.polling_interval_ms === 5000 || !iface.polling_interval_ms ? "selected" : ""}>5 seconds</option>
              <option value="10000" ${iface.polling_interval_ms === 10000 ? "selected" : ""}>10 seconds</option>
              <option value="30000" ${iface.polling_interval_ms === 30000 ? "selected" : ""}>30 seconds</option>
              <option value="60000" ${iface.polling_interval_ms === 60000 ? "selected" : ""}>1 minute</option>
            </select>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Show Ticker Bar</label>
            <span class="settings-field-hint">Live metrics ticker in header</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle">
              <input type="checkbox" id="settingTicker" ${iface.show_ticker_bar !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Table Page Size</label>
            <span class="settings-field-hint">Default rows per table page</span>
          </div>
          <div class="settings-field-control">
            <select id="settingPageSize" class="settings-select" data-custom-select>
              <option value="10" ${iface.table_page_size === 10 ? "selected" : ""}>10 rows</option>
              <option value="25" ${iface.table_page_size === 25 || !iface.table_page_size ? "selected" : ""}>25 rows</option>
              <option value="50" ${iface.table_page_size === 50 ? "selected" : ""}>50 rows</option>
              <option value="100" ${iface.table_page_size === 100 ? "selected" : ""}>100 rows</option>
            </select>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Auto-expand Categories</label>
            <span class="settings-field-hint">Expand config categories by default</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle">
              <input type="checkbox" id="settingAutoExpand" ${iface.auto_expand_categories ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Show Contextual Hints</label>
            <span class="settings-field-hint">Display help icons explaining dashboard features</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle">
              <input type="checkbox" id="settingShowHints" ${iface.show_hints !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Show Billboard Row</label>
            <span class="settings-field-hint">Display featured tokens row on Home and Tokens pages</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle">
              <input type="checkbox" id="settingShowBillboard" ${iface.show_billboard !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title">Sound Effects</h3>
      <div class="settings-group">
        <div class="settings-field">
          <div class="settings-field-info">
            <label>Enable Sounds</label>
            <span class="settings-field-hint">Subtle audio feedback for button clicks and actions</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle">
              <input type="checkbox" id="settingSoundsEnabled" ${iface.sounds_enabled !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>
      </div>
    </div>
  `;
}

/**
 * Attach handlers for Interface tab
 */
export function attachInterfaceHandlers(dialog, content) {
  const fields = {
    theme: content.querySelector("#settingTheme"),
    logoShapes: content.querySelectorAll('input[name="tokenLogoShape"]'),
    animations: content.querySelector("#settingAnimations"),
    compact: content.querySelector("#settingCompact"),
    polling: content.querySelector("#settingPolling"),
    ticker: content.querySelector("#settingTicker"),
    pageSize: content.querySelector("#settingPageSize"),
    autoExpand: content.querySelector("#settingAutoExpand"),
    showHints: content.querySelector("#settingShowHints"),
    showBillboard: content.querySelector("#settingShowBillboard"),
    soundsEnabled: content.querySelector("#settingSoundsEnabled"),
  };

  const updateSetting = (path, value) => {
    if (!dialog.settings.dashboard) dialog.settings.dashboard = {};
    if (!dialog.settings.dashboard.interface) dialog.settings.dashboard.interface = {};
    dialog.settings.dashboard.interface[path] = value;
    dialog._checkForChanges();
  };

  if (fields.theme) {
    fields.theme.addEventListener("change", (e) => updateSetting("theme", e.target.value));
  }
  fields.logoShapes.forEach((field) => {
    field.addEventListener("change", (e) => {
      if (e.target.checked) updateSetting("token_logo_shape", e.target.value);
    });
  });
  if (fields.animations) {
    fields.animations.addEventListener("change", (e) =>
      updateSetting("enable_animations", e.target.checked)
    );
  }
  if (fields.compact) {
    fields.compact.addEventListener("change", (e) =>
      updateSetting("compact_mode", e.target.checked)
    );
  }
  if (fields.polling) {
    fields.polling.addEventListener("change", (e) =>
      updateSetting("polling_interval_ms", parseInt(e.target.value, 10))
    );
  }
  if (fields.ticker) {
    fields.ticker.addEventListener("change", (e) =>
      updateSetting("show_ticker_bar", e.target.checked)
    );
  }
  if (fields.pageSize) {
    fields.pageSize.addEventListener("change", (e) =>
      updateSetting("table_page_size", parseInt(e.target.value, 10))
    );
  }
  if (fields.autoExpand) {
    fields.autoExpand.addEventListener("change", (e) =>
      updateSetting("auto_expand_categories", e.target.checked)
    );
  }
  if (fields.showHints) {
    fields.showHints.addEventListener("change", (e) =>
      updateSetting("show_hints", e.target.checked)
    );
  }
  if (fields.showBillboard) {
    fields.showBillboard.addEventListener("change", (e) =>
      updateSetting("show_billboard", e.target.checked)
    );
  }
  if (fields.soundsEnabled) {
    fields.soundsEnabled.addEventListener("change", (e) => {
      updateSetting("sounds_enabled", e.target.checked);
      setSoundsEnabled(e.target.checked);
    });
  }

  // Enhance all selects after attaching handlers
  enhanceAllSelects(content);
}
