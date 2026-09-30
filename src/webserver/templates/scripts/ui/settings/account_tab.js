// Settings > Account.
//
// The same panel the setup screen mounts, plus the things that only make sense
// once you have an account: what the free RPC does, and where to manage the
// account itself.
//
// Deliberately thin. Anything that can be changed from the website is LINKED to
// rather than reimplemented here — password, email, connected devices and
// referral payouts all live at screenerbot.io/dashboard, and a second
// implementation in the app would be a second thing to keep correct.

const DASHBOARD_URL = "https://screenerbot.io/dashboard";

let instance = null;

export function buildAccountTab() {
  return `
    <div class="settings-section">
      <h3 class="settings-section-title" data-l10n-id="settings-account-title"></h3>
      <p class="settings-section-description" data-l10n-id="settings-account-description"></p>

      <div class="account-panel" id="settingsAccountPanel"></div>
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title" data-l10n-id="settings-account-data-title"></h3>
      <p class="settings-section-description" data-l10n-id="settings-account-data-description"></p>
      <div class="settings-data-access" id="settingsDataAccess" aria-live="polite"></div>
      <p class="settings-section-description" data-l10n-id="settings-account-data-fallback"></p>
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title" data-l10n-id="settings-account-gateway-title"></h3>
      <p class="settings-section-description" data-l10n-id="settings-account-gateway-description"></p>

      <label class="settings-toggle-row">
        <input type="checkbox" id="settingsUseGateway" />
        <span class="settings-toggle-copy">
          <span class="settings-toggle-title" data-l10n-id="settings-account-gateway-label"></span>
          <span class="settings-toggle-hint" data-l10n-id="settings-account-gateway-hint"></span>
        </span>
      </label>
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title" data-l10n-id="settings-account-manage-title"></h3>
      <p class="settings-section-description" data-l10n-id="settings-account-manage-description"></p>
      <button
        type="button"
        class="account-btn account-btn-ghost"
        id="settingsOpenDashboard"
        data-l10n-id="settings-account-open-dashboard"
      ></button>
    </div>`;
}

export function attachAccountHandlers() {
  const container = document.getElementById("settingsAccountPanel");
  if (container && window.AccountPanel) {
    instance = window.AccountPanel.mount(container, {
      // Settings has a "ScreenerBot data" section of its own below, so the panel
      // must not paint the same availability block a second time here.
      showDataAccess: false,
      // The panel already fetched the status this section renders, so it hands
      // it over rather than making Settings fetch the same thing again.
      onChange: (status) => {
        renderDataAccess(status?.data_access);
        void syncGatewayToggle();
      },
    });
  }

  const gateway = document.getElementById("settingsUseGateway");
  if (gateway) {
    void syncGatewayToggle();
    gateway.addEventListener("change", async () => {
      try {
        await fetch("/api/account/gateway", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ enabled: gateway.checked }),
        });
      } catch {
        // Revert the control rather than leave it claiming a state the config
        // does not have.
        gateway.checked = !gateway.checked;
      }
    });
  }

  const dashboard = document.getElementById("settingsOpenDashboard");
  if (dashboard) {
    dashboard.addEventListener("click", () => {
      fetch("/api/system/open-url", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: DASHBOARD_URL }),
      });
    });
  }
}

/**
 * Paint the data-service state.
 *
 * Every sentence comes from the backend (`data_server::access`), so this section,
 * the account panel and the first-run introduction cannot describe the same
 * state three different ways.
 */
function renderDataAccess(access) {
  const host = document.getElementById("settingsDataAccess");
  if (!host) return;

  if (!access) {
    host.innerHTML = "";
    return;
  }

  const escape = (value) =>
    String(value ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");

  host.innerHTML = `
    <div class="account-data" data-state="${escape(access.state)}">
      <p class="account-data-headline">
        <i class="${access.available ? "icon-circle-check" : "icon-info"}" aria-hidden="true"></i>
        <span>${escape(I18n.text(access.text))}</span>
      </p>
      <p class="account-data-detail">${escape(I18n.textAttr(access.text, "detail"))}</p>
    </div>`;
}

/** Read the current flag so the checkbox reflects config rather than a guess. */
async function syncGatewayToggle() {
  const gateway = document.getElementById("settingsUseGateway");
  if (!gateway) return;

  try {
    const response = await fetch("/api/config/account");
    if (!response.ok) return;
    const body = await response.json();
    gateway.checked = Boolean(body?.data?.use_gateway_rpc);
  } catch {
    // Leave the control alone; an unreadable config is not a reason to assert
    // a value the user did not choose.
  }
}

/** The dialog is closing. Stop the panel's sign-in watcher. */
export function teardownAccountTab() {
  instance?.destroy();
  instance = null;
}
