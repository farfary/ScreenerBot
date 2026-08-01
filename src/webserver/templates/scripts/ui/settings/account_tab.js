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
      <h3 class="settings-section-title">ScreenerBot account</h3>
      <p class="settings-section-description">
        Optional. Signing in adds free transaction sending, token voting and your referral
        earnings. Nothing in ScreenerBot requires an account, and nothing stops working without
        one.
      </p>

      <div class="account-panel" id="settingsAccountPanel"></div>
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title">Sending transactions</h3>
      <p class="settings-section-description">
        When you are signed in, ScreenerBot can broadcast your swaps through screenerbot.io
        instead of your own RPC. Your bot still builds and signs every transaction on this
        machine — the server only relays it, and cannot change a signed transaction without
        invalidating its signature.
      </p>

      <label class="settings-toggle-row">
        <input type="checkbox" id="settingsUseGateway" />
        <span class="settings-toggle-copy">
          <span class="settings-toggle-title">Use ScreenerBot RPC for sending transactions</span>
          <span class="settings-toggle-hint">
            Submission only. Price data always comes from your own RPC — pool polling is far too
            heavy for a shared endpoint, so it is never sent there.
          </span>
        </span>
      </label>
    </div>

    <div class="settings-section">
      <h3 class="settings-section-title">Managing your account</h3>
      <p class="settings-section-description">
        Your password, email address, connected devices and referral payouts are managed on the
        website. Revoking a device there signs it out everywhere, including this one.
      </p>
      <button type="button" class="account-btn account-btn-ghost" id="settingsOpenDashboard">
        Open your dashboard
      </button>
    </div>`;
}

export function attachAccountHandlers() {
  const container = document.getElementById("settingsAccountPanel");
  if (container && window.AccountPanel) {
    instance = window.AccountPanel.mount(container, {
      onChange: () => void syncGatewayToggle(),
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
