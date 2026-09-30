/**
 * Interface Tab Module - Theme, animations, display settings
 * Extracted from settings_dialog.js
 */
import * as Utils from "../../core/utils.js";
import { enhanceAllSelects } from "../custom_select.js";
import { setSoundsEnabled } from "../../core/sounds.js";
import { resetFeaturedRowConfigCache } from "../featured_row.js";

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
      <h3 class="settings-section-title" data-l10n-id="settings-interface-section-appearance"></h3>
      <div class="settings-group">
        <div class="settings-field">
          <div class="settings-field-info">
            <label data-l10n-id="settings-interface-theme-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-interface-theme-hint"></span>
          </div>
          <div class="settings-field-control">
            <select id="settingTheme" class="settings-select" data-custom-select>
              <option value="dark" ${activeTheme === "dark" ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-interface-theme-dark"))}</option>
              <option value="light" ${activeTheme === "light" ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-interface-theme-light"))}</option>
            </select>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label data-l10n-id="settings-interface-language-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-interface-language-hint"></span>
          </div>
          <div class="settings-field-control">
            <select id="settingLanguage" class="settings-select" data-custom-select data-cs-fit-options>
              <option value="system" selected>${Utils.escapeHtml(I18n.t("common-language-system"))}</option>
            </select>
          </div>
        </div>

        <div class="settings-field settings-field--logo-shape">
          <div class="settings-field-info">
            <label id="tokenLogoShapeLabel" data-l10n-id="settings-interface-logo-shape-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-interface-logo-shape-hint"></span>
          </div>
          <fieldset class="logo-shape-options" aria-labelledby="tokenLogoShapeLabel">
            <label class="logo-shape-option">
              <input type="radio" name="tokenLogoShape" value="circle" ${activeLogoShape === "circle" ? "checked" : ""}>
              <img class="logo-shape-option__preview logo-shape-option__preview--circle" src="/assets/logo.png" alt="">
              <span data-l10n-id="settings-interface-logo-shape-circle"></span>
            </label>
            <label class="logo-shape-option">
              <input type="radio" name="tokenLogoShape" value="rounded-square" ${activeLogoShape === "rounded-square" ? "checked" : ""}>
              <img class="logo-shape-option__preview logo-shape-option__preview--rounded-square" src="/assets/logo.png" alt="">
              <span data-l10n-id="settings-interface-logo-shape-natural"></span>
            </label>
          </fieldset>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label data-l10n-id="settings-interface-animations-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-interface-animations-hint"></span>
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
            <label data-l10n-id="settings-interface-compact-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-interface-compact-hint"></span>
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
      <h3 class="settings-section-title" data-l10n-id="settings-interface-section-data"></h3>
      <div class="settings-group">
        <div class="settings-field">
          <div class="settings-field-info">
            <label data-l10n-id="settings-interface-refresh-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-interface-refresh-hint"></span>
          </div>
          <div class="settings-field-control">
            <select id="settingPolling" class="settings-select" data-custom-select>
              <option value="1000" ${iface.polling_interval_ms === 1000 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-interface-refresh-seconds", { count: 1 }))}</option>
              <option value="2000" ${iface.polling_interval_ms === 2000 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-interface-refresh-seconds", { count: 2 }))}</option>
              <option value="5000" ${iface.polling_interval_ms === 5000 || !iface.polling_interval_ms ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-interface-refresh-seconds", { count: 5 }))}</option>
              <option value="10000" ${iface.polling_interval_ms === 10000 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-interface-refresh-seconds", { count: 10 }))}</option>
              <option value="30000" ${iface.polling_interval_ms === 30000 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-interface-refresh-seconds", { count: 30 }))}</option>
              <option value="60000" ${iface.polling_interval_ms === 60000 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-interface-refresh-minutes", { count: 1 }))}</option>
            </select>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label data-l10n-id="settings-interface-ticker-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-interface-ticker-hint"></span>
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
            <label data-l10n-id="settings-interface-page-size-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-interface-page-size-hint"></span>
          </div>
          <div class="settings-field-control">
            <select id="settingPageSize" class="settings-select" data-custom-select>
              <option value="10" ${iface.table_page_size === 10 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-interface-page-size-rows", { count: 10 }))}</option>
              <option value="25" ${iface.table_page_size === 25 || !iface.table_page_size ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-interface-page-size-rows", { count: 25 }))}</option>
              <option value="50" ${iface.table_page_size === 50 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-interface-page-size-rows", { count: 50 }))}</option>
              <option value="100" ${iface.table_page_size === 100 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-interface-page-size-rows", { count: 100 }))}</option>
            </select>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label data-l10n-id="settings-interface-auto-expand-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-interface-auto-expand-hint"></span>
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
            <label data-l10n-id="settings-interface-hints-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-interface-hints-hint"></span>
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
            <label data-l10n-id="settings-interface-featured-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-interface-featured-hint"></span>
          </div>
          <div class="settings-field-control">
            <label class="toggle">
              <input type="checkbox" id="settingShowFeaturedRow" ${iface.show_featured_row !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title" data-l10n-id="settings-interface-section-sound"></h3>
      <div class="settings-group">
        <div class="settings-field">
          <div class="settings-field-info">
            <label data-l10n-id="settings-interface-sounds-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-interface-sounds-hint"></span>
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
 * Fill a language select with `first` (an `{ code, name }` entry such as
 * "System") plus every registered locale, each shown by its native name, and
 * select the configured value.
 */
export async function populateLanguageOptions(select, current, first) {
  const lead = first ?? { code: "system", name: I18n.t("common-language-system") };
  try {
    const response = await fetch("/api/i18n/locales");
    if (!response.ok) throw new Error(response.statusText);
    const { locales } = await response.json();
    const options = [lead, ...locales];
    // A configured value that is not offered (a developer pseudo-locale) stays
    // visible, labelled by its code, instead of displaying as the lead entry.
    if (current && !options.some((o) => o.code === current)) {
      options.push({ code: current, name: current });
    }
    select.replaceChildren(
      ...options.map(({ code, name, dir }) => {
        const option = document.createElement("option");
        option.value = code;
        option.textContent = name;
        // A registered locale carries its direction: its native name renders in
        // its own script and direction.
        if (dir) {
          option.lang = code;
          option.dir = dir;
        }
        return option;
      })
    );
    select.value = options.some((o) => o.code === current) ? current : lead.code;
  } catch (error) {
    console.error("Failed to load display languages:", error);
  }
}

/**
 * Attach handlers for Interface tab
 */
export function attachInterfaceHandlers(dialog, content) {
  const fields = {
    theme: content.querySelector("#settingTheme"),
    language: content.querySelector("#settingLanguage"),
    logoShapes: content.querySelectorAll('input[name="tokenLogoShape"]'),
    animations: content.querySelector("#settingAnimations"),
    compact: content.querySelector("#settingCompact"),
    polling: content.querySelector("#settingPolling"),
    ticker: content.querySelector("#settingTicker"),
    pageSize: content.querySelector("#settingPageSize"),
    autoExpand: content.querySelector("#settingAutoExpand"),
    showHints: content.querySelector("#settingShowHints"),
    showFeaturedRow: content.querySelector("#settingShowFeaturedRow"),
    soundsEnabled: content.querySelector("#settingSoundsEnabled"),
  };

  const updateSetting = (path, value) => {
    if (!dialog.settings.dashboard) dialog.settings.dashboard = {};
    if (!dialog.settings.dashboard.interface) dialog.settings.dashboard.interface = {};
    dialog.settings.dashboard.interface[path] = value;
    dialog._checkForChanges();
  };

  if (fields.language) {
    populateLanguageOptions(fields.language, dialog.settings?.dashboard?.interface?.language);
    fields.language.addEventListener("change", (e) => updateSetting("language", e.target.value));
  }
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
  if (fields.showFeaturedRow) {
    fields.showFeaturedRow.addEventListener("change", (e) => {
      updateSetting("show_featured_row", e.target.checked);
      // The row caches this flag for 30s to keep every page entry from hitting
      // /api/config/gui. Without this drop, toggling the setting appears to do
      // nothing for up to half a minute.
      resetFeaturedRowConfigCache();
    });
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
