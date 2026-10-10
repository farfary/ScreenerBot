// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Pure presentation helpers for the storage section of Settings > Data.
 *
 * The data tab owns requests and lifecycle. This module turns the storage report
 * (`GET /api/system/storage`, `storage::report::StorageReport`) into markup and
 * the storage limit choices.
 */

/** Category id -> label key, keyed by `storage::types::Category`. */
export const STORAGE_CATEGORY_LABELS = Object.freeze({
  wallet_keys: "settings-data-category-wallet-keys",
  trading_records: "settings-data-category-trading-records",
  wallet_history: "settings-data-category-wallet-history",
  market_data: "settings-data-category-market-data",
  settings: "settings-data-category-settings",
  recovery_copies: "settings-data-category-recovery-copies",
  updates: "settings-data-category-updates",
  logs: "settings-data-category-logs",
  leftovers: "settings-data-category-leftovers",
});

/** Fixed storage limits offered besides automatic, in gigabytes. */
export const STORAGE_LIMIT_CHOICES_GB = Object.freeze([1, 2, 5, 10, 20]);

const BYTES_PER_MEGABYTE = 1024 * 1024;
const MEGABYTES_PER_GIGABYTE = 1024;

export function createStorageView(Utils) {
  const escape = (value) => Utils.escapeHtml(value);

  function fact(label, value, tone = "") {
    return `
      <div class="data-fact">
        <span class="data-fact-label">${escape(label)}</span>
        <span class="data-fact-value${tone ? ` ${tone}` : ""}">${escape(value)}</span>
      </div>`;
  }

  function lastCleanupText(report) {
    if (report.running) return I18n.t("settings-data-cleanup-now-running");
    const last = report.last_cleanup;
    if (!last) return I18n.t("settings-data-last-cleanup-never");
    return I18n.t("settings-data-last-cleanup-value", {
      when: Utils.formatTimeAgo(last.finished_at),
      freed: Utils.formatBytes(last.freed_bytes),
    });
  }

  /** The usage headline, one bar per category that takes space, and the disk facts. */
  function overviewHtml(report) {
    const categories = (report.categories || []).filter((entry) => entry.bytes > 0);
    const largest = Math.max(1, ...categories.map((entry) => entry.bytes));
    const rows = categories
      .map(
        (entry) => `
          <div class="data-db-item">
            <span class="data-db-name">${escape(I18n.label(STORAGE_CATEGORY_LABELS, entry.category))}</span>
            <div class="data-db-bar-container">
              <div class="data-db-bar" style="width: ${(entry.bytes / largest) * 100}%"></div>
            </div>
            <span class="data-db-size">${escape(Utils.formatBytes(entry.bytes))}</span>
          </div>`
      )
      .join("");

    const disk = report.disk
      ? I18n.t("settings-data-usage-free-disk-value", {
          free: Utils.formatBytes(report.disk.free_bytes),
          total: Utils.formatBytes(report.disk.total_bytes),
        })
      : I18n.t("settings-data-usage-free-disk-unknown");
    const failures = report.last_cleanup?.failures || 0;

    return `
      <div class="data-total-bar">
        <span class="data-total-label">${escape(I18n.t("settings-data-usage-total"))}</span>
        <span class="data-total-value">${escape(
          I18n.t("settings-data-usage-of-limit", {
            used: Utils.formatBytes(report.total_bytes),
            limit: Utils.formatBytes(report.limit_bytes),
          })
        )}</span>
      </div>
      <div class="data-db-list">${rows}</div>
      <div class="data-facts">
        ${fact(I18n.t("settings-data-usage-free-disk"), disk)}
        ${fact(I18n.t("settings-data-last-cleanup-label"), lastCleanupText(report))}
        ${
          failures > 0 && !report.running
            ? fact(
                I18n.t("settings-data-last-cleanup-failures-label"),
                I18n.t("settings-data-last-cleanup-failures", { count: failures }),
                "warning"
              )
            : ""
        }
      </div>`;
  }

  /**
   * The storage limit choices, in megabytes as `maintenance.storage_limit_mb`
   * stores them: automatic with the size it resolves to on this disk, the fixed
   * sizes, and the configured size when it is none of them.
   */
  function limitOptions(report) {
    const configured = Number(report.limit_megabytes) || 0;
    const options = [
      {
        value: "0",
        label: I18n.t("settings-data-limit-automatic", {
          size: Utils.formatBytes(report.automatic_limit_bytes),
        }),
      },
      ...STORAGE_LIMIT_CHOICES_GB.map((gigabytes) => ({
        value: String(gigabytes * MEGABYTES_PER_GIGABYTE),
        label: Utils.formatSizeAt(gigabytes, { unit: "gb", decimals: 0 }),
      })),
    ];
    if (!options.some((option) => Number(option.value) === configured)) {
      options.push({
        value: String(configured),
        label: Utils.formatBytes(configured * BYTES_PER_MEGABYTE),
      });
    }
    return options.map((option) => ({ ...option, selected: Number(option.value) === configured }));
  }

  return { overviewHtml, limitOptions };
}
