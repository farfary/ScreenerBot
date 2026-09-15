/**
 * Summary rail for the Position Details dialog.
 *
 * The header carries the four headline figures; the rail is the ledger behind them — what was
 * bought and sold and when, the price path the position lived through, what it cost, the
 * token's risk and the market it trades in. Flat sections separated by rules, never cards.
 */
import * as Utils from "../../core/utils.js";

const fact = (label, value, { sub = "", tone = "", title = "" } = {}) => `
  <div class="pdd-fact"${title ? ` title="${title}"` : ""}>
    <dt>${label}</dt>
    <dd><span class="pdd-fact-value ${tone}">${value}</span>${sub ? `<span class="pdd-fact-sub">${sub}</span>` : ""}</dd>
  </div>`;

const facts = (rows) => {
  const html = rows.filter(Boolean).join("");
  return html ? `<dl class="pdd-facts">${html}</dl>` : "";
};

const section = (title, body) =>
  body ? `<section class="pdd-section"><h3 class="pdd-section-title">${title}</h3>${body}</section>` : "";

const humanize = (value) => {
  const text = String(value)
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/_/g, " ")
    .toLowerCase();
  return Utils.escapeHtml(text.charAt(0).toUpperCase() + text.slice(1));
};

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
    const symbol = Utils.escapeHtml(pos.symbol || "tokens");
    const remaining = pos.remaining_token_amount || 0;
    const exited = pos.total_exited_amount || 0;
    // Tokens ever acquired = still held + already sold. NOT `token_amount`, which is only the
    // entry buy and never grows on a DCA.
    const bought = remaining + exited;
    const adds = pos.dca_count || 0;
    const partials = pos.partial_exit_count || 0;
    const tokens = (raw) => `${Utils.formatCompactNumber(this._toUiAmount(raw))} ${symbol}`;
    const shareOfBought = (raw) =>
      bought > 0 ? `${Utils.formatNumber((raw / bought) * 100, 1)}% of bought` : "";
    const returned = exits.reduce((sum, exit) => sum + (exit.sol_received || 0), 0);
    const when = (ts) => Utils.formatTimestamp(ts, { includeSeconds: false });
    const age = (seconds) => Utils.formatUptime(Math.max(0, seconds), { style: "compact" });

    const rows = [
      fact("Bought", bought ? tokens(bought) : "—", {
        sub: adds > 0 ? `1 entry + ${this._plural(adds, "add")}` : "1 entry",
      }),
    ];

    // Until something is sold, Holding would only repeat Bought.
    if (!settled && exited > 0) {
      rows.push(fact("Holding", tokens(remaining), { sub: shareOfBought(remaining) }));
    }
    if (exited > 0) {
      rows.push(
        fact("Sold", tokens(exited), {
          // SOL back is summed from exit records; without them it would print an invented 0.
          sub:
            settled || !exits.length
              ? shareOfBought(exited)
              : `${this._plural(partials, "partial exit")} · ${this._formatSol(returned)} back`,
        })
      );
      // The header already prints the P&L when the booked figure reads the same. Compared as
      // printed: the two are computed separately and differ in the eleventh decimal.
      if (!settled && pos.pnl != null && this._formatSol(pos.pnl) !== this._formatSol(pos.unrealized_pnl)) {
        rows.push(
          fact("Realized", this._formatSol(pos.pnl, { sign: true }), {
            sub: pos.pnl_percent != null ? this._formatPct(pos.pnl_percent) : "",
            tone: this._toneClass(pos.pnl),
          })
        );
      }
    }

    rows.push(
      fact("Opened", when(pos.entry_time), {
        sub: settled ? "" : `${age(Date.now() / 1000 - pos.entry_time)} ago`,
      })
    );
    if (settled && pos.exit_time) {
      rows.push(fact("Closed", when(pos.exit_time), { sub: `held ${age(pos.exit_time - pos.entry_time)}` }));
    }
    if (settled && pos.closed_reason) rows.push(fact("Reason", humanize(pos.closed_reason)));
    if (pos.status === "archived" && pos.archived_at) rows.push(fact("Archived", when(pos.archived_at)));

    const verified = settled ? pos.transaction_exit_verified : pos.transaction_entry_verified;
    rows.push(
      fact(settled ? "Exit" : "Entry", verified ? "Verified on chain" : "Confirming", {
        tone: verified ? "pdd-positive" : "pdd-caution",
      })
    );

    return section("Position", facts(rows));
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
    const markLabel = settled ? "Exit" : "Now";
    const vsEntry = (price) => `${this._formatPct(((price - entry) / entry) * 100, 1)} vs entry`;
    const fromPeak = mark ? ((mark - peak) / peak) * 100 : null;

    const prices = (this.fullDetails?.entries || []).map((e) => e.price).filter((p) => p > 0);
    const minEntry = prices.length > 1 ? Math.min(...prices) : null;
    const maxEntry = prices.length > 1 ? Math.max(...prices) : null;

    const rows = [
      fact("Peak", `${this._formatPrice(peak)} SOL`, { sub: vsEntry(peak) }),
      fact("Low", `${this._formatPrice(low)} SOL`, { sub: vsEntry(low) }),
      fromPeak !== null
        ? fact(`${markLabel} vs peak`, this._formatPct(fromPeak, 1), { tone: this._toneClass(fromPeak) })
        : "",
      minEntry !== null && minEntry !== maxEntry
        ? fact("Entry range", `${this._formatPrice(minEntry)} – ${this._formatPrice(maxEntry)}`)
        : "",
    ];

    return section("Price path", this._buildRangeBar({ low, peak, entry, mark, markLabel }) + facts(rows));
  };

  proto._buildRangeBar = function ({ low, peak, entry, mark, markLabel }) {
    const lo = Math.min(low, entry, mark || entry);
    const hi = Math.max(peak, entry, mark || entry);
    if (!(hi > lo)) return "";

    const at = (price) => `${(((price - lo) / (hi - lo)) * 100).toFixed(2)}%`;
    const tone = mark && mark < entry ? "is-down" : "is-up";
    const span = mark
      ? `<span class="pdd-range-span ${tone}" style="--from: ${at(Math.min(entry, mark))}; --to: ${at(Math.max(entry, mark))}"></span>`
      : "";

    return `
      <div class="pdd-range" role="img" aria-label="Entry and ${markLabel.toLowerCase()} price between the low and the peak">
        <div class="pdd-range-track">
          ${span}
          <span class="pdd-range-tick is-entry" style="--at: ${at(entry)}"></span>
          ${mark ? `<span class="pdd-range-tick ${tone}" style="--at: ${at(mark)}"></span>` : ""}
        </div>
        <div class="pdd-range-scale">
          <span>Low</span>
          <span class="pdd-range-key"><span class="is-entry">Entry</span>${mark ? `<span class="${tone}">${markLabel}</span>` : ""}</span>
          <span>Peak</span>
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
    const share = invested > 0 ? `${Utils.formatNumber((total / invested) * 100, 2)}% of invested` : "";
    // A total of one fee only repeats it, so the share moves onto that fee instead.
    const both = entryFees > 0 && exitFees > 0;
    return section(
      "Network fees",
      facts([
        entryFees > 0 ? fact("Entry", this._formatSol(entryFees), { sub: both ? "" : share }) : "",
        exitFees > 0 ? fact("Exit", this._formatSol(exitFees), { sub: both ? "" : share }) : "",
        both ? fact("Total", this._formatSol(total), { sub: share }) : "",
      ])
    );
  };

  /** Why the header's risk badge reads the way it does. The score itself stays in the badge. */
  proto._buildRiskSection = function () {
    const security = this.fullDetails?.security;
    if (!security) return "";

    const rows = facts([
      security.has_mint_authority ? fact("Mint authority", "Active", { tone: "pdd-negative" }) : "",
      security.has_freeze_authority
        ? fact("Freeze authority", "Active", { tone: "pdd-negative" })
        : "",
    ]);
    const risks = (security.top_risks || [])
      .map((risk) => `<li>${Utils.escapeHtml(risk)}</li>`)
      .join("");

    return section("Risk", rows + (risks ? `<ul class="pdd-risk-list">${risks}</ul>` : ""));
  };

  proto._buildMarketSection = function () {
    const market = this.fullDetails?.market_data;
    const pool = this.fullDetails?.pool_info;
    const usd = (value) => (value ? Utils.formatCurrencyUSD(value) : "—");
    const rows = [];

    if (pool?.dex_name || pool?.liquidity_sol != null) {
      rows.push(
        fact("Pool", Utils.escapeHtml(pool.dex_name || "—"), {
          sub:
            pool.liquidity_sol != null
              ? `${Utils.formatCompactNumber(pool.liquidity_sol)} SOL liquidity`
              : "",
        })
      );
    }

    if (market) {
      rows.push(
        fact("Market cap", usd(market.market_cap), {
          sub: market.fdv && market.fdv !== market.market_cap ? `FDV ${usd(market.fdv)}` : "",
        })
      );
      rows.push(fact("Liquidity", usd(market.liquidity_usd)));
      rows.push(fact("Volume 24h", usd(market.volume_24h)));

      const changes = [
        ["1h", market.price_change_h1],
        ["24h", market.price_change_h24],
      ].filter(([, value]) => value != null);
      if (changes.length) {
        rows.push(
          fact(
            "Price change",
            changes
              .map(
                ([period, value]) =>
                  `<span class="pdd-change">${period} <span class="${this._toneClass(value)}">${this._formatPct(value, 1)}</span></span>`
              )
              .join("")
          )
        );
      }
      rows.push(
        fact("Holders", market.holder_count ? Utils.formatCompactNumber(market.holder_count) : "—")
      );
    }

    // A finished position is read long after it closed: say these are today's numbers.
    return section(this._isSettled() ? "Market now" : "Market", facts(rows));
  };

  /** Outside references. Solscan already has its own control in the header. */
  proto._buildLinksSection = function () {
    const tokenInfo = this.fullDetails?.token_info;
    const links = this.fullDetails?.external_links || {};
    const items = [
      ["Website", tokenInfo?.website],
      ["X", tokenInfo?.twitter],
      ["Telegram", tokenInfo?.telegram],
      ["DexScreener", links.dexscreener],
      ["Birdeye", links.birdeye],
      ["RugCheck", links.rugcheck],
      ["Photon", links.photon],
    ].filter(([, url]) => isWebUrl(url));
    if (!items.length) return "";

    return section(
      "Links",
      `<div class="pdd-links">${items
        .map(
          ([label, url]) =>
            `<a class="pdd-link" href="${Utils.escapeHtml(url)}" target="_blank" rel="noopener">${label}<i class="icon-arrow-up-right"></i></a>`
        )
        .join("")}</div>`
    );
  };
}
