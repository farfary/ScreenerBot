/**
 * Position Modal Module
 * Handles all position detail modal functionality including:
 * - Modal state management
 * - Position detail fetching and rendering
 * - Executions, transactions, and timeline rendering
 * - Copy/link actions (GMGN, DexScreener, Solscan)
 */
(function () {
  "use strict";

  const REFETCH_INTERVAL_MS = 15000;

  const state = {
    currentKey: null,
    keydownHandler: null,
    detail: null,
    detailKey: null,
    abortController: null,
    lastFetchedAt: 0,
  };

  const FIELD_IDS = [
    "positionMint",
    "positionId",
    "positionState",
    "positionPositionType",
    "positionLiquidity Tier",
    "positionProfitTargets",
    "positionPhantom",
    "positionPhantomFirstSeen",
    "positionSynthetic",
    "positionEntryPrice",
    "positionEffectiveEntryPrice",
    "positionEntrySize",
    "positionTotalSize",
    "positionPriceHigh",
    "positionPriceLow",
    "positionCurrentPrice",
    "positionCurrentUpdated",
    "positionEntryTime",
    "positionEntrySig",
    "positionEntryVerified",
    "positionEntryFee",
    "positionTokenAmount",
    "positionExitPrice",
    "positionEffectiveExitPrice",
    "positionSolReceived",
    "positionExitTime",
    "positionExitSig",
    "positionExitVerified",
    "positionExitFee",
    "positionClosedReason",
    "positionModalSymbol",
    "positionModalName",
  ];

  // ===== Helper Functions =====

  function getPositionFromStore(key) {
    if (!key || !window.positionsStore) {
      return null;
    }
    try {
      return window.positionsStore.get(key) || null;
    } catch (_) {
      return null;
    }
  }

  function setModalText(id, value) {
    const el = document.getElementById(id);
    if (!el) return;
    if (value === undefined || value === null) {
      el.textContent = "—";
      return;
    }
    el.textContent = String(value);
  }

  function formatPositionTimestamp(value) {
    if (typeof Utils !== "undefined" && Utils?.formatTimeFromSeconds) {
      return Utils.formatTimeFromSeconds(value, { fallback: "—" });
    }
    if (!Number.isFinite(value)) return "—";
    const date = new Date(value * 1000);
    if (Number.isNaN(date.getTime())) {
      return "—";
    }
    return date.toLocaleString();
  }

  function formatPositionPrice(value) {
    if (value === null || value === undefined) {
      return "—";
    }
    if (typeof Utils !== "undefined" && Utils?.formatPriceSol) {
      return Utils.formatPriceSol(value, { fallback: "—" });
    }
    const num = Number(value);
    if (!Number.isFinite(num)) return "—";
    return num.toFixed(12);
  }

  function formatPositionSol(value, { decimals = 4, fallback = "—" } = {}) {
    if (value === null || value === undefined) {
      return fallback;
    }
    if (typeof Utils !== "undefined" && Utils?.formatSol) {
      return Utils.formatSol(value, { decimals, fallback });
    }
    const num = Number(value);
    if (!Number.isFinite(num)) return fallback;
    return `${num.toFixed(decimals)} SOL`;
  }

  function formatPositionSolDelta(value) {
    if (value === null || value === undefined) return "—";
    const num = Number(value);
    if (!Number.isFinite(num)) return "—";
    const formatted = formatPositionSol(Math.abs(num));
    if (formatted === "—") return formatted;
    if (num > 0) return `+${formatted}`;
    if (num < 0) return `-${formatted}`;
    return formatted;
  }

  function formatPercentPlain(value) {
    const num = Number(value);
    if (!Number.isFinite(num)) return "—";
    const magnitude = Math.abs(num).toFixed(2);
    if (num > 0) return `+${magnitude}%`;
    if (num < 0) return `-${magnitude}%`;
    return `${magnitude}%`;
  }

  function formatBooleanFlag(value) {
    if (typeof Utils !== "undefined" && Utils?.formatBooleanFlag) {
      return Utils.formatBooleanFlag(value, "—");
    }
    if (value === true) return "Yes";
    if (value === false) return "No";
    return "—";
  }

  function formatTokenAmount(value) {
    if (value === null || value === undefined) return "—";
    const num = Number(value);
    if (!Number.isFinite(num)) return String(value);
    if (typeof Utils !== "undefined" && Utils?.formatNumber) {
      return Utils.formatNumber(num, {
        decimals: 0,
        fallback: "—",
        useGrouping: true,
      });
    }
    return num.toLocaleString();
  }

  function lamportsToSolNumber(value) {
    const num = Number(value);
    if (!Number.isFinite(num)) return null;
    return num / 1_000_000_000;
  }

  function formatLamportsToSol(value) {
    const sol = lamportsToSolNumber(value);
    if (sol === null) return "—";
    return formatPositionSol(sol, { decimals: 6, fallback: "—" });
  }

  function formatProfitTargets(min, max) {
    const hasMin = Number.isFinite(Number(min));
    const hasMax = Number.isFinite(Number(max));
    if (!hasMin && !hasMax) {
      return "—";
    }
    const minText = hasMin ? formatPercentPlain(min) : "—";
    const maxText = hasMax ? formatPercentPlain(max) : "—";
    return `${minText} / ${maxText}`;
  }

  function setModalPnl(id, value, { decimals = 4 } = {}) {
    const el = document.getElementById(id);
    if (!el) return;
    el.classList.remove("pnl-positive", "pnl-negative", "pnl-neutral");
    if (value === null || value === undefined) {
      el.textContent = "—";
      el.classList.add("pnl-neutral");
      return;
    }

    const num = Number(value);
    if (!Number.isFinite(num)) {
      el.textContent = "—";
      el.classList.add("pnl-neutral");
      return;
    }

    const sign = num > 0 ? "+" : num < 0 ? "-" : "";
    const magnitude = Math.abs(num).toFixed(decimals);
    el.textContent = `${sign}${magnitude} SOL`;

    if (num > 0) {
      el.classList.add("pnl-positive");
    } else if (num < 0) {
      el.classList.add("pnl-negative");
    } else {
      el.classList.add("pnl-neutral");
    }
  }

  function setModalPercent(id, value, { decimals = 2 } = {}) {
    const el = document.getElementById(id);
    if (!el) return;
    el.classList.remove("pnl-positive", "pnl-negative", "pnl-neutral");
    if (value === null || value === undefined) {
      el.textContent = "—";
      el.classList.add("pnl-neutral");
      return;
    }

    const num = Number(value);
    if (!Number.isFinite(num)) {
      el.textContent = "—";
      el.classList.add("pnl-neutral");
      return;
    }

    const sign = num > 0 ? "+" : num < 0 ? "-" : "";
    const magnitude = Math.abs(num).toFixed(decimals);
    el.textContent = `${sign}${magnitude}%`;

    if (num > 0) {
      el.classList.add("pnl-positive");
    } else if (num < 0) {
      el.classList.add("pnl-negative");
    } else {
      el.classList.add("pnl-neutral");
    }
  }

  function getPositionStatusInfo(position) {
    if (!position) {
      return {
        label: "Unknown",
        className: "status-closed",
        state: "Unknown",
      };
    }
    if (position.synthetic_exit) {
      return {
        label: "Synthetic",
        className: "status-synthetic",
        state: "Synthetic Exit",
      };
    }
    if (position.transaction_exit_verified) {
      return {
        label: "Closed",
        className: "status-closed",
        state: "Closed",
      };
    }
    return {
      label: "Open",
      className: "status-open",
      state: "Open",
    };
  }

  function applyStatusBadge(element, info) {
    if (!element || !info) return;
    element.classList.remove(
      "status-open",
      "status-closed",
      "status-synthetic"
    );
    element.classList.add(info.className);
    element.textContent = info.label.toUpperCase();
  }

  function setLoading(text) {
    const loading = document.getElementById("positionModalLoading");
    const content = document.getElementById("positionModalContent");
    const error = document.getElementById("positionModalError");
    if (loading) {
      loading.classList.add("active");
      const textEl = document.getElementById("positionModalLoadingText");
      if (textEl) {
        textEl.textContent = text || "Loading position details...";
      }
    }
    if (content) {
      content.classList.add("hidden");
    }
    if (error) {
      error.classList.remove("active");
      error.textContent = "";
    }
  }

  function clearError() {
    const error = document.getElementById("positionModalError");
    if (error) {
      error.classList.remove("active");
      error.textContent = "";
    }
  }

  function setError(message) {
    const loading = document.getElementById("positionModalLoading");
    const content = document.getElementById("positionModalContent");
    const error = document.getElementById("positionModalError");
    if (loading) {
      loading.classList.remove("active");
    }
    if (content) {
      content.classList.add("hidden");
    }
    if (error) {
      error.textContent = message || "Failed to load position details";
      error.classList.add("active");
    }
  }

  function showContent() {
    const loading = document.getElementById("positionModalLoading");
    const content = document.getElementById("positionModalContent");
    if (loading) {
      loading.classList.remove("active");
    }
    if (content) {
      content.classList.remove("hidden");
    }
  }

  function renderModal(detail) {
    if (!detail || !detail.position) {
      setError("Position detail unavailable");
      return;
    }

    const position = detail.position;
    clearError();
    showContent();

    setModalText("positionModalSymbol", position.symbol || "—");
    setModalText("positionModalName", position.name || "—");
    setModalText("positionMint", position.mint || "—");
    setModalText(
      "positionId",
      position.id !== undefined && position.id !== null
        ? String(position.id)
        : "—"
    );

    const statusInfo = getPositionStatusInfo(position);
    const statusEl = document.getElementById("positionModalStatus");
    applyStatusBadge(statusEl, statusInfo);
    setModalText("positionState", statusInfo.state);

    setModalText(
      "positionPositionType",
      position.position_type ? position.position_type.toUpperCase() : "—"
    );
    setModalText("positionLiquidityTier", position.liquidity_tier || "—");
    setModalText(
      "positionProfitTargets",
      formatProfitTargets(
        position.profit_target_min,
        position.profit_target_max
      )
    );

    const phantomLabel = position.phantom_remove ? "Remove flagged" : "No";
    const phantomConfirmations = Number(position.phantom_confirmations) || 0;
    setModalText(
      "positionPhantom",
      `${phantomLabel} (${phantomConfirmations} confirmations)`
    );
    setModalText(
      "positionPhantomFirstSeen",
      formatPositionTimestamp(position.phantom_first_seen)
    );
    setModalText(
      "positionSynthetic",
      formatBooleanFlag(position.synthetic_exit)
    );

    setModalText(
      "positionEntryPrice",
      formatPositionPrice(
        position.effective_entry_price || position.entry_price
      )
    );
    setModalText(
      "positionEffectiveEntryPrice",
      formatPositionPrice(position.effective_entry_price)
    );
    setModalText(
      "positionEntrySize",
      formatPositionSol(position.entry_size_sol)
    );
    setModalText(
      "positionTotalSize",
      formatPositionSol(position.total_size_sol)
    );
    setModalText(
      "positionPriceHigh",
      formatPositionPrice(position.price_highest)
    );
    setModalText(
      "positionPriceLow",
      formatPositionPrice(position.price_lowest)
    );
    setModalText(
      "positionCurrentPrice",
      formatPositionPrice(position.current_price)
    );
    setModalText(
      "positionCurrentUpdated",
      formatPositionTimestamp(position.current_price_updated)
    );
    setModalPnl("positionUnrealizedPnl", position.unrealized_pnl);
    setModalPercent(
      "positionUnrealizedPercent",
      position.unrealized_pnl_percent
    );
    setModalPnl("positionRealizedPnl", position.pnl);
    setModalPercent("positionRealizedPercent", position.pnl_percent);

    setModalText(
      "positionEntryTime",
      formatPositionTimestamp(position.entry_time)
    );
    setModalText(
      "positionEntrySig",
      position.entry_transaction_signature
        ? Utils.formatAddressCompact(position.entry_transaction_signature, {
            startChars: 8,
            endChars: 6,
          })
        : "—"
    );
    setModalText(
      "positionEntryVerified",
      formatBooleanFlag(position.transaction_entry_verified)
    );
    setModalText(
      "positionEntryFee",
      formatLamportsToSol(position.entry_fee_lamports)
    );
    setModalText(
      "positionTokenAmount",
      formatTokenAmount(position.token_amount)
    );

    setModalText("positionExitPrice", formatPositionPrice(position.exit_price));
    setModalText(
      "positionEffectiveExitPrice",
      formatPositionPrice(position.effective_exit_price)
    );
    setModalText(
      "positionSolReceived",
      formatPositionSol(position.sol_received)
    );
    setModalText(
      "positionExitTime",
      formatPositionTimestamp(position.exit_time)
    );
    setModalText(
      "positionExitSig",
      position.exit_transaction_signature
        ? Utils.formatAddressCompact(position.exit_transaction_signature, {
            startChars: 8,
            endChars: 6,
          })
        : "—"
    );
    setModalText(
      "positionExitVerified",
      formatBooleanFlag(position.transaction_exit_verified)
    );
    setModalText(
      "positionExitFee",
      formatLamportsToSol(position.exit_fee_lamports)
    );
    setModalText("positionClosedReason", position.closed_reason || "—");

    const exitSection = document.getElementById("positionExitSection");
    if (exitSection) {
      const shouldShowExit =
        position.transaction_exit_verified ||
        position.synthetic_exit ||
        position.exit_time !== null;
      exitSection.style.display = shouldShowExit ? "block" : "none";
    }

    renderExecutions(Array.isArray(detail.executions) ? detail.executions : []);
    renderTransactions(
      Array.isArray(detail.transactions) ? detail.transactions : []
    );
    renderTimeline(
      Array.isArray(detail.state_history) ? detail.state_history : []
    );
  }

  function resetModal() {
    FIELD_IDS.forEach((id) => setModalText(id, "—"));
    setModalPnl("positionUnrealizedPnl", null);
    setModalPercent("positionUnrealizedPercent", null);
    setModalPnl("positionRealizedPnl", null);
    setModalPercent("positionRealizedPercent", null);
    renderExecutions([]);
    renderTransactions([]);
    renderTimeline([]);
    const loading = document.getElementById("positionModalLoading");
    const content = document.getElementById("positionModalContent");
    if (loading) loading.classList.remove("active");
    if (content) content.classList.add("hidden");
    clearError();
    const statusEl = document.getElementById("positionModalStatus");
    if (statusEl) {
      applyStatusBadge(statusEl, {
        label: "Open",
        className: "status-open",
        state: "Open",
      });
    }
  }

  function renderExecutions(executions) {
    const tbody = document.getElementById("positionExecutionsBody");
    if (!tbody) return;

    if (!Array.isArray(executions) || executions.length === 0) {
      tbody.innerHTML =
        '<tr><td colspan="11" class="position-empty">No execution data</td></tr>';
      return;
    }

    tbody.innerHTML = executions
      .map((execution) => {
        const kind = Utils.escapeHtml((execution.kind || "—").toUpperCase());
        const timestamp = formatPositionTimestamp(execution.timestamp);
        const price = formatPositionPrice(execution.price_sol);
        const effective = formatPositionPrice(execution.effective_price_sol);
        const sizeSol = formatPositionSol(execution.size_sol);
        const totalTokens = formatTokenAmount(execution.token_amount);
        const solDelta = formatPositionSolDelta(execution.sol_delta);
        const feeValue =
          execution.fee_sol ??
          (execution.fee_lamports != null
            ? lamportsToSolNumber(execution.fee_lamports)
            : null);
        const feeSol = formatPositionSol(feeValue, {
          decimals: 6,
          fallback: "—",
        });
        const verified = execution.verified
          ? '<span class="status-badge status-open">VERIFIED</span>'
          : '<span class="status-badge status-closed">PENDING</span>';
        const signatureCell = renderSignatureCell(execution.signature);
        const notes = execution.notes ? Utils.escapeHtml(execution.notes) : "—";

        return `
          <tr>
            <td>${kind}</td>
            <td>${Utils.escapeHtml(timestamp)}</td>
            <td>${Utils.escapeHtml(price)}</td>
            <td>${Utils.escapeHtml(effective)}</td>
            <td>${Utils.escapeHtml(sizeSol)}</td>
            <td>${Utils.escapeHtml(solDelta)}</td>
            <td>${Utils.escapeHtml(totalTokens)}</td>
            <td>${Utils.escapeHtml(feeSol)}</td>
            <td>${verified}</td>
            <td>${signatureCell}</td>
            <td>${notes}</td>
          </tr>
        `;
      })
      .join("");
  }

  function renderTransactions(transactions) {
    const tbody = document.getElementById("positionTransactionsBody");
    if (!tbody) return;

    if (!Array.isArray(transactions) || transactions.length === 0) {
      tbody.innerHTML =
        '<tr><td colspan="11" class="position-empty">No transaction data</td></tr>';
      return;
    }

    tbody.innerHTML = transactions
      .map((tx) => {
        const kind = Utils.escapeHtml((tx.kind || "—").toUpperCase());
        const status = tx.status
          ? Utils.escapeHtml(tx.status)
          : tx.available
          ? "—"
          : "Unavailable";
        const success =
          tx.success === true ? "✅" : tx.success === false ? "❌" : "—";
        const timestamp = formatPositionTimestamp(tx.timestamp);
        const slot = tx.slot !== null && tx.slot !== undefined ? tx.slot : "—";
        const feeValue =
          tx.fee_sol ??
          (tx.fee_lamports != null
            ? lamportsToSolNumber(tx.fee_lamports)
            : null);
        const fee = formatPositionSol(feeValue, {
          decimals: 6,
          fallback: "—",
        });
        const direction = tx.direction ? Utils.escapeHtml(tx.direction) : "—";
        const router = tx.router ? Utils.escapeHtml(tx.router) : "—";
        const solChange = formatPositionSolDelta(tx.sol_change);
        const signatureCell = renderSignatureCell(tx.signature);
        const notes = tx.notes
          ? Utils.escapeHtml(tx.notes)
          : tx.available
          ? "—"
          : "Transaction data not indexed yet";

        return `
          <tr>
            <td>${kind}</td>
            <td>${status}</td>
            <td>${success}</td>
            <td>${Utils.escapeHtml(timestamp)}</td>
            <td>${Utils.escapeHtml(slot)}</td>
            <td>${Utils.escapeHtml(fee)}</td>
            <td>${direction}</td>
            <td>${router}</td>
            <td>${Utils.escapeHtml(solChange)}</td>
            <td>${signatureCell}</td>
            <td>${notes}</td>
          </tr>
        `;
      })
      .join("");
  }

  function renderTimeline(entries) {
    const list = document.getElementById("positionStateTimeline");
    if (!list) return;

    if (!Array.isArray(entries) || entries.length === 0) {
      list.innerHTML =
        '<li class="position-empty">No state history recorded</li>';
      return;
    }

    list.innerHTML = entries
      .map((entry) => {
        const state = Utils.escapeHtml(entry.state || "—");
        const timestamp = formatPositionTimestamp(entry.changed_at);
        const reason = entry.reason ? Utils.escapeHtml(entry.reason) : "";
        const reasonMarkup = reason
          ? `<div style="font-size:0.85rem;color:var(--text-secondary);">${reason}</div>`
          : "";

        return `
          <li class="position-timeline-item">
            <div>
              <strong>${state}</strong>
              ${reasonMarkup}
            </div>
            <div class="position-timeline-meta">
              <span>${Utils.escapeHtml(timestamp)}</span>
            </div>
          </li>
        `;
      })
      .join("");
  }

  function renderSignatureCell(signature) {
    if (!signature) {
      return "<span>—</span>";
    }
    const truncated = Utils.formatAddressCompact(signature, {
      startChars: 8,
      endChars: 6,
    });
    const escapedSignature = Utils.escapeHtml(signature);
    return `
      <span class="position-signature-cell">
        <span title="${escapedSignature}">${Utils.escapeHtml(truncated)}</span>
        <button
          class="position-signature-button"
          type="button"
          data-signature="${escapedSignature}"
        >
          Copy
        </button>
      </span>
    `;
  }

  // ===== Modal Lifecycle =====

  function handleKeyDown(event) {
    if (event.key === "Escape") {
      close();
    }
  }

  function ensureKeyListener() {
    if (state.keydownHandler) {
      return;
    }
    state.keydownHandler = handleKeyDown;
    document.addEventListener("keydown", state.keydownHandler);
  }

  function removeKeyListener() {
    if (!state.keydownHandler) {
      return;
    }
    document.removeEventListener("keydown", state.keydownHandler);
    state.keydownHandler = null;
  }

  function getActiveDetail() {
    return state.detail || null;
  }

  function getCurrentPosition() {
    return getPositionFromStore(state.currentKey);
  }

  async function open(key) {
    if (!key) return;
    state.currentKey = key;

    const overlay = document.getElementById("positionModal");
    if (overlay) {
      overlay.classList.add("active");
    }
    ensureKeyListener();

    const cachedDetail = state.detailKey === key ? getActiveDetail() : null;
    if (cachedDetail && cachedDetail.position) {
      renderModal(cachedDetail);
    } else {
      setLoading("Loading position details...");
    }

    try {
      await loadDetail(key, { silent: !!cachedDetail });
    } catch (err) {
      console.error("[PositionModal] Failed to load detail:", err);
      if (!cachedDetail) {
        setError("Failed to load position details");
      }
    }
  }

  function close(event) {
    if (event && event.target && event.currentTarget) {
      if (event.target !== event.currentTarget && event.type === "click") {
        return;
      }
    }
    if (state.abortController) {
      state.abortController.abort();
      state.abortController = null;
    }
    const overlay = document.getElementById("positionModal");
    if (overlay) {
      overlay.classList.remove("active");
    }
    state.currentKey = null;
    state.detail = null;
    state.detailKey = null;
    state.lastFetchedAt = 0;
    removeKeyListener();
    resetModal();
  }

  async function loadDetail(key, { silent = false } = {}) {
    if (!key) return;

    if (state.abortController) {
      state.abortController.abort();
    }

    const controller = new AbortController();
    state.abortController = controller;

    if (!silent) {
      setLoading("Loading position details...");
    }

    try {
      const response = await fetch(
        `/api/positions/${encodeURIComponent(key)}/details`,
        { signal: controller.signal }
      );

      if (!response.ok) {
        throw new Error(`Request failed with status ${response.status}`);
      }

      const detail = await response.json();
      if (controller.signal.aborted) {
        return;
      }

      state.detail = detail;
      state.detailKey = key;
      state.lastFetchedAt = Date.now();
      renderModal(detail);
    } catch (err) {
      if (controller.signal.aborted) {
        return;
      }
      if (!silent) {
        setError(err.message || "Failed to load position details");
      } else {
        console.warn("[PositionModal] loadDetail (silent) failed:", err);
      }
    } finally {
      if (state.abortController === controller) {
        state.abortController = null;
      }
    }
  }

  function mergeDetailWithStore(detailPosition, storePosition, detail) {
    if (!detailPosition || !storePosition) {
      return;
    }

    const passthroughFields = [
      "current_price",
      "current_price_updated",
      "unrealized_pnl",
      "unrealized_pnl_percent",
      "pnl",
      "pnl_percent",
      "exit_price",
      "effective_exit_price",
      "exit_time",
      "sol_received",
      "transaction_exit_verified",
      "synthetic_exit",
      "closed_reason",
      "price_highest",
      "price_lowest",
      "phantom_confirmations",
      "phantom_remove",
      "phantom_first_seen",
    ];

    passthroughFields.forEach((field) => {
      if (Object.prototype.hasOwnProperty.call(storePosition, field)) {
        detailPosition[field] = storePosition[field];
      }
    });

    if (detail && Array.isArray(detail.executions)) {
      detail.executions = detail.executions.map((execution) => {
        if (!execution || typeof execution !== "object") {
          return execution;
        }

        if ((execution.kind || "").toLowerCase() === "entry") {
          const sizeSol =
            detailPosition.entry_size_sol ?? execution.size_sol ?? null;
          const feeSol =
            detailPosition.entry_fee_lamports != null
              ? lamportsToSolNumber(detailPosition.entry_fee_lamports)
              : execution.fee_sol;

          return {
            ...execution,
            timestamp: detailPosition.entry_time ?? execution.timestamp,
            price_sol: detailPosition.entry_price ?? execution.price_sol,
            effective_price_sol:
              detailPosition.effective_entry_price ??
              execution.effective_price_sol,
            size_sol: sizeSol,
            total_size_sol:
              detailPosition.total_size_sol ?? execution.total_size_sol,
            token_amount: detailPosition.token_amount ?? execution.token_amount,
            sol_delta:
              sizeSol != null ? -Math.abs(sizeSol) : execution.sol_delta,
            verified:
              detailPosition.transaction_entry_verified ?? execution.verified,
            fee_lamports:
              detailPosition.entry_fee_lamports ?? execution.fee_lamports,
            fee_sol: feeSol ?? execution.fee_sol,
          };
        }

        if ((execution.kind || "").toLowerCase() === "exit") {
          const feeSol =
            detailPosition.exit_fee_lamports != null
              ? lamportsToSolNumber(detailPosition.exit_fee_lamports)
              : execution.fee_sol;

          return {
            ...execution,
            timestamp: detailPosition.exit_time ?? execution.timestamp,
            price_sol: detailPosition.exit_price ?? execution.price_sol,
            effective_price_sol:
              detailPosition.effective_exit_price ??
              execution.effective_price_sol,
            sol_delta:
              detailPosition.sol_received ?? execution.sol_delta ?? null,
            verified:
              detailPosition.transaction_exit_verified ?? execution.verified,
            fee_lamports:
              detailPosition.exit_fee_lamports ?? execution.fee_lamports,
            fee_sol: feeSol ?? execution.fee_sol,
          };
        }

        return execution;
      });
    }
  }

  function refresh() {
    if (!state.currentKey) {
      return;
    }

    const detail = getActiveDetail();
    const storePosition = getCurrentPosition();

    if (detail && detail.position && storePosition) {
      mergeDetailWithStore(detail.position, storePosition, detail);
      renderModal(detail);
    } else if (!storePosition) {
      close();
      return;
    }

    const now = Date.now();
    if (now - (state.lastFetchedAt || 0) > REFETCH_INTERVAL_MS) {
      loadDetail(state.currentKey, { silent: true }).catch((err) => {
        console.warn("[PositionModal] refresh fetch failed:", err);
      });
    }
  }

  // ===== Action Handlers =====

  function getActiveMint() {
    const detail = getActiveDetail();
    if (detail?.position?.mint) return detail.position.mint;
    const storePosition = getCurrentPosition();
    return storePosition?.mint || null;
  }

  function copyMint() {
    const mint = getActiveMint();
    if (!mint || !Utils?.copyMint) return;
    Utils.copyMint(mint);
  }

  function openGMGN() {
    const mint = getActiveMint();
    if (!mint || !Utils?.openGMGN) return;
    Utils.openGMGN(mint);
  }

  function openDexScreener() {
    const mint = getActiveMint();
    if (!mint || !Utils?.openDexScreener) return;
    Utils.openDexScreener(mint);
  }

  function openSolscan() {
    const mint = getActiveMint();
    if (!mint || !Utils?.openSolscan) return;
    Utils.openSolscan(mint);
  }

  async function openTokenDetail() {
    const mint = getActiveMint();
    if (!mint) {
      Utils.showToast?.("❌ Mint not available", "error");
      return;
    }

    try {
      if (window.Router && typeof Router.loadPage === "function") {
        await Router.loadPage("tokens");
        scheduleTokenModalOpen(mint, 0);
      } else if (typeof window.openTokenModal === "function") {
        await window.openTokenModal(mint);
      } else if (Utils?.openDexScreener) {
        Utils.openDexScreener(mint);
      }
    } catch (err) {
      console.error("[PositionModal] openTokenDetail failed:", err);
      Utils.showToast?.("❌ Failed to open token details", "error");
    }
  }

  function scheduleTokenModalOpen(mint, attempt) {
    if (typeof window.openTokenModal === "function") {
      window.openTokenModal(mint).catch((err) => {
        console.error("[PositionModal] openTokenModal error:", err);
        Utils.showToast?.("❌ Failed to open token details", "error");
      });
      return;
    }
    if (attempt >= 20) {
      Utils.showToast?.("❌ Token detail module unavailable", "error");
      return;
    }
    setTimeout(() => scheduleTokenModalOpen(mint, attempt + 1), 120);
  }

  function copyDebug() {
    const mint = getActiveMint();
    if (!mint || !Utils?.copyDebugInfo) return;
    Utils.copyDebugInfo(mint, "position");
  }

  function copySignature(signature) {
    if (!signature) return;

    let copyPromise = null;
    if (Utils?.copyToClipboard) {
      copyPromise = Utils.copyToClipboard(signature);
    } else if (navigator?.clipboard?.writeText) {
      copyPromise = navigator.clipboard.writeText(signature);
    }

    if (!copyPromise || typeof copyPromise.then !== "function") {
      console.warn("[PositionModal] Clipboard copy function not available");
      return;
    }

    copyPromise
      .then(() => {
        Utils?.showToast?.("✅ Signature copied", "success");
      })
      .catch((err) => {
        console.error("[PositionModal] copySignature failed:", err);
        Utils?.showToast?.("❌ Failed to copy signature", "error");
      });
  }

  // ===== Event Listeners =====

  document.addEventListener("click", (event) => {
    const button = event?.target?.closest?.(".position-signature-button");
    if (!button) {
      return;
    }
    const signature = button.getAttribute("data-signature");
    if (signature) {
      copySignature(signature);
    }
  });

  // ===== Public API =====

  const PositionModal = {
    open,
    close,
    refresh,
    copyMint,
    openGMGN,
    openDexScreener,
    openSolscan,
    openTokenDetail,
    copyDebug,
    copySignature,
  };

  // Export to window
  window.PositionModal = PositionModal;

  console.log("[PositionModal] Module loaded");
})();
