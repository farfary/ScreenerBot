// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * The updater state as the dashboard reads it.
 *
 * Settings > Updates, its nav indicator, the desktop attention watcher and the
 * Home update notice all read GET /api/updates/status through this one reader
 * and name phases through these maps, so every surface tells the same story.
 */

// Ids are the serialized `UpdatePhase` values (src/version/types.rs); the Rust test
// `update_phase_messages_exist_in_the_catalog` pins the keys.
export const UPDATE_PHASE_HEADLINE_LABELS = Object.freeze({
  idle: "updates-phase-idle-headline",
  up_to_date: "updates-phase-up-to-date-headline",
  checking: "updates-phase-checking-headline",
  available: "updates-phase-available-headline",
  downloading: "updates-phase-downloading-headline",
  verifying: "updates-phase-verifying-headline",
  ready_to_apply: "updates-phase-ready-to-apply-headline",
  ready_to_install: "updates-phase-ready-to-install-headline",
  applying: "updates-phase-applying-headline",
  applied: "updates-phase-applied-headline",
  failed: "updates-phase-failed-headline",
  check_failed: "updates-phase-check-failed-headline",
});

// The detail line; for the phases that can carry backend text it is the fallback.
// Available and downloading updates describe their kind instead.
export const UPDATE_PHASE_DETAIL_LABELS = Object.freeze({
  idle: "updates-phase-idle-detail",
  up_to_date: "updates-phase-up-to-date-detail",
  checking: "updates-phase-checking-detail",
  verifying: "updates-phase-verifying-detail",
  ready_to_apply: "updates-phase-ready-to-apply-detail",
  ready_to_install: "updates-phase-ready-to-install-detail",
  applying: "updates-phase-applying-detail",
  applied: "updates-phase-applied-detail",
  failed: "updates-phase-failed-detail",
  check_failed: "updates-phase-check-failed-detail",
});

/**
 * Read the updater state. Returns the `UpdateState` with the response's
 * derived fields folded in, or null when the status could not be read.
 */
export async function readUpdateStatus(signal) {
  try {
    const response = await fetch("/api/updates/status", { signal, cache: "no-store" });
    const body = await response.json();
    if (!response.ok || body.success === false) return null;
    const payload = body.data || body;
    const state = payload.state || payload;
    state.blocked_reason = payload.blocked_reason || null;
    state.requires_user_action = Boolean(payload.requires_user_action);
    state.self_install = payload.self_install !== false;
    state.current_version = payload.current_version || null;
    return state;
  } catch {
    return null;
  }
}
