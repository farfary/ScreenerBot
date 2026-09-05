/**
 * Pure presentation helpers for Settings > Updates.
 *
 * The controller owns requests and lifecycle. This module turns updater state,
 * release-note text, and configuration metadata into stable dashboard markup.
 */

const PREFERENCE_ORDER = [
  "auto_check",
  "check_interval_hours",
  "auto_download",
  "auto_install",
  "defer_while_trading",
  "notify_telegram",
];

const CATEGORY_ORDER = ["Checking", "Installing", "Notifications"];

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
        section = { heading: "Highlights", bullets: [], paragraphs: [] };
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

  function updateSize(update) {
    return Utils.formatBytes(
      update?.kind === "core" ? update.core?.size : update?.file_size,
      "unknown size"
    );
  }

  function describeKind(update) {
    const size = updateSize(update);
    return update?.kind === "core"
      ? `Core update · ${size} · short restart`
      : `Desktop update · ${size} · installer required`;
  }

  function progressBar(progress, indeterminate, label) {
    const width = Math.max(0, Math.min(100, Math.round(progress.progress_percent || 0)));
    const transferred = `${Utils.formatBytes(progress.bytes_downloaded || 0)} of ${Utils.formatBytes(
      progress.total_bytes || 0
    )}`;
    const valueText = indeterminate ? label : `${label}, ${width}%, ${transferred}`;

    return `
      <div class="updates-progress-copy">
        <span>${escape(label)}</span>
        ${indeterminate ? "" : `<span>${escape(transferred)} · ${width}%</span>`}
      </div>
      <div class="updates-progress${indeterminate ? " is-indeterminate" : ""}"
        role="progressbar" aria-label="${escape(label)}" aria-valuemin="0" aria-valuemax="100"
        aria-valuetext="${escape(valueText)}"${indeterminate ? "" : ` aria-valuenow="${width}"`}>
        <div class="updates-progress-fill"${indeterminate ? "" : ` style="width:${width}%"`}></div>
      </div>
    `;
  }

  function detailRows(state, update) {
    const rows = [
      ["Installed version", `v${state.currentVersion}`],
      ["System", state.platform || "Unknown"],
      [
        "Last checked",
        Utils.formatTimestamp(state.last_check || state.last_check_attempt, {
          fallback: "Never",
          includeSeconds: false,
        }),
      ],
    ];

    if (update) {
      rows.push(["Available version", `v${update.version}`]);
      rows.push(["Download size", updateSize(update)]);
    }

    return `
      <dl class="updates-detail-list" aria-label="Installation details">
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

  function renderVersions(current, update) {
    if (!update) {
      return `
        <div class="updates-version-flow updates-version-flow--single">
          <div class="updates-version-point">
            <span>Installed</span>
            <strong>v${escape(current)}</strong>
          </div>
        </div>
      `;
    }

    return `
      <div class="updates-version-flow">
        <div class="updates-version-point">
          <span>Installed</span>
          <strong>v${escape(current)}</strong>
        </div>
        <i class="icon-arrow-right" aria-hidden="true"></i>
        <div class="updates-version-point updates-version-point--target">
          <span>Available</span>
          <strong>v${escape(update.version)}</strong>
        </div>
      </div>
    `;
  }

  function renderStatus(rawState) {
    const state = { ...rawState };
    const update = state.available_update;
    const progress = state.download_progress || {};
    const current = state.currentVersion;
    let headline = "Update status is unavailable";
    let detail = "The reported update state is not recognized.";
    let icon = "icon-circle-alert";
    let tone = "warning";
    let actions = [button("updatesCheck", "Check again", "icon-refresh-cw", "ghost")];
    let progressHtml = "";

    switch (state.phase) {
      case "idle":
        headline = "Ready to check for updates";
        detail = `ScreenerBot v${current} is installed.`;
        icon = "icon-refresh-cw";
        tone = "neutral";
        actions = [button("updatesCheck", "Check now", "icon-refresh-cw")];
        break;
      case "up_to_date":
        headline = "You are up to date";
        detail = `ScreenerBot v${current} is the latest version.`;
        icon = "icon-circle-check";
        tone = "success";
        actions = [button("updatesCheck", "Check again", "icon-refresh-cw", "ghost")];
        break;
      case "checking":
        headline = "Checking for updates";
        detail = "Looking for the latest published release.";
        icon = "icon-loader";
        tone = "neutral";
        actions = [];
        progressHtml = progressBar(progress, true, "Checking for updates");
        break;
      case "available":
        headline = `Version ${update?.version || ""} is available`;
        detail = describeKind(update);
        icon = "icon-arrow-down-to-line";
        tone = "primary";
        actions = [button("updatesDownload", "Download update", "icon-arrow-down-to-line")];
        break;
      case "downloading":
        headline = `Downloading v${update?.version || ""}`;
        detail = describeKind(update);
        icon = "icon-arrow-down-to-line";
        tone = "primary";
        actions = [];
        progressHtml = progressBar(progress, false, "Downloading update");
        break;
      case "verifying":
        headline = `Verifying v${update?.version || ""}`;
        detail = "Checking the download against its published checksum.";
        icon = "icon-shield-check";
        tone = "primary";
        actions = [];
        progressHtml = progressBar(progress, true, "Verifying update");
        break;
      case "ready_to_apply":
        headline = `Version ${update?.version || ""} is ready`;
        detail =
          state.blocked_reason ||
          "The update can be installed now with a short restart, or automatically on the next start.";
        icon = "icon-circle-check";
        tone = "success";
        actions = [button("updatesApply", "Restart to update", "icon-refresh-cw")];
        break;
      case "ready_to_install":
        headline = `Version ${update?.version || ""} is ready`;
        detail = state.blocked_reason || "The desktop installer is ready to finish this update.";
        icon = "icon-package";
        tone = "primary";
        actions = [button("updatesInstall", "Open installer", "icon-package")];
        break;
      case "applying":
        headline = "Installing update";
        detail = "ScreenerBot is restarting onto the new version.";
        icon = "icon-loader";
        tone = "primary";
        actions = [];
        progressHtml = progressBar(progress, true, "Installing update");
        break;
      case "applied":
        headline = `Updated to v${current}`;
        detail = "The update was installed. Nothing else is needed.";
        icon = "icon-circle-check";
        tone = "success";
        actions = [button("updatesCheck", "Check again", "icon-refresh-cw", "ghost")];
        break;
      case "failed":
        headline = "The update did not finish";
        detail = progress.error || "Try the update again.";
        icon = "icon-circle-x";
        tone = "error";
        actions = [button("updatesRetry", "Try again", "icon-refresh-cw")];
        break;
      case "check_failed":
        headline = "Could not check for updates";
        detail = state.check_error || "The release service could not be reached.";
        icon = "icon-circle-alert";
        tone = "error";
        actions = [button("updatesCheck", "Try again", "icon-refresh-cw")];
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

  function renderReleaseNotes(release) {
    if (!release) {
      return `
        <div class="updates-empty">
          <h3>No release notes yet</h3>
          <p>Release notes will appear here when a new version is available.</p>
        </div>
      `;
    }

    const parsed = parseReleaseNotes(release.release_notes);
    const date = Utils.formatTimestamp(release.release_date, {
      fallback: "",
      includeSeconds: false,
    });
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

    return `
      <article class="updates-release">
        <header class="updates-release-header">
          <span>Release notes</span>
          <h3>What’s new in v${escape(release.version)}</h3>
          ${date ? `<time datetime="${escape(release.release_date)}">Released ${escape(date)}</time>` : ""}
        </header>
        ${intro}${sections || '<p class="updates-release-empty">No changes were listed for this release.</p>'}
      </article>
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
    return `
      <div class="updates-number-control">
        <input class="updates-number-input" type="number" id="updatePref_${escape(key)}"
          data-pref="${escape(key)}" data-saved-value="${escape(value)}" value="${escape(value)}"
          ${min}${max}${step}>
        ${metadata.unit ? `<span class="input-unit">${escape(metadata.unit)}</span>` : ""}
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
          <h3>Update preferences are unavailable</h3>
          <p>The update configuration could not be loaded.</p>
          ${button("updatesPrefsRetry", "Try again", "icon-refresh-cw")}
        </div>
      `;
    }

    const categoryNames = [...new Set(fields.map((field) => field.metadata.category || "Other"))];
    const categories = orderBy(categoryNames, CATEGORY_ORDER, (category) => category).map(
      (category) => ({
        category,
        fields: fields.filter((field) => (field.metadata.category || "Other") === category),
      })
    );

    return categories
      .map(
        ({ category, fields: categoryFields }) => `
          <section class="updates-preference-section">
            <h3 class="updates-subhead">${escape(category)}</h3>
            <div class="settings-group">
              ${categoryFields
                .map(
                  ({ key, value, metadata: field }) => `
                    <div class="settings-field" data-update-field="${escape(key)}">
                      <div class="settings-field-info">
                        <label for="updatePref_${escape(key)}">${escape(field.label || key)}</label>
                        ${field.hint ? `<span class="settings-field-hint">${escape(field.hint)}</span>` : ""}
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
