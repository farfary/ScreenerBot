/** Wallet observation UI for Wallets > Watched. */

const SOLANA_ADDRESS_RE = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

export function createWatchedWallets({ $, on, Utils, requestManager }) {
  let targets = [];
  let statuses = new Map();
  let lastRenderKey = "";
  let loading = false;

  function setup() {
    const form = $("#watched-wallet-form");
    const root = $("#watched-wallets-root");
    if (form) on(form, "submit", addTarget);
    if (root) on(root, "click", handleListAction);
  }

  async function load({ force = false } = {}) {
    if (loading && !force) return;
    loading = true;
    try {
      const data = await requestManager.fetch("/api/wallets/watch/", {
        priority: force ? "high" : "normal",
        skipDedup: force,
      });
      targets = data.targets || [];
      const statusEntries = await Promise.all(
        targets.map(async (target) => {
          try {
            const status = await requestManager.fetch(`/api/wallets/watch/${target.id}/status`);
            return [target.id, status];
          } catch {
            return [target.id, null];
          }
        })
      );
      statuses = new Map(statusEntries);
      render();
    } catch (error) {
      console.error("[Wallets] Failed to load watched addresses:", error);
      renderError("Watched addresses could not be loaded.");
    } finally {
      loading = false;
    }
  }

  async function addTarget(event) {
    event.preventDefault();
    const addressInput = $("#watched-wallet-address");
    const labelInput = $("#watched-wallet-label");
    const errorEl = $("#watched-wallet-address-error");
    const submit = event.currentTarget.querySelector('button[type="submit"]');
    const address = addressInput?.value.trim() || "";
    if (!SOLANA_ADDRESS_RE.test(address)) {
      if (errorEl) errorEl.textContent = "Enter a valid Solana wallet address.";
      addressInput?.setAttribute("aria-invalid", "true");
      return;
    }
    if (errorEl) errorEl.textContent = "";
    addressInput?.removeAttribute("aria-invalid");
    if (submit) submit.disabled = true;
    try {
      await requestManager.fetch("/api/wallets/watch/", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ address, label: labelInput?.value.trim() || null }),
        priority: "high",
        skipDedup: true,
      });
      event.currentTarget.reset();
      Utils.showToast("Wallet watch added", "success");
      await load({ force: true });
    } catch (error) {
      const message =
        error.status === 409
          ? "That wallet is already watched."
          : "Wallet watch could not be added.";
      if (errorEl) errorEl.textContent = message;
      Utils.showToast(message, "error");
    } finally {
      if (submit) submit.disabled = false;
    }
  }

  async function handleListAction(event) {
    const button = event.target.closest("button[data-watch-action]");
    if (!button) return;
    const id = Number(button.dataset.watchId);
    const action = button.dataset.watchAction;
    const target = targets.find((item) => item.id === id);
    if (!target) return;
    button.disabled = true;
    try {
      if (action === "toggle") {
        await requestManager.fetch(`/api/wallets/watch/${id}/enabled`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ enabled: !target.enabled }),
          priority: "high",
          skipDedup: true,
        });
        Utils.showToast(target.enabled ? "Wallet watch paused" : "Wallet watch enabled", "success");
      } else if (action === "delete") {
        await requestManager.fetch(`/api/wallets/watch/${id}`, {
          method: "DELETE",
          priority: "high",
          skipDedup: true,
        });
        Utils.showToast("Wallet watch removed", "success");
      }
      await load({ force: true });
    } catch (error) {
      console.error("[Wallets] Watch action failed:", error);
      Utils.showToast("Wallet watch could not be updated", "error");
      button.disabled = false;
    }
  }

  function render() {
    const root = $("#watched-wallets-root");
    const summary = $("#watched-wallets-summary");
    if (!root) return;
    if (summary)
      summary.textContent = `${targets.length} ${targets.length === 1 ? "address" : "addresses"}`;
    const key = JSON.stringify({ targets, statuses: [...statuses.entries()] });
    if (key === lastRenderKey) return;
    lastRenderKey = key;
    if (targets.length === 0) {
      root.innerHTML =
        '<div class="watched-wallets-empty"><i class="icon-eye"></i><strong>No watched addresses</strong><span>Add a public wallet above to begin recording its activity.</span></div>';
      return;
    }
    root.innerHTML = targets.map(renderTarget).join("");
  }

  function renderTarget(target) {
    const status = statuses.get(target.id);
    const shortAddress = `${target.address.slice(0, 6)}…${target.address.slice(-4)}`;
    const state = !target.enabled ? "Paused" : status?.subscribed ? "Streaming" : "Polling";
    const stateClass = !target.enabled
      ? "is-paused"
      : status?.subscribed
        ? "is-streaming"
        : "is-polling";
    const lastActivity = status?.last_activity_at
      ? formatTime(status.last_activity_at)
      : "No activity yet";
    const label = target.label || "Unlabelled wallet";
    return `<article class="watched-wallet-row">
      <div class="watched-wallet-identity"><strong>${Utils.escapeHtml(label)}</strong><button class="watched-wallet-address" type="button" data-copy="${Utils.escapeHtml(target.address)}" title="Copy address">${Utils.escapeHtml(shortAddress)}</button></div>
      <div class="watched-wallet-state ${stateClass}"><i class="icon-activity"></i><span>${state}</span></div>
      <div class="watched-wallet-last"><span>Last sync</span><strong>${Utils.escapeHtml(lastActivity)}</strong></div>
      <div class="watched-wallet-actions"><button class="btn" type="button" data-watch-action="toggle" data-watch-id="${target.id}">${target.enabled ? "Pause" : "Enable"}</button><button class="btn-icon danger" type="button" data-watch-action="delete" data-watch-id="${target.id}" title="Remove" aria-label="Remove ${Utils.escapeHtml(label)}"><i class="icon-trash-2"></i></button></div>
    </article>`;
  }

  function renderError(message) {
    const root = $("#watched-wallets-root");
    if (root)
      root.innerHTML = `<div class="watched-wallets-empty"><i class="icon-circle-alert"></i><strong>${Utils.escapeHtml(message)}</strong><span>Use refresh to try again.</span></div>`;
  }

  function formatTime(value) {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? "Unknown" : date.toLocaleString();
  }

  function reset() {
    targets = [];
    statuses = new Map();
    lastRenderKey = "";
    loading = false;
  }

  return { setup, load, reset };
}
