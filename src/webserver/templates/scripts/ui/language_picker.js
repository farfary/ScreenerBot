// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Display-language selection: the registered-locale option list shared by Settings, and the picker on the first-run screens that saves the choice and reloads into it.
import * as Utils from "../core/utils.js";
import { apiErrorMessage } from "../core/request_manager.js";
import { enhanceAllSelects } from "./custom_select.js";

/**
 * Fill a language select with `first` (an `{ code, name }` entry such as
 * "System") plus every registered locale, each shown by its native name, and
 * select `current`; without one, the display language the server has
 * configured. Resolves to the selected value, or null when the list could not
 * be loaded.
 */
export async function populateLanguageOptions(select, current, first) {
  const lead = first ?? { code: "system", name: I18n.t("common-language-system") };
  try {
    const response = await fetch("/api/i18n/locales");
    if (!response.ok) throw new Error(response.statusText);
    const { locales, setting } = await response.json();
    const configured = current === undefined ? setting : current;
    const options = [lead, ...locales];
    // A configured value that is not offered (a developer pseudo-locale) stays
    // visible, labelled by its code, instead of displaying as the lead entry.
    if (configured && !options.some((o) => o.code === configured)) {
      options.push({ code: configured, name: configured });
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
    select.value = options.some((o) => o.code === configured) ? configured : lead.code;
    return select.value;
  } catch (error) {
    console.error("Failed to load display languages:", error);
    return null;
  }
}

/**
 * Mount the display-language picker into `container` (the onboarding and setup
 * headers). The configuration API is closed before setup, so the choice goes
 * through `POST /api/i18n/language`; the page then reloads, because the server
 * renders every page in the configured language. `locked()` reports whether a
 * reload would discard input the screen holds; while it does, the picker is
 * disabled and says where the language can be changed instead. `beforeReload`
 * lets the screen keep its own position across the reload.
 */
export function mountLanguagePicker(container, { locked = () => false, beforeReload } = {}) {
  container.innerHTML = `
    <i class="icon-languages language-picker-icon" aria-hidden="true"></i>
    <select class="language-picker-select" data-custom-select data-cs-fit-options></select>`;
  const select = container.querySelector("select");
  select.setAttribute("aria-label", I18n.t("common-language-label"));
  let saved = null;
  let saving = false;

  const refresh = () => {
    const isLocked = locked();
    select.disabled = saving || isLocked;
    if (isLocked) container.title = I18n.t("common-language-locked");
    else container.removeAttribute("title");
  };

  select.addEventListener("change", async () => {
    const language = select.value;
    if (language === saved || saving || locked()) return;
    saving = true;
    refresh();
    try {
      const response = await fetch("/api/i18n/language", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ language }),
      });
      if (!response.ok) {
        const body = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(body, response.statusText || `HTTP ${response.status}`));
      }
      beforeReload?.();
      window.location.reload();
    } catch (error) {
      console.error("Failed to save the display language:", error);
      select.value = saved;
      saving = false;
      refresh();
      Utils.showToast({ type: "error", title: error.message });
    }
  });

  void populateLanguageOptions(select).then((setting) => {
    saved = setting;
    enhanceAllSelects(container);
    refresh();
  });

  return { refresh };
}
