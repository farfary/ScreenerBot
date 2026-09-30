/**
 * Summary rail for the Position Details dialog.
 *
 * The header carries the four headline figures; the rail is the ledger behind them — what was
 * bought and sold and when, the price path the position lived through, what it cost, the
 * token's risk and the market it trades in. Flat sections separated by rules, never cards.
 */
import * as Utils from "../../core/utils.js";
import { formatPercentValue, withAgo } from "../../core/format.js";
import { closeReasonText } from "../trade_reason.js";

// Labels, sub-lines and titles are plain text and escaped here; values are HTML.
const fact = (label, value, { sub = "", tone = "", title = "" } = {}) => `
  <div class="pdd-fact"${title ? ` title="${Utils.escapeHtml(title)}"` : ""}>
    <dt>${Utils.escapeHtml(label)}</dt>
    <dd><span class="pdd-fact-value ${tone}">${value}</span>${sub ? `<span class="pdd-fact-sub">${Utils.escapeHtml(sub)}</span>` : ""}</dd>
  </div>`;

const facts = (rows) => {
  const html = rows.filter(Boolean).join("");
  return html ? `<dl class="pdd-facts">${html}</dl>` : "";
};

const section = (title, body) =>
  body ? `<section class="pdd-section"><h3 class="pdd-section-title">${Utils.escapeHtml(title)}</h3>${body}</section>` : "";

const closeReasonLabel = (reason) => Utils.escapeHtml(closeReasonText(reason));

const isWebUrl = (url) => typeof url === "string" && /^https?:\/\//i.test(url);

export function applySummaryMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  proto._renderSummary = function () {
    const pos = this.fullDetails?.position;
    if (!pos) return;

    this._paintRegion(
      "#pddSummary",
      [
        this._buildPositionSection(pos),
        this._buildPricePathSection(pos),
        this._buildCostsSection(pos),
        this._buildRiskSection(),
        this._buildMarketSection(),
        this._buildLinksSection(),
      ].join("")
    );
  };

  proto._buildPositionSection = function (pos) {
    const exits = this.fullDetails?.exits || [];
    const settled = this._isSettled();
    const symbol = Utils.escapeHtml(pos.symbol || I18n.t("positions-fact-tokens-fallback"));
    const remaining = pos.remaining_token_amount || 0;
    const exited = pos.total_exited_amount || 0;
    // Tokens ever acquired = still held + already sold. NOT `token_amount`, which is only the
    // entry buy and never grows on a DCA.
    const bought = remaining + exited;
    const adds = pos.dca_count || 0;
    const partials = pos.partial_exit_count || 0;
    const tokens = (raw) => `${Utils.formatCompactNumber(this._toUiAmount(raw))} ${symbol}`;
    const shareOfBought = (raw) =>
      bought > 0
        ? I18n.t("positions-fact-share-of-bought", {
            percent: formatPercentValue((raw / bought) * 100, { decimals: 1, includeSign: false }),
          })
        : "";
    const returned = exits.reduce((sum, exit) => sum + (exit.sol_received || 0), 0);
    const when = (ts) => Utils.formatTimestamp(ts, { includeSeconds: false });
    const age = (seconds) => Utils.formatUptime(Math.max(0, seconds), { style: "compact" });

    const rows = [
      fact(I18n.t("positions-fact-bought"), bought ? tokens(bought) : "—", {
        sub: I18n.t("positions-fact-entry-count", { count: adds }),
      }),
    ];

    // Until something is sold, Holding would only repeat Bought.
    if (!settled && exited > 0) {
      rows.push(
        fact(I18n.t("positions-fact-holding"), tokens(remaining), { sub: shareOfBought(remaining) })
      );
    }
    if (exited > 0) {
      rows.push(
        fact(I18n.t("positions-fact-sold"), tokens(exited), {
          // SOL back is summed from exit records; without them it would print an invented 0.
          sub:
            settled || !exits.length
              ? shareOfBought(exited)
              : I18n.t("positions-fact-partial-exits-back", {
                  count: partials,
                  returned: this._formatSol(returned),
                }),
        })
      );
      // The header already prints the P&L when the booked figure reads the same. Compared as
      // printed: the two are computed separately and differ in the eleventh decimal.
      if (!settled && pos.pnl != null && this._formatSol(pos.pnl) !== this._formatSol(pos.unrealized_pnl)) {
        rows.push(
          fact(I18n.t("positions-fact-realized"), this._formatSol(pos.pnl, { sign: true }), {
            sub: pos.pnl_percent != null ? this._formatPct(pos.pnl_percent) : "",
            tone: this._toneClass(pos.pnl),
          })
        );
      }
    }

    const heldSeconds = Date.now() / 1000 - pos.entry_time;
    rows.push(
      fact(I18n.t("positions-fact-opened"), when(pos.entry_time), {
        sub: settled ? "" : withAgo(age(heldSeconds), heldSeconds),
      })
    );
    if (settled && pos.exit_time) {
      rows.push(
        fact(I18n.t("positions-fact-closed"), when(pos.exit_time), {
          sub: I18n.t("positions-fact-held", { age: age(pos.exit_time - pos.entry_time) }),
        })
      );
    }
    if (settled && pos.closed_reason) {
      rows.push(fact(I18n.t("positions-fact-reason"), closeReasonLabel(pos.closed_reason)));
    }
    if (pos.status === "archived" && pos.archived_at) {
      rows.push(fact(I18n.t("positions-fact-archived"), when(pos.archived_at)));
    }

    const verified = settled ? pos.transaction_exit_verified : pos.transaction_entry_verified;
    rows.push(
      fact(
        settled ? I18n.t("positions-fact-exit") : I18n.t("positions-fact-entry"),
        Utils.escapeHtml(
          verified ? I18n.t("positions-fact-verified") : I18n.t("positions-fact-confirming")
        ),
        { tone: verified ? "pdd-positive" : "pdd-caution" }
      )
    );

    return section(I18n.t("positions-summary-position"), facts(rows));
  };

  /**
   * Where the entry and the current (or exit) price sit inside the range the position lived
   * through, drawn to scale, followed by the numbers behind it.
   */
  proto._buildPricePathSection = function (pos) {
    const entry = pos.average_entry_price || pos.entry_price;
    const peak = pos.price_highest;
    const low = pos.price_lowest;
    if (!entry || !peak || !low) return "";

    const settled = this._isSettled();
    const mark = settled ? pos.average_exit_price || pos.exit_price : pos.current_price;
    const vsEntry = (price) =>
      I18n.t("positions-fact-vs-entry", {
        percent: this._formatPct(((price - entry) / entry) * 100, 1),
      });
    const fromPeak = mark ? ((mark - peak) / peak) * 100 : null;

    const prices = (this.fullDetails?.entries || []).map((e) => e.price).filter((p) => p > 0);
    const minEntry = prices.length > 1 ? Math.min(...prices) : null;
    const maxEntry = prices.length > 1 ? Math.max(...prices) : null;

    const rows = [
      fact(I18n.t("positions-range-peak"), `${this._formatPrice(peak)} SOL`, { sub: vsEntry(peak) }),
      fact(I18n.t("positions-range-low"), `${this._formatPrice(low)} SOL`, { sub: vsEntry(low) }),
      fromPeak !== null
        ? fact(
            settled ? I18n.t("positions-fact-exit-vs-peak") : I18n.t("positions-fact-now-vs-peak"),
            this._formatPct(fromPeak, 1),
            { tone: this._toneClass(fromPeak) }
          )
        : "",
      minEntry !== null && minEntry !== maxEntry
        ? fact(
            I18n.t("positions-fact-entry-range"),
            `${this._formatPrice(minEntry)} – ${this._formatPrice(maxEntry)}`
          )
        : "",
    ];

    return section(
      I18n.t("positions-summary-price-path"),
      this._buildRangeBar({ low, peak, entry, mark, settled }) + facts(rows)
    );
  };

  proto._buildRangeBar = function ({ low, peak, entry, mark, settled }) {
    const lo = Math.min(low, entry, mark || entry);
    const hi = Math.max(peak, entry, mark || entry);
    if (!(hi > lo)) return "";

    const at = (price) =>
      formatPercentValue(((price - lo) / (hi - lo)) * 100, { decimals: 2, plus: "" });
    const tone = mark && mark < entry ? "is-down" : "is-up";
    const span = mark
      ? `<span class="pdd-range-span ${tone}" style="--from: ${at(Math.min(entry, mark))}; --to: ${at(Math.max(entry, mark))}"></span>`
      : "";

    return `
      <div class="pdd-range" role="img" aria-label="${Utils.escapeHtml(settled ? I18n.t("positions-range-label-exit") : I18n.t("positions-range-label-now"))}">
        <div class="pdd-range-track">
          ${span}
          <span class="pdd-range-tick is-entry" style="--at: ${at(entry)}"></span>
          ${mark ? `<span class="pdd-range-tick ${tone}" style="--at: ${at(mark)}"></span>` : ""}
        </div>
        <div class="pdd-range-scale">
          <span>${Utils.escapeHtml(I18n.t("positions-range-low"))}</span>
          <span class="pdd-range-key"><span class="is-entry">${Utils.escapeHtml(I18n.t("positions-fact-entry"))}</span>${mark ? `<span class="${tone}">${Utils.escapeHtml(settled ? I18n.t("positions-fact-exit") : I18n.t("positions-range-now"))}</span>` : ""}</span>
          <span>${Utils.escapeHtml(I18n.t("positions-range-peak"))}</span>
        </div>
      </div>`;
  };

  proto._buildCostsSection = function (pos) {
    // The position's own fee fields cover only the entry and the final close; every DCA add
    // and partial exit carries its fee on its RECORD, reported in SOL (`fees_sol`).
    const recordFees = (records) => records.reduce((sum, r) => sum + (r.fees_sol || 0), 0);
    const entryFees =
      recordFees(this.fullDetails?.entries || []) || this._lamportsToSol(pos.entry_fee_lamports);
    const exitFees =
      recordFees(this.fullDetails?.exits || []) || this._lamportsToSol(pos.exit_fee_lamports);
    const total = entryFees + exitFees;
    if (!(total > 0)) return "";

    const invested = pos.total_size_sol || 0;
    const share =
      invested > 0
        ? I18n.t("positions-fact-share-of-invested", {
            percent: formatPercentValue((total / invested) * 100, {
              decimals: 2,
              includeSign: false,
            }),
          })
        : "";
    // A total of one fee only repeats it, so the share moves onto that fee instead.
    const both = entryFees > 0 && exitFees > 0;
    return section(
      I18n.t("positions-summary-network-fees"),
      facts([
        entryFees > 0
          ? fact(I18n.t("positions-fact-entry"), this._formatSol(entryFees), { sub: both ? "" : share })
          : "",
        exitFees > 0
          ? fact(I18n.t("positions-fact-exit"), this._formatSol(exitFees), { sub: both ? "" : share })
          : "",
        both ? fact(I18n.t("positions-fact-total"), this._formatSol(total), { sub: share }) : "",
      ])
    );
  };

  /** Why the header's risk badge reads the way it does. The score itself stays in the badge. */
  proto._buildRiskSection = function () {
    const security = this.fullDetails?.security;
    if (!security) return "";

    const rows = facts([
      security.has_mint_authority
        ? fact(I18n.t("positions-fact-mint-authority"), Utils.escapeHtml(I18n.t("positions-fact-active")), {
            tone: "pdd-negative",
          })
        : "",
      security.has_freeze_authority
        ? fact(I18n.t("positions-fact-freeze-authority"), Utils.escapeHtml(I18n.t("positions-fact-active")), {
            tone: "pdd-negative",
          })
        : "",
    ]);
    const risks = (security.top_risks || [])
      .map((risk) => `<li>${Utils.escapeHtml(risk)}</li>`)
      .join("");

    return section(I18n.t("positions-summary-risk"), rows + (risks ? `<ul class="pdd-risk-list">${risks}</ul>` : ""));
  };

  proto._buildMarketSection = function () {
    const market = this.fullDetails?.market_data;
    const pool = this.fullDetails?.pool_info;
    const usd = (value) => (value ? Utils.formatCurrencyUSD(value) : "—");
    const rows = [];

    if (pool?.dex_name || pool?.liquidity_sol != null) {
      rows.push(
        fact(I18n.t("positions-fact-pool"), Utils.escapeHtml(pool.dex_name || "—"), {
          sub:
            pool.liquidity_sol != null
              ? I18n.t("positions-fact-pool-liquidity", {
                  amount: Utils.formatCompactNumber(pool.liquidity_sol),
                })
              : "",
        })
      );
    }

    if (market) {
      rows.push(
        fact(I18n.t("positions-fact-market-cap"), usd(market.market_cap), {
          sub:
            market.fdv && market.fdv !== market.market_cap
              ? I18n.t("positions-fact-fdv", { value: usd(market.fdv) })
              : "",
        })
      );
      rows.push(fact(I18n.t("positions-fact-liquidity"), usd(market.liquidity_usd)));
      rows.push(fact(I18n.t("positions-fact-volume-24h"), usd(market.volume_24h)));

      const changes = [
        [I18n.t("positions-change-period-1h"), market.price_change_h1],
        [I18n.t("positions-change-period-24h"), market.price_change_h24],
      ].filter(([, value]) => value != null);
      if (changes.length) {
        rows.push(
          fact(
            I18n.t("positions-fact-price-change"),
            changes
              .map(
                ([period, value]) =>
                  `<span class="pdd-change">${Utils.escapeHtml(period)} <span class="${this._toneClass(value)}">${this._formatPct(value, 1)}</span></span>`
              )
              .join("")
          )
        );
      }
      rows.push(
        fact(
          I18n.t("positions-fact-holders"),
          market.holder_count ? Utils.formatCompactNumber(market.holder_count) : "—"
        )
      );
    }

    // A finished position is read long after it closed: say these are today's numbers.
    return section(
      this._isSettled() ? I18n.t("positions-summary-market-now") : I18n.t("positions-summary-market"),
      facts(rows)
    );
  };

  /** Outside references. Solscan already has its own control in the header. */
  proto._buildLinksSection = function () {
    const tokenInfo = this.fullDetails?.token_info;
    const links = this.fullDetails?.external_links || {};
    const items = [
      [I18n.t("positions-link-website"), tokenInfo?.website],
      [I18n.t("positions-link-x"), tokenInfo?.twitter],
      [I18n.t("positions-link-telegram"), tokenInfo?.telegram],
      [I18n.t("positions-link-dexscreener"), links.dexscreener],
      [I18n.t("positions-link-birdeye"), links.birdeye],
      [I18n.t("positions-link-rugcheck"), links.rugcheck],
      [I18n.t("positions-link-photon"), links.photon],
    ].filter(([, url]) => isWebUrl(url));
    if (!items.length) return "";

    return section(
      I18n.t("positions-summary-links"),
      `<div class="pdd-links">${items
        .map(
          ([label, url]) =>
            `<a class="pdd-link" href="${Utils.escapeHtml(url)}" target="_blank" rel="noopener">${Utils.escapeHtml(label)}<i class="icon-arrow-up-right"></i></a>`
        )
        .join("")}</div>`
    );
  };
}
