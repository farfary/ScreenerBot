/**
 * Wire contract for the Assistant page "Settings" tab.
 *
 * `GET /api/llm-analysis/config` returns a FLAT `AnalysisConfigResponse` and
 * `PATCH /api/llm-analysis/config` takes a FLAT `UpdateAnalysisConfigRequest`
 * of the same snake_case keys (src/webserver/routes/llm_analysis/types.rs).
 * Every value is a plain scalar: the two confidence fields are a Rust `u8`
 * percentage (0-100), the durations and counts are plain integers. A nested
 * body such as `{ filtering: { min_confidence } }` is silently dropped by
 * serde, so this table is the single place that maps each on-page control to
 * its wire key.
 *
 * Master fields (`enabled` / `default_provider`) are deliberately absent: they
 * are owned by `GET`/`PATCH /api/llm/config` and `assistant.js` carries them
 * on a separate request.
 */

/**
 * `[wireKey, controlId, kind]`. `kind` drives both the form -> wire parse and
 * the slider-label suffix. `"bool"` is a checkbox; `"percent"` / `"seconds"` /
 * `"count"` are `<input type="range">` whose `value` is already the integer the
 * wire expects. Only controls that actually exist in `assistant.html` appear
 * here — fields with no control (e.g. `filtering_use_cache`, the background-check
 * settings) are intentionally not sent.
 */
export const ANALYSIS_CONFIG_FIELDS = [
  ["filtering_enabled", "setting-filtering-enabled", "bool"],
  ["filtering_min_confidence", "setting-min-confidence", "percent"],
  ["filtering_fallback_pass", "setting-fallback-pass", "bool"],
  ["entry_analysis_enabled", "setting-entry-analysis", "bool"],
  ["exit_analysis_enabled", "setting-exit-analysis", "bool"],
  ["ai_trailing_stop_enabled", "setting-trailing-stop", "bool"],
  ["auto_blacklist_enabled", "setting-auto-blacklist-enabled", "bool"],
  ["auto_blacklist_min_confidence", "setting-blacklist-min-confidence", "percent"],
  ["cache_ttl_seconds", "setting-cache-ttl", "seconds"],
  ["max_evaluations_per_minute", "setting-max-evaluations", "count"],
];

/** Text appended after a range value in its `slider-value-*` readout. */
export const SLIDER_SUFFIX = { percent: "%", seconds: "", count: "" };

/**
 * Read every backed control into a flat PATCH body for
 * `/api/llm-analysis/config`. `getControl(id)` returns the element (or a
 * falsy value when it is not in the DOM); absent controls are skipped, so the
 * request only ever carries fields the user can actually see and change.
 */
export function readAnalysisConfigForm(getControl) {
  const patch = {};
  for (const [key, id, kind] of ANALYSIS_CONFIG_FIELDS) {
    const el = getControl(id);
    if (!el) continue;
    if (kind === "bool") {
      patch[key] = !!el.checked;
    } else {
      const value = Number.parseInt(el.value, 10);
      if (Number.isFinite(value)) patch[key] = value;
    }
  }
  return patch;
}

/**
 * Apply a flat `AnalysisConfigResponse` back onto the controls. `getControl(id)`
 * resolves an element; `setSliderLabel(id, text)` writes the range readout.
 * Keys missing from `config` are left untouched so a partial payload never
 * blanks a field.
 */
export function applyAnalysisConfigForm(config, getControl, setSliderLabel) {
  for (const [key, id, kind] of ANALYSIS_CONFIG_FIELDS) {
    const el = getControl(id);
    if (!el || config[key] === undefined || config[key] === null) continue;
    if (kind === "bool") {
      el.checked = !!config[key];
    } else {
      el.value = String(config[key]);
      if (setSliderLabel) setSliderLabel(id, `${config[key]}${SLIDER_SUFFIX[kind]}`);
    }
  }
}

/** The `slider-value-*` readout element id that pairs with a `setting-*` control. */
export function sliderLabelId(controlId) {
  return `slider-value-${controlId.replace(/^setting-/, "")}`;
}
