// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Pure presentation helpers for Settings > Updates.
 *
 * The controller owns requests and lifecycle. This module turns updater state,
 * release-note text, and configuration metadata into stable dashboard markup.
 */

import {
  categoryLabel,
  fieldHint,
  fieldLabel,
  fieldUnit,
} from "../../pages/config/field_text.js";
import { UPDATE_PHASE_DETAIL_LABELS, UPDATE_PHASE_HEADLINE_LABELS } from "./update_status.js";

const PREFERENCE_ORDER = [
  "auto_check",
  "check_interval_hours",
  "auto_download",
  "auto_install",
  "defer_while_trading",
  "notify_telegram",
];

const CATEGORY_ORDER = ["checking", "installing", "notifications"];

// Phases whose status line already names the installed build (the idle and
// up-to-date details, the "Updated to" headline). The installed version is shown
// once per view: in that line, in the version flow while an update is pending, or
// otherwise in the details list.
const PHASES_NAMING_INSTALLED = new Set(["idle", "up_to_date", "applied"]);

// Ids are the serialized `UpdateKind` values (src/version/types.rs).
const UPDATE_KIND_LABELS = Object.freeze({
  core: "updates-kind-core",
  full: "updates-kind-full",
});

// What a release is to this installation; a past release carries no tag.
const RELEASE_STATE_LABELS = Object.freeze({
  installed: "updates-version-installed",
  available: "updates-version-available",
});

function orderBy(items, preferredOrder, valueFor) {
  const ranks = new Map(preferredOrder.map((value, index) => [value, index]));
  return items.sort((left, right) => {
    const leftValue = valueFor(left);
    const rightValue = valueFor(right);
    const leftRank = ranks.get(leftValue) ?? preferredOrder.length;
    const rightRank = ranks.get(rightValue) ?? preferredOrder.length;
    return leftRank - rightRank || String(leftValue).localeCompare(String(rightValue));
  });
}

export function parseReleaseNotes(source) {
  const document = { title: "", intro: [], sections: [] };
  let section = null;

  for (const rawLine of String(source || "").split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line) continue;

    if (line.startsWith("### ")) {
      section = { heading: line.slice(4).trim(), bullets: [], paragraphs: [] };
      document.sections.push(section);
      continue;
    }

    if (line.startsWith("## ")) {
      document.title = line.slice(3).trim();
      continue;
    }

    if (line.startsWith("- ")) {
      if (!section) {
        section = { heading: I18n.t("updates-notes-highlights"), bullets: [], paragraphs: [] };
        document.sections.push(section);
      }
      section.bullets.push(line.slice(2).trim());
      continue;
    }

    if (section) {
      section.paragraphs.push(line);
    } else {
      document.intro.push(line);
    }
  }

  return document;
}

export function createUpdatesView(Utils) {
  const escape = (value) => Utils.escapeHtml(String(value ?? ""));

  function button(id, label, icon, variant = "primary") {
    return `
      <button class="btn btn-${variant}" id="${id}" type="button">
        ${icon ? `<i class="${icon}" aria-hidden="true"></i>` : ""}<span>${escape(label)}</span>
      </button>
    `;
  }

  const versionText = (version) => I18n.t("updates-version-number", { version });

  function updateSize(update) {
    return Utils.formatBytes(
      update?.kind === "core" ? update.core?.size : update?.file_size,
      I18n.t("updates-size-unknown")
    );
  }

  function describeKind(update) {
    return I18n.label(UPDATE_KIND_LABELS, update?.kind === "core" ? "core" : "full", {
      size: updateSize(update),
    });
  }

  function progressBar(progress, indeterminate, label) {
    const width = Math.max(0, Math.min(100, Math.round(progress.progress_percent || 0)));
    const percent = Utils.formatPercentValue(width, { decimals: 0, includeSign: false });
    const transferred = I18n.t("updates-progress-transferred", {
      done: Utils.formatBytes(progress.bytes_downloaded || 0),
      total: Utils.formatBytes(progress.total_bytes || 0),
    });
    const valueText = indeterminate
      ? label
      : I18n.t("updates-progress-value-text", { label, transferred, percent });
    return `
      <div class="updates-progress-copy">
        <span>${escape(label)}</span>
        ${indeterminate ? "" : `<span>${escape(I18n.t("updates-progress-summary", { transferred, percent }))}</span>`}
      </div>
      <div class="updates-progress${indeterminate ? " is-indeterminate" : ""}"
        role="progressbar" aria-label="${escape(label)}" aria-valuemin="0" aria-valuemax="100"
        aria-valuetext="${escape(valueText)}"${indeterminate ? "" : ` aria-valuenow="${width}"`}>
        <div class="updates-progress-fill"${indeterminate ? "" : ` style="width:${width}%"`}></div>
      </div>
    `;
  }

  function detailRows(state, update) {
    const rows = [];
    if (!update && !PHASES_NAMING_INSTALLED.has(state.phase)) {
      rows.push([I18n.t("updates-detail-installed-version"), versionText(state.currentVersion)]);
    }
    rows.push(
      [I18n.t("updates-detail-system"), state.platform || I18n.t("format-unknown")],
      [
        I18n.t("updates-detail-last-checked"),
        Utils.formatTimestamp(state.last_check || state.last_check_attempt, {
          fallback: I18n.t("updates-detail-never"),
          includeSeconds: false,
        }),
      ]
    );

    if (update) {
      rows.push([I18n.t("updates-detail-download-size"), updateSize(update)]);
    }

    return `
      <dl class="updates-detail-list" aria-label="${escape(I18n.t("updates-detail-list-label"))}">
        ${rows
          .map(
            ([label, value]) => `
              <div class="updates-detail-row">
                <dt>${escape(label)}</dt>
                <dd>${escape(value)}</dd>
              </div>`
          )
          .join("")}
      </dl>
    `;
  }

  /** Installed to available, shown only while an update is pending. */
  function renderVersions(current, update) {
    if (!update) return "";

    return `
      <div class="updates-version-flow">
        <div class="updates-version-point">
          <span>${escape(I18n.t("updates-version-installed"))}</span>
          <strong>${escape(versionText(current))}</strong>
        </div>
        <i class="icon-arrow-right" aria-hidden="true"></i>
        <div class="updates-version-point updates-version-point--target">
          <span>${escape(I18n.t("updates-version-available"))}</span>
          <strong>${escape(versionText(update.version))}</strong>
        </div>
      </div>
    `;
  }

  function renderStatus(rawState) {
    const state = { ...rawState };
    const update = state.available_update;
    const progress = state.download_progress || {};
    const current = state.currentVersion;
    const recognized = Object.hasOwn(UPDATE_PHASE_HEADLINE_LABELS, state.phase);
    // "Updated to vX" names the running build; every other headline names the pending update.
    const headlineVersion = state.phase === "applied" ? current : update?.version || "";
    let headline = I18n.t("updates-status-unavailable-headline");
    let detail = I18n.t("updates-phase-unrecognized-detail");
    let icon = "icon-circle-alert";
    let tone = "warning";
    let actions = [
      button("updatesCheck", I18n.t("updates-action-check-again"), "icon-refresh-cw", "secondary"),
    ];
    let progressHtml = "";

    if (recognized) {
      headline = I18n.label(UPDATE_PHASE_HEADLINE_LABELS, state.phase, { version: headlineVersion });
      if (Object.hasOwn(UPDATE_PHASE_DETAIL_LABELS, state.phase)) {
        detail = I18n.label(UPDATE_PHASE_DETAIL_LABELS, state.phase, { version: current });
      }
    }

    switch (state.phase) {
      case "idle":
        icon = "icon-refresh-cw";
        tone = "neutral";
        actions = [button("updatesCheck", I18n.t("updates-action-check-now"), "icon-refresh-cw")];
        break;
      case "up_to_date":
        icon = "icon-circle-check";
        tone = "success";
        break;
      case "checking":
        icon = "icon-loader";
        tone = "neutral";
        actions = [];
        progressHtml = progressBar(progress, true, headline);
        break;
      case "available":
        icon = "icon-arrow-down-to-line";
        tone = "primary";
        if (state.self_install === false) {
          // A headless installation is updated by its package manager.
          detail = I18n.t("updates-headless-install-detail");
          actions = [];
        } else {
          detail = describeKind(update);
          actions = [
            button("updatesDownload", I18n.t("updates-action-download"), "icon-arrow-down-to-line"),
          ];
        }
        break;
      case "downloading":
        detail = describeKind(update);
        icon = "icon-arrow-down-to-line";
        tone = "primary";
        actions = [];
        progressHtml = progressBar(progress, false, I18n.t("updates-progress-downloading"));
        break;
      case "verifying":
        icon = "icon-shield-check";
        tone = "primary";
        actions = [];
        progressHtml = progressBar(progress, true, I18n.t("updates-progress-verifying"));
        break;
      case "ready_to_apply":
        detail = I18n.text(state.blocked_reason) || detail;
        icon = "icon-circle-check";
        tone = "success";
        actions = [button("updatesApply", I18n.t("updates-action-restart"), "icon-refresh-cw")];
        break;
      case "ready_to_install":
        detail = I18n.text(state.blocked_reason) || detail;
        icon = "icon-package";
        tone = "primary";
        actions = [button("updatesInstall", I18n.t("updates-action-open-installer"), "icon-package")];
        break;
      case "applying":
        icon = "icon-loader";
        tone = "primary";
        actions = [];
        progressHtml = progressBar(progress, true, headline);
        break;
      case "applied":
        icon = "icon-circle-check";
        tone = "success";
        break;
      case "failed":
        detail = progress.error || detail;
        icon = "icon-circle-x";
        tone = "error";
        actions = [button("updatesRetry", I18n.t("updates-action-try-again"), "icon-refresh-cw")];
        break;
      case "check_failed":
        detail = I18n.text(state.check_error) || detail;
        icon = "icon-circle-alert";
        tone = "error";
        actions = [button("updatesCheck", I18n.t("updates-action-try-again"), "icon-refresh-cw")];
        break;
    }

    return {
      announcement: `${headline}. ${detail}`,
      html: `
        <section class="updates-status" data-tone="${tone}">
          <div class="updates-status-copy">
            <i class="updates-status-icon ${icon}" aria-hidden="true"></i>
            <div>
              <h3 class="updates-headline">${escape(headline)}</h3>
              <p class="updates-detail">${escape(detail)}</p>
            </div>
          </div>
          ${renderVersions(current, update)}
          ${progressHtml}
          ${actions.length ? `<div class="updates-actions">${actions.join("")}</div>` : ""}
          ${detailRows(state, update)}
        </section>
      `,
    };
  }

  function releaseBodyHtml(release) {
    const parsed = parseReleaseNotes(release.release_notes);
    const intro = parsed.intro.map((paragraph) => `<p>${escape(paragraph)}</p>`).join("");
    const sections = parsed.sections
      .map((section) => {
        const paragraphs = section.paragraphs
          .map((paragraph) => `<p>${escape(paragraph)}</p>`)
          .join("");
        const bullets = section.bullets.length
          ? `<ul>${section.bullets.map((bullet) => `<li>${escape(bullet)}</li>`).join("")}</ul>`
          : "";
        return `
          <section class="updates-release-section">
            <h4>${escape(section.heading)}</h4>
            ${paragraphs}${bullets}
          </section>
        `;
      })
      .join("");

    return {
      changeCount: parsed.sections.reduce((total, section) => total + section.bullets.length, 0),
      html:
        intro + sections ||
        `<p class="updates-release-empty">${escape(I18n.t("updates-release-empty"))}</p>`,
    };
  }

  /**
   * One release in the history list.
   *
   * `state` is what this version is to this installation — the build running
   * right now, the update waiting to be installed, or an earlier release — and
   * it is the only thing that earns a tag. The newest entry carries no "latest"
   * tag of its own: it is already first in the list.
   */
  function releaseEntryHtml(release, state, expanded) {
    const body = releaseBodyHtml(release);
    const date = Utils.formatDate(release.release_date, { fallback: "" });
    const tag = Object.hasOwn(RELEASE_STATE_LABELS, state)
      ? I18n.label(RELEASE_STATE_LABELS, state)
      : "";
    const changes = body.changeCount
      ? I18n.t("updates-release-changes", { count: body.changeCount })
      : "";

    return `
      <li class="updates-release-item">
        <details class="updates-release" data-state="${escape(state)}"${expanded ? " open" : ""}>
          <summary class="updates-release-summary">
            <span class="updates-release-version">${escape(versionText(release.version))}</span>
            ${tag ? `<span class="updates-release-tag">${escape(tag)}</span>` : ""}
            <span class="updates-release-facts">
              ${date ? `<time datetime="${escape(release.release_date)}">${escape(date)}</time>` : ""}
              ${changes ? `<span>${escape(changes)}</span>` : ""}
            </span>
            <i class="updates-release-chevron icon-chevron-down" aria-hidden="true"></i>
          </summary>
          <div class="updates-release-body">${body.html}</div>
        </details>
      </li>
    `;
  }

  /**
   * The Release Notes panel: every published release, newest first.
   *
   * `releases` comes from screenerbot.io, which owns the editorial record. When
   * it cannot be read the panel still shows whatever single release the updater
   * itself knows about, so an offline machine is never left with nothing.
   */
  function renderReleaseNotes({ releases, currentVersion, availableVersion, loading, error }) {
    if (loading) {
      return `<div class="updates-loading">${escape(I18n.t("common-loading"))}</div>`;
    }

    if (!releases.length) {
      return `
        <div class="updates-empty">
          <h3>${escape(I18n.t("updates-notes-empty-title"))}</h3>
          <p>
            ${escape(error ? I18n.t("updates-notes-empty-error") : I18n.t("updates-notes-empty-none"))}
          </p>
          ${button("updatesNotesRetry", I18n.t("updates-action-try-again"), "icon-refresh-cw", "secondary")}
        </div>
      `;
    }

    const stateFor = (version) => {
      if (version === currentVersion) return "installed";
      if (version === availableVersion) return "available";
      return "past";
    };
    // One entry starts open, and it is the one the reader came for: the update
    // waiting to be installed if there is one, otherwise the build they are
    // running, otherwise the newest release.
    const preference = ["available", "installed"];
    const expandedIndex = Math.max(
      0,
      preference
        .map((wanted) => releases.findIndex((release) => stateFor(release.version) === wanted))
        .find((index) => index >= 0) ?? -1
    );

    return `
      <div class="updates-history">
        ${
          error
            ? `<p class="updates-history-notice">${escape(I18n.t("updates-notes-history-notice"))}</p>`
            : ""
        }
        <ol class="updates-release-list">
          ${releases
            .map((release, index) =>
              releaseEntryHtml(release, stateFor(release.version), index === expandedIndex)
            )
            .join("")}
        </ol>
      </div>
    `;
  }

  function renderPreferenceControl(key, value, metadata) {
    if (metadata.type === "boolean") {
      return `
        <label class="toggle">
          <input type="checkbox" id="updatePref_${escape(key)}" data-pref="${escape(key)}"
            data-saved-value="${value ? "true" : "false"}"${value ? " checked" : ""}>
          <span class="toggle-track"></span>
        </label>
      `;
    }

    const min = Number.isFinite(metadata.min) ? ` min="${metadata.min}"` : "";
    const max = Number.isFinite(metadata.max) ? ` max="${metadata.max}"` : "";
    const step = Number.isFinite(metadata.step) ? ` step="${metadata.step}"` : "";
    const unit = fieldUnit(metadata.key);
    return `
      <div class="updates-number-control">
        <input class="updates-number-input" type="number" id="updatePref_${escape(key)}"
          data-pref="${escape(key)}" data-saved-value="${escape(value)}" value="${escape(value)}"
          ${min}${max}${step}>
        ${unit ? `<span class="input-unit">${escape(unit)}</span>` : ""}
      </div>
    `;
  }

  function renderPreferences(values, metadata) {
    const fields = orderBy(
      Object.entries(metadata)
        .filter(
          ([, field]) => !field.hidden && ["boolean", "integer", "number"].includes(field.type)
        )
        .map(([key, field]) => ({ key, value: values[key] ?? field.default, metadata: field })),
      PREFERENCE_ORDER,
      (field) => field.key
    );

    if (!fields.length) {
      return `
        <div class="updates-empty">
          <h3>${escape(I18n.t("updates-preferences-unavailable-title"))}</h3>
          <p>${escape(I18n.t("updates-preferences-unavailable-detail"))}</p>
          ${button("updatesPrefsRetry", I18n.t("updates-action-try-again"), "icon-refresh-cw")}
        </div>
      `;
    }

    const categoryOf = (field) => field.metadata.category || "general";
    const categoryIds = [...new Set(fields.map(categoryOf))];
    const categories = orderBy(categoryIds, CATEGORY_ORDER, (category) => category).map(
      (category) => ({
        category,
        fields: fields.filter((field) => categoryOf(field) === category),
      })
    );

    return categories
      .map(
        ({ category, fields: categoryFields }) => `
          <section class="updates-preference-section">
            <h3 class="updates-subhead">${escape(categoryLabel(category))}</h3>
            <div class="settings-group">
              ${categoryFields
                .map(
                  ({ key, value, metadata: field }) => `
                    <div class="settings-field" data-update-field="${escape(key)}">
                      <div class="settings-field-info">
                        <label for="updatePref_${escape(key)}">${escape(fieldLabel(field.key))}</label>
                        ${fieldHint(field.key) ? `<span class="settings-field-hint">${escape(fieldHint(field.key))}</span>` : ""}
                      </div>
                      <div class="settings-field-control">
                        ${renderPreferenceControl(key, value, field)}
                      </div>
                    </div>
                  `
                )
                .join("")}
            </div>
          </section>
        `
      )
      .join("");
  }

  return { renderStatus, renderReleaseNotes, renderPreferences };
}
