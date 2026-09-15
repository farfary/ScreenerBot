/**
 * Panes of the Position Details dialog.
 *
 * The chart and the activity share the main column as two fixed panes, never as one scrolling
 * page. The chart owns the mouse wheel and the trackpad (zoom and pan), so a column scrolling
 * underneath it could only be scrolled with the pointer somewhere else. Here every pane keeps
 * its own gesture: the wheel zooms the chart, scrolls the activity list, scrolls the rail.
 *
 * Modes, remembered across opens:
 *   balanced — chart over activity
 *   custom   — the same, at the share the divider was dragged to
 *   chart    — the activity folds to its header strip
 *   activity — the chart folds to a glance strip (no axes, no wheel); clicking it restores
 *
 * The panes are linked: hovering an activity event puts the chart crosshair on its candle, and
 * clicking a position marker on the chart finds that event in the activity list.
 */
import * as AppState from "../../core/app_state.js";

const SPLIT_STATE_KEY = "positionDetailsSplit";
const BALANCED_SHARE = 0.62;
// A drag released this close to either end snaps to that end's focus mode.
const SNAP_STRIP_PX = 48;
const SNAP_CHART_SHARE = 0.94;
const SNAP_BALANCED = 0.03;
const KEY_STEP = 0.05;
// Movement that turns a press on the header strip from a click into a drag.
const DRAG_THRESHOLD_PX = 4;
// No room for the rail beside the panes: it moves to the top of the activity pane's scroll.
// Must match the breakpoint in styles/ui/position_details/base.css.
const STACK_QUERY = "(width <= 1100px)";
const LOCATE_FLASH_MS = 1600;
// Slightly longer than the pane height transition in chart.css, so a scroll measured after an
// unfold reads the final geometry.
const PANE_SETTLE_MS = 220;

const MODES = new Set(["balanced", "custom", "chart", "activity"]);
const isFocus = (mode) => mode === "chart" || mode === "activity";
const clampShare = (share) => Math.min(1, Math.max(0, share));

const setControl = (button, label, icon) => {
  button.title = label;
  button.setAttribute("aria-label", label);
  const glyph = button.querySelector("i");
  if (glyph) glyph.className = icon;
};

export function applyPanesMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  /** Restore the remembered mode and wire the panes. Called once per dialog element. */
  proto._initPanes = function () {
    const saved = AppState.load(SPLIT_STATE_KEY, null);
    this._split = {
      mode: MODES.has(saved?.mode) ? saved.mode : "balanced",
      share: Number.isFinite(saved?.share) ? clampShare(saved.share) : BALANCED_SHARE,
    };
    this._splitReturn = null;
    this._splitDragged = false;
    this._previewCard = null;
    this._applySplit();
    this._bindPaneHandlers();

    this._stackQuery = window.matchMedia(STACK_QUERY);
    this._stackListener = () => this._syncStack();
    this._stackQuery.addEventListener("change", this._stackListener);
    this._syncStack();
  };

  proto._teardownPanes = function () {
    this._stackQuery?.removeEventListener("change", this._stackListener);
    this._stackQuery = null;
    this._stackListener = null;
    clearTimeout(this._locateTimer);
    this._locateTimer = null;
    this._previewCard = null;
  };

  /** Chart share of the column the current mode stands for, 0 (folded) to 1 (full). */
  proto._chartShare = function () {
    const { mode, share } = this._split;
    if (mode === "chart") return 1;
    if (mode === "activity") return 0;
    return mode === "custom" ? share : BALANCED_SHARE;
  };

  proto._applySplit = function () {
    const main = this.dialogEl?.querySelector(".pdd-main");
    if (!main) return;
    const { mode, share } = this._split;
    main.dataset.split = mode;
    main.style.setProperty("--pdd-chart-share", String(mode === "custom" ? share : BALANCED_SHARE));
    this._syncPaneControls();
  };

  /** Controls and chart follow the mode. Also called once the chart exists (chart.js). */
  proto._syncPaneControls = function () {
    const mode = this._split?.mode;
    if (!this.dialogEl || !mode) return;
    this._pddChart?.setCompact(mode === "activity");
    const chart = this.dialogEl.querySelector("#pddChartSection");
    if (chart) chart.title = mode === "activity" ? "Show chart" : "";

    const toggle = this.dialogEl.querySelector("#pddActivityToggle");
    if (toggle) {
      const label =
        mode === "activity" ? "Show chart" : mode === "chart" ? "Show activity" : "Expand activity";
      setControl(toggle, label, mode === "activity" ? "icon-chevron-down" : "icon-chevron-up");
      toggle.setAttribute("aria-expanded", String(mode !== "chart"));
    }

    const focus = this.dialogEl.querySelector("#pddChartFocusBtn");
    if (focus) {
      const full = mode === "chart";
      setControl(
        focus,
        full ? "Restore activity" : "Expand chart",
        full ? "icon-minimize-2" : "icon-maximize-2"
      );
      focus.classList.toggle("active", full);
      focus.setAttribute("aria-pressed", String(full));
    }

    this.dialogEl
      .querySelector("#pddSplitHandle")
      ?.setAttribute("aria-valuenow", String(Math.round(this._chartShare() * 100)));
  };

  /** Leaving balanced or custom for a focus remembers where to come back to. */
  proto._setSplit = function (mode, share = this._split.share) {
    const current = this._split;
    if (mode === current.mode && (mode !== "custom" || share === current.share)) return;
    if (isFocus(mode) && !isFocus(current.mode)) this._splitReturn = { ...current };
    this._split = { mode, share: mode === "custom" ? clampShare(share) : current.share };
    this._applySplit();
    AppState.save(SPLIT_STATE_KEY, this._split);
  };

  proto._restoreSplit = function () {
    const back = this._splitReturn || { mode: "balanced", share: this._split.share };
    this._splitReturn = null;
    this._setSplit(back.mode, back.share);
  };

  proto._toggleActivityFocus = function () {
    if (isFocus(this._split.mode)) this._restoreSplit();
    else this._setSplit("activity");
  };

  proto._toggleChartFocus = function () {
    if (this._split.mode === "chart") this._restoreSplit();
    else this._setSplit("chart");
  };

  proto._bindPaneHandlers = function () {
    const main = this.dialogEl?.querySelector(".pdd-main");
    const head = this.dialogEl?.querySelector("#pddActivityHead");
    const body = this.dialogEl?.querySelector("#pddActivityBody");
    if (!main || !head || !body) return;

    // The chart toolbar is rebuilt per chart, so its button is reached by delegation.
    main.addEventListener("click", (event) => {
      if (event.target.closest("#pddChartFocusBtn")) this._toggleChartFocus();
    });

    // The header strip is the activity pane's handle: a click anywhere on it that is not one of
    // its own controls toggles the pane, exactly like the chevron.
    head.addEventListener("click", (event) => {
      if (this._splitDragged) {
        this._splitDragged = false;
        return;
      }
      if (event.target.closest("#pddActivityToggle")) {
        this._toggleActivityFocus();
        return;
      }
      if (event.target.closest("button, a, [role='separator']")) return;
      this._toggleActivityFocus();
    });
    head.addEventListener("pointerdown", (event) => this._startSplitDrag(event));
    head.addEventListener("dblclick", (event) => {
      if (event.target.closest("#pddSplitHandle")) this._setSplit("balanced");
    });
    this.dialogEl
      .querySelector("#pddSplitHandle")
      ?.addEventListener("keydown", (event) => this._onSplitKey(event));

    body.addEventListener("pointerover", (event) => this._previewActivityEvent(event));
    body.addEventListener("pointerout", (event) => this._endActivityPreview(event));
  };

  /**
   * Drag the divider (the header's top edge) or the header strip itself. A press on the strip
   * becomes a drag only after it moves, so a plain click still toggles the pane.
   */
  proto._startSplitDrag = function (event) {
    if (event.button !== 0) return;
    const onHandle = Boolean(event.target.closest("#pddSplitHandle"));
    if (!onHandle && event.target.closest("button, a")) return;

    const head = event.currentTarget;
    const main = this.dialogEl?.querySelector(".pdd-main");
    const chart = this.dialogEl?.querySelector("#pddChartSection");
    // A token without candles has a fixed notice, not a pane worth sizing.
    if (!main || !chart || chart.classList.contains("is-empty")) return;

    const startY = event.clientY;
    const startPx = chart.getBoundingClientRect().height;
    const usable = main.clientHeight - head.offsetHeight;
    const strip = parseFloat(getComputedStyle(main).getPropertyValue("--pdd-chart-strip")) || 0;
    if (usable <= strip) return;

    let dragging = false;
    let share = startPx / usable;
    this._splitDragged = false;
    head.setPointerCapture(event.pointerId);

    const begin = () => {
      dragging = true;
      main.classList.add("is-resizing");
      this._pddChart?.setCompact(false);
    };
    if (onHandle) begin();

    const move = (e) => {
      const delta = e.clientY - startY;
      if (!dragging) {
        if (Math.abs(delta) < DRAG_THRESHOLD_PX) return;
        begin();
      }
      share = Math.min(usable, Math.max(strip, startPx + delta)) / usable;
      main.dataset.split = "custom";
      main.style.setProperty("--pdd-chart-share", String(share));
    };

    const end = (e) => {
      head.removeEventListener("pointermove", move);
      head.removeEventListener("pointerup", end);
      head.removeEventListener("pointercancel", end);
      if (head.hasPointerCapture(e.pointerId)) head.releasePointerCapture(e.pointerId);
      if (!dragging) return;

      main.classList.remove("is-resizing");
      // The click that follows a drag is not a toggle.
      this._splitDragged = true;
      if (share * usable <= strip + SNAP_STRIP_PX) this._setSplit("activity");
      else if (share >= SNAP_CHART_SHARE) this._setSplit("chart");
      else if (Math.abs(share - BALANCED_SHARE) <= SNAP_BALANCED) this._setSplit("balanced");
      else this._setSplit("custom", share);
      // _setSplit returns early when the mode did not change; the drag already rewrote the DOM.
      this._applySplit();
    };

    head.addEventListener("pointermove", move);
    head.addEventListener("pointerup", end);
    head.addEventListener("pointercancel", end);
  };

  proto._onSplitKey = function (event) {
    const step = { ArrowUp: -KEY_STEP, ArrowDown: KEY_STEP }[event.key];
    if (step !== undefined) {
      const next = clampShare(this._chartShare() + step);
      if (next <= 0) this._setSplit("activity");
      else if (next >= 1) this._setSplit("chart");
      else this._setSplit("custom", next);
    } else if (event.key === "Home") {
      this._setSplit("activity");
    } else if (event.key === "End") {
      this._setSplit("chart");
    } else if (event.key === "Enter") {
      this._setSplit("balanced");
    } else {
      return;
    }
    event.preventDefault();
  };

  /** Narrow windows: the rail moves to the top of the activity pane's scroll, and back. */
  proto._syncStack = function () {
    const rail = this.dialogEl?.querySelector("#pddSummary");
    const body = this.dialogEl?.querySelector("#pddActivityBody");
    const layout = this.dialogEl?.querySelector(".pdd-layout");
    if (!rail || !body || !layout) return;

    const stacked = Boolean(this._stackQuery?.matches);
    if (stacked && rail.parentElement !== body) body.prepend(rail);
    else if (!stacked && rail.parentElement !== layout) layout.append(rail);
  };

  // ===========================================================================
  // LINKING
  // ===========================================================================

  /** Hovering an event shows where it sits on the chart. */
  proto._previewActivityEvent = function (event) {
    const card = event.target.closest(".pdd-act-card[data-ts]");
    if (!card || card === this._previewCard) return;
    this._previewCard = card;

    const bar = this._barForTimestamp(Number(card.dataset.ts));
    if (!bar) {
      this._pddChart?.clearCrosshair();
      return;
    }
    const price = Number(card.dataset.price);
    // The crosshair emits its usual move, so the tooltip and the O/H/L/C readout follow too.
    this._pddChart?.showCrosshairAt(bar.time, price > 0 ? price : bar.close);
  };

  proto._endActivityPreview = function (event) {
    const card = this._previewCard;
    if (!card || card.contains(event.relatedTarget)) return;
    this._previewCard = null;
    this._pddChart?.clearCrosshair();
  };

  /** A click on the chart: the glance strip restores; a marked candle finds its event. */
  proto._onPositionChartClick = function (param) {
    if (this._split?.mode === "activity") {
      this._restoreSplit();
      return;
    }
    if (param?.time === undefined || !this._pddMarkerBars?.has(param.time)) return;
    this._locateActivityAt(param.time);
  };

  proto._locateActivityAt = function (barTime) {
    const body = this.dialogEl?.querySelector("#pddActivityBody");
    if (!body) return;

    const cards = [...body.querySelectorAll(".pdd-act-card[data-ts]")].filter(
      (card) => this._barForTimestamp(Number(card.dataset.ts))?.time === barTime
    );
    // Markers are this position's events, so its own round wins over an older one.
    const card =
      cards.find((c) => c.closest(".pdd-act-round.is-current, .pdd-act-round.is-single")) ||
      cards[0];
    if (!card) return;

    const folded = this._split.mode === "chart";
    if (folded) this._restoreSplit();
    if (card.classList.contains("is-filtered-out")) this._setActivityFilter("all");
    const round = card.closest(".pdd-act-round");
    if (round && !round.classList.contains("is-open")) this._setRoundOpen(round, true);

    clearTimeout(this._locateTimer);
    body.querySelectorAll(".pdd-act-card.is-located").forEach((el) => {
      el.classList.remove("is-located");
    });

    // Scroll the pane itself: scrollIntoView would also scroll the overflow-hidden ancestors
    // that hold the dialog's layout together.
    const reveal = () => {
      const offset = card.getBoundingClientRect().top - body.getBoundingClientRect().top;
      const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      body.scrollTo({
        top: Math.max(0, body.scrollTop + offset - body.clientHeight * 0.25),
        behavior: reduce ? "auto" : "smooth",
      });
      card.classList.add("is-located");
      this._locateTimer = setTimeout(() => card.classList.remove("is-located"), LOCATE_FLASH_MS);
    };
    if (folded) this._locateTimer = setTimeout(reveal, PANE_SETTLE_MS);
    else reveal();
  };
}
