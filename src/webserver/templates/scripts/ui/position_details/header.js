/**
 * Header mixin for the Position Details dialog: identity, the four headline figures, the
 * status strip and the controls that act on the position.
 */
import * as Utils from "../../core/utils.js";
import * as Hints from "../../core/hints.js";
import { notificationManager } from "../../core/notifications.js";
import { HintTrigger } from "../hint_popover.js";
import { manualTrade } from "../manual_trade.js";

const STATUS_LABELS = { open: "Open", closed: "Closed", archived: "Archived" };

const MANAGEMENT_LABELS = {
  auto_trader: "Auto Trader",
  user_only: "User only",
  copy_task: "Copy task",
  hybrid: "Hybrid",
};

// Rugcheck normalised score: lower is safer. The level is the backend's banding of it.
const RISK_BADGES = {
  low: ["Low risk", "is-success"],
  medium: ["Medium risk", "is-warning"],
  high: ["High risk", "is-danger"],
};

const BUSY_LABELS = {
  buying: "Buy in progress…",
  selling: "Sell in progress…",
  closing: "Close in progress…",
};

export function applyHeaderMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  proto._renderHeader = function () {
    const pos = this._position();
    if (!pos || !this.dialogEl) return;

    this._renderIdentity(pos);
    this._paintRegion("#pddHeaderPrice", this._buildHeaderMetrics(pos));
    this._paintRegion("#pddHeaderBadges", this._buildHeaderBadges(pos));
    this._paintRegion("#pddPendingSwaps", this._buildPendingSwaps());
    this._paintRegion("#pddTradeActions", this._buildTradeActions());
    this._updateTradeButtonsState();
  };

  proto._renderIdentity = function (pos) {
    const symbol = pos.symbol || "";
    const name = pos.name || symbol || Utils.formatAddressCompact(pos.mint);
    const logoUrl = pos.logo_url || this.fullDetails?.token_info?.image_url || "";
    const status = this._status();
    const initial = Utils.escapeHtml((symbol || name || "?").charAt(0).toUpperCase());

    const html = `
      <div class="header-logo token-logo-frame">
        ${logoUrl ? `<img class="token-logo-artwork" src="${Utils.escapeHtml(logoUrl)}" alt="" />` : ""}
        <span class="logo-placeholder"${logoUrl ? " hidden" : ""}>${initial}</span>
      </div>
      <div class="header-identity">
        <div class="header-title">
          <span class="title-main" id="pdd-dialog-title" title="${Utils.escapeHtml(name)}">${Utils.escapeHtml(name)}</span>
          ${symbol ? `<span class="title-symbol token-symbol-type">$${Utils.escapeHtml(symbol.toUpperCase())}</span>` : ""}
          ${status ? `<span class="pdd-badge pdd-status is-${status}">${STATUS_LABELS[status] || Utils.escapeHtml(status)}</span>` : ""}
        </div>
        <div class="header-mint-full">${Utils.escapeHtml(pos.mint)}</div>
      </div>`;

    this._paintRegion("#pddIdentity", html, (el) => {
      const img = el.querySelector("img");
      img?.addEventListener(
        "error",
        () => {
          img.remove();
          el.querySelector(".logo-placeholder")?.removeAttribute("hidden");
        },
        { once: true }
      );
    });
  };

  /**
   * The four figures a position is read by. A holding shows where it stands now; a finished
   * round shows how it ended. Nothing here is repeated in the summary rail.
   */
  proto._buildHeaderMetrics = function (pos) {
    const metric = (label, value, { sub = "", tone = "", title = "" } = {}) => `
      <div class="header-metric ${tone}"${title ? ` title="${title}"` : ""}>
        <span class="header-metric-label">${label}</span>
        <span class="header-metric-value">${value}${value === "—" ? "" : "<small>SOL</small>"}</span>
        <span class="header-metric-sub">${sub || "&nbsp;"}</span>
      </div>`;

    const pnlSub = (pnl, pct) =>
      [pct != null ? this._formatPct(pct) : "", this._formatUsd(pnl)].filter(Boolean).join(" · ");
    const invested = pos.total_size_sol;
    const avgEntry = pos.average_entry_price || pos.entry_price;
    const entryMetric = metric("Avg entry", avgEntry ? this._formatPrice(avgEntry) : "—", {
      sub: this._plural(1 + (pos.dca_count || 0), "buy"),
    });

    if (this._isSettled()) {
      const exitPrice = pos.average_exit_price || pos.exit_price;
      return [
        metric("Exit price", exitPrice ? this._formatPrice(exitPrice) : "—", {
          sub: pos.exit_time ? `closed ${Utils.formatTimeAgo(pos.exit_time)}` : "",
        }),
        metric("Realized P&L", this._formatSol(pos.pnl, { sign: true, unit: false }), {
          sub: pnlSub(pos.pnl, pos.pnl_percent),
          tone: this._toneClass(pos.pnl),
          title: "USD at today's SOL price",
        }),
        metric("Returned", this._formatSol(pos.sol_received, { unit: false }), {
          sub: invested != null ? `of ${this._formatSol(invested)} invested` : "",
        }),
        entryMetric,
      ].join("");
    }

    // Archived before it settled: nothing tracks the position any more, so these are the last
    // known figures and must not read as live ones.
    const live = this._status() === "open";
    return [
      metric(live ? "Price" : "Last price", pos.current_price ? this._formatPrice(pos.current_price) : "—", {
        sub: pos.current_price_updated
          ? `pool · ${Utils.formatTimeAgo(pos.current_price_updated)}`
          : "",
      }),
      metric(live ? "Unrealized P&L" : "P&L at last price", this._formatSol(pos.unrealized_pnl, { sign: true, unit: false }), {
        sub: pnlSub(pos.unrealized_pnl, pos.unrealized_pnl_percent),
        tone: this._toneClass(pos.unrealized_pnl),
        title: "USD at today's SOL price",
      }),
      metric(live ? "Value" : "Last value", this._formatSol(this._calculateCurrentValue(pos), { unit: false }), {
        sub: invested != null ? `${this._formatSol(invested)} invested` : "",
      }),
      entryMetric,
    ].join("");
  };

  proto._buildHeaderBadges = function (pos) {
    const badges = [
      `<span class="pdd-badge" title="How this position was opened">${this._originLabel(pos.origin)}</span>`,
    ];
    if (this._status() === "open") badges.push(this._buildManagementControl(pos));

    const security = this.fullDetails?.security;
    if (security) {
      const [label, tone] = RISK_BADGES[String(security.risk_level).toLowerCase()] || [
        "Risk unknown",
        "",
      ];
      const score = security.score_normalized != null ? ` · ${security.score_normalized}/100` : "";
      badges.push(
        `<span class="pdd-badge ${tone}" title="Rugcheck score — lower is safer">${label}${score}</span>`
      );
    }
    if (pos.holding_state === "frozen") {
      badges.push(
        '<span class="pdd-badge is-danger" title="The mint authority froze this holding">Frozen</span>'
      );
    }
    return badges.join("");
  };

  proto._originLabel = function (origin) {
    const kind = origin?.kind || "auto";
    if (kind === "copy") {
      const wallet = String(origin.source_wallet || "unknown");
      const shortWallet = wallet.length > 14 ? `${wallet.slice(0, 6)}…${wallet.slice(-4)}` : wallet;
      return `Copied · task ${Utils.escapeHtml(String(origin.task_id ?? "unknown"))} · ${Utils.escapeHtml(shortWallet)}`;
    }
    if (kind === "manual") return "Manual entry";
    if (kind === "external") return "Wallet entry";
    return origin?.strategy_id ? `Auto · ${Utils.escapeHtml(origin.strategy_id)}` : "Auto entry";
  };

  /** Who may manage an open position. Copy positions add the copy-specific modes. */
  proto._buildManagementControl = function (pos) {
    const allowed =
      pos.origin?.kind === "copy" ? Object.keys(MANAGEMENT_LABELS) : ["auto_trader", "user_only"];
    const options = allowed
      .map(
        (value) =>
          `<option value="${value}"${pos.management === value ? " selected" : ""}>${MANAGEMENT_LABELS[value]}</option>`
      )
      .join("");
    const hint = Hints.getHint("positions.positionManagement");
    const hintHtml = hint
      ? HintTrigger.render(hint, "positions.positionManagement", { size: "sm", position: "bottom" })
      : "";

    return `
      <label class="pdd-management">
        <span>Managed by</span>
        <select id="pddManagementSelect" class="cs-sm" aria-label="Position management" data-custom-select>${options}</select>
      </label>${hintHtml}`;
  };

  /**
   * Swaps this position has submitted but not booked yet.
   *
   * A manual add returns success as soon as the swap is SUBMITTED; the position's invested
   * total, average entry and DCA count only move when the verifier applies it — seconds
   * later, with no event of its own. Without this the dialog read as broken: the toast said
   * the add went through and every figure still showed the pre-trade state. The amounts here
   * are what was SENT, never what the position has booked.
   */
  proto._buildPendingSwaps = function () {
    const pending = this.fullDetails?.pending_swaps || [];
    return pending
      .map((swap) => {
        const label =
          swap.kind === "dca"
            ? `Adding${swap.size_sol != null ? ` ${this._formatSol(swap.size_sol)}` : ""}`
            : `Selling${swap.exit_percentage != null ? ` ${Utils.formatNumber(swap.exit_percentage, 0)}%` : ""}`;
        return `<span class="pdd-pending" title="Submitted and waiting for on-chain confirmation. The figures update once it is verified."><i class="icon-loader spin"></i>${Utils.escapeHtml(label)} · confirming</span>`;
      })
      .join("");
  };

  /** Controls that act on the position: trade while it is open, otherwise its token. */
  proto._buildTradeActions = function () {
    const status = this._status();
    if (!status) return "";

    // Tone classes stay literal in each button so the button-contract audit can see them set.
    const attrs = (action, title) =>
      `data-trade-action="${action}" data-idle-title="${title}" title="${title}" aria-label="${title}"`;

    if (status === "open") {
      return `
        <button type="button" class="pdd-trade-btn is-add" ${attrs("add", "Add to position")}><i class="icon-circle-plus"></i><span>Add</span></button>
        <button type="button" class="pdd-trade-btn is-sell" ${attrs("sell", "Sell part of the position")}><i class="icon-scissors"></i><span>Sell</span></button>
        <button type="button" class="pdd-trade-btn is-close" ${attrs("close", "Sell everything and close")}><i class="icon-circle-x"></i><span>Close position</span></button>`;
    }
    return `<button type="button" class="pdd-trade-btn is-neutral" ${attrs("token", "Open token details")}><i class="icon-coins"></i><span>Token details</span></button>`;
  };

  /**
   * Whether a trade is processing for this position. Mirrors the positions list: an exit
   * pending verification ("closing"), or a live buy/sell action in flight for this mint.
   * @returns {"buying"|"selling"|"closing"|null}
   */
  proto._inFlightTradeState = function () {
    const pos = this._position();
    if (!pos) return null;
    if (pos.exit_transaction_signature && !pos.transaction_exit_verified) return "closing";

    const live = notificationManager.getInFlightTradeForMint(pos.mint);
    if (live) return live;

    // An action reports "completed" the moment its swap is SUBMITTED, so it stops covering
    // the position well before the position changes. A pending swap keeps the controls locked
    // for the rest of that window, which is exactly when a second add would be fired on top
    // of one still confirming.
    const pending = this.fullDetails?.pending_swaps?.[0];
    if (pending) return pending.kind === "dca" ? "buying" : "selling";
    return null;
  };

  /** Lock the trade controls while a trade is in flight, so it cannot be double-fired. */
  proto._updateTradeButtonsState = function () {
    const busy = this._inFlightTradeState();
    this.dialogEl
      ?.querySelectorAll('[data-trade-action]:not([data-trade-action="token"])')
      .forEach((btn) => {
        btn.disabled = busy !== null;
        btn.title = busy ? BUSY_LABELS[busy] : btn.dataset.idleTitle || "";
      });
  };

  proto._handleTradeAction = async function (action, btn) {
    const pos = this._position();
    if (!pos?.mint) return;
    // Captured before any await: close() nulls the position this dialog holds.
    const { mint, symbol, name } = pos;

    if (action === "token") {
      const logoUrl = pos.logo_url || this.fullDetails?.token_info?.image_url || null;
      this.close();
      window.dispatchEvent(
        new CustomEvent("screenerbot:open-token-details", {
          detail: { mint, symbol, name, logo_url: logoUrl },
        })
      );
      return;
    }

    // Add, sell and close all run through the shared manual-trade flow (ui/manual_trade.js),
    // which owns the dialog, payload, endpoints and toasts. Close is the sell dialog
    // preselected at 100%, which that flow submits as `close_all`, so no dust is left.
    const context =
      action === "add"
        ? { entrySize: pos.entry_size_sol }
        : action === "close"
          ? { preselect: 100 }
          : {};
    const icon = btn.querySelector("i");
    const idleIcon = icon?.className;
    if (icon) icon.className = "icon-loader spin";

    const placed = await manualTrade({
      action: action === "add" ? "add" : "sell",
      mint,
      symbol,
      btn,
      context,
    });

    if (icon) icon.className = idleIcon;
    if (!this.dialogEl) return;
    this._updateTradeButtonsState();
    if (!placed) return;

    this.onTradeComplete({ action, mint });
    if (action === "close") {
      this.close();
      return;
    }
    await this._fetchDetails();
  };

  proto._checkFavoriteState = async function () {
    const mint = this.positionData?.mint;
    if (!mint) return;
    try {
      const response = await fetch("/api/tokens/favorites");
      if (!response.ok) return;
      const data = await response.json();
      const favorites = data.favorites || [];
      this._updateFavoriteButton(favorites.some((favorite) => favorite.mint === mint));
    } catch {
      // Best-effort state check; the button stays usable if this request fails.
    }
  };

  proto._updateFavoriteButton = function (isFavorite) {
    const button = this.dialogEl?.querySelector("#pddFavoriteBtn");
    if (!button) return;
    const label = isFavorite ? "Remove from favorites" : "Add to favorites";
    button.classList.toggle("active", isFavorite);
    button.title = label;
    button.setAttribute("aria-label", label);
  };

  proto._toggleFavorite = async function () {
    const button = this.dialogEl?.querySelector("#pddFavoriteBtn");
    const position = this._position();
    if (!button || button.disabled || !position?.mint) return;

    const { mint, symbol, name, logo_url: logoUrl } = position;
    const currentlyFavorite = button.classList.contains("active");
    button.disabled = true;
    try {
      const response = currentlyFavorite
        ? await fetch(`/api/tokens/favorites/${encodeURIComponent(mint)}`, { method: "DELETE" })
        : await fetch("/api/tokens/favorites", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              mint,
              symbol: symbol || null,
              name: name || null,
              logo_url: logoUrl || null,
            }),
          });
      if (!response.ok) {
        throw new Error(currentlyFavorite ? "Failed to remove favorite" : "Failed to add favorite");
      }

      const isFavorite = !currentlyFavorite;
      this._updateFavoriteButton(isFavorite);
      Utils.showToast(
        `${symbol || "Token"} ${isFavorite ? "added to" : "removed from"} favorites`,
        "success"
      );
      window.dispatchEvent(
        new CustomEvent("screenerbot:favorites-changed", { detail: { mint, isFavorite } })
      );
    } catch (error) {
      Utils.showToast(error.message || "Failed to update favorites", "error");
    } finally {
      button.disabled = false;
    }
  };
}
