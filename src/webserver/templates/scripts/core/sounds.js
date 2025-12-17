/**
 * Sound Effects System
 * Provides subtle audio feedback for dashboard interactions using Web Audio API
 * Generates tones programmatically - no external audio files needed
 *
 * Sound Types:
 * - playClick: Button clicks (1200Hz, 30ms)
 * - playTabSwitch: Tab/navigation switching (higher pitch, subtle)
 * - playToggleOn: Toggle activated (ascending sweep)
 * - playToggleOff: Toggle deactivated (descending sweep)
 * - playSuccess: Success confirmation (two-tone chime)
 * - playError: Error feedback (low warning tone)
 * - playPanelOpen: Panel/dialog opening (soft whoosh up)
 * - playPanelClose: Panel/dialog closing (soft whoosh down)
 */

/* global performance */

// Sound configuration state
const state = {
  enabled: true, // Default enabled, will be overridden by config on load
  volume: 0.04, // Ultra-low volume for Apple-inspired minimal sounds
  initialized: false,
  context: null,
  lastPlayTime: {}, // Throttle tracking per sound type
};

// Throttle intervals (ms) per sound type to prevent spam
const THROTTLE_MS = {
  click: 30, // Very short for rapid button clicks
  tabSwitch: 100, // Slightly longer for tab switches
  toggle: 150, // Longer for toggle sounds
  panel: 200, // Panel sounds shouldn't overlap
  feedback: 100, // Success/error feedback
};

/**
 * Initialize the audio context (must be called after user interaction)
 */
function initAudioContext() {
  if (state.initialized && state.context) {
    return true;
  }

  try {
    const AudioContext = window.AudioContext || window.webkitAudioContext;
    if (!AudioContext) {
      console.warn("[Sounds] Web Audio API not supported");
      return false;
    }

    state.context = new AudioContext();
    state.initialized = true;

    if (state.context.state === "suspended") {
      state.context.resume();
    }

    return true;
  } catch (err) {
    console.error("[Sounds] Failed to initialize audio context:", err);
    return false;
  }
}

/**
 * Ensure context is ready (resume if suspended)
 */
function ensureContext() {
  if (!state.context) {
    initAudioContext();
  }
  if (state.context && state.context.state === "suspended") {
    state.context.resume();
  }
  return state.context !== null;
}

/**
 * Throttle check - returns true if sound can play
 * @param {string} soundType - Type identifier for throttle tracking
 * @param {number} throttleMs - Minimum time between plays
 */
function canPlaySound(soundType, throttleMs) {
  const now = performance.now();
  const lastTime = state.lastPlayTime[soundType] || 0;

  if (now - lastTime < throttleMs) {
    return false;
  }

  state.lastPlayTime[soundType] = now;
  return true;
}

/**
 * Play a subtle click sound for buttons (Apple-inspired)
 * Pure sine wave, ultra-short duration
 */
export function playClick() {
  if (!state.enabled || !ensureContext()) return;
  if (!canPlaySound("click", THROTTLE_MS.click)) return;

  try {
    const ctx = state.context;
    const now = ctx.currentTime;

    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = "sine";
    // Clean, pure tone - Apple style
    osc.frequency.setValueAtTime(1200, now);

    gain.gain.setValueAtTime(state.volume, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.025);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.03);
  } catch (err) {
    // Silent fail
  }
}

/**
 * Play a subtle tab switch sound (distinct from button click)
 * Slightly higher pitch, even shorter - for navigation
 */
export function playTabSwitch() {
  if (!state.enabled || !ensureContext()) return;
  if (!canPlaySound("tabSwitch", THROTTLE_MS.tabSwitch)) return;

  try {
    const ctx = state.context;
    const now = ctx.currentTime;

    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = "sine";
    // Higher, crisper tone for navigation
    osc.frequency.setValueAtTime(1500, now);
    osc.frequency.exponentialRampToValueAtTime(1400, now + 0.02);

    gain.gain.setValueAtTime(state.volume * 0.8, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.03);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.035);
  } catch (err) {
    // Silent fail
  }
}

/**
 * Play toggle on sound - gentle ascending (Apple-inspired)
 */
export function playToggleOn() {
  if (!state.enabled || !ensureContext()) return;
  if (!canPlaySound("toggle", THROTTLE_MS.toggle)) return;

  try {
    const ctx = state.context;
    const now = ctx.currentTime;

    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = "sine";
    osc.frequency.setValueAtTime(900, now);
    osc.frequency.exponentialRampToValueAtTime(1300, now + 0.05);

    gain.gain.setValueAtTime(state.volume * 1.25, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.06);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.07);
  } catch (err) {
    // Silent fail
  }
}

/**
 * Play toggle off sound - gentle descending (Apple-inspired)
 */
export function playToggleOff() {
  if (!state.enabled || !ensureContext()) return;
  if (!canPlaySound("toggle", THROTTLE_MS.toggle)) return;

  try {
    const ctx = state.context;
    const now = ctx.currentTime;

    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = "sine";
    osc.frequency.setValueAtTime(1100, now);
    osc.frequency.exponentialRampToValueAtTime(800, now + 0.05);

    gain.gain.setValueAtTime(state.volume * 1.25, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.06);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.07);
  } catch (err) {
    // Silent fail
  }
}

/**
 * Play panel/dialog open sound - soft ascending whoosh
 */
export function playPanelOpen() {
  if (!state.enabled || !ensureContext()) return;
  if (!canPlaySound("panel", THROTTLE_MS.panel)) return;

  try {
    const ctx = state.context;
    const now = ctx.currentTime;

    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = "sine";
    osc.frequency.setValueAtTime(600, now);
    osc.frequency.exponentialRampToValueAtTime(1000, now + 0.06);

    gain.gain.setValueAtTime(state.volume * 0.8, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.08);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.1);
  } catch (err) {
    // Silent fail
  }
}

/**
 * Play panel/dialog close sound - soft descending whoosh
 */
export function playPanelClose() {
  if (!state.enabled || !ensureContext()) return;
  if (!canPlaySound("panel", THROTTLE_MS.panel)) return;

  try {
    const ctx = state.context;
    const now = ctx.currentTime;

    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = "sine";
    osc.frequency.setValueAtTime(900, now);
    osc.frequency.exponentialRampToValueAtTime(500, now + 0.06);

    gain.gain.setValueAtTime(state.volume * 0.6, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.07);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.09);
  } catch (err) {
    // Silent fail
  }
}

/**
 * Play success sound - gentle two-tone (Apple-inspired)
 */
export function playSuccess() {
  if (!state.enabled || !ensureContext()) return;
  if (!canPlaySound("feedback", THROTTLE_MS.feedback)) return;

  try {
    const ctx = state.context;
    const now = ctx.currentTime;

    // First tone - lower
    const osc1 = ctx.createOscillator();
    const gain1 = ctx.createGain();
    osc1.type = "sine";
    osc1.frequency.setValueAtTime(1000, now);
    gain1.gain.setValueAtTime(state.volume * 1.5, now);
    gain1.gain.exponentialRampToValueAtTime(0.001, now + 0.08);
    osc1.connect(gain1);
    gain1.connect(ctx.destination);
    osc1.start(now);
    osc1.stop(now + 0.1);

    // Second tone - higher (subtle overlap)
    const osc2 = ctx.createOscillator();
    const gain2 = ctx.createGain();
    osc2.type = "sine";
    osc2.frequency.setValueAtTime(1300, now);
    gain2.gain.setValueAtTime(0, now);
    gain2.gain.setValueAtTime(state.volume * 1.5, now + 0.05);
    gain2.gain.exponentialRampToValueAtTime(0.001, now + 0.13);
    osc2.connect(gain2);
    gain2.connect(ctx.destination);
    osc2.start(now + 0.05);
    osc2.stop(now + 0.15);
  } catch (err) {
    // Silent fail
  }
}

/**
 * Play error sound - subtle low tone (Apple-inspired)
 */
export function playError() {
  if (!state.enabled || !ensureContext()) return;
  if (!canPlaySound("feedback", THROTTLE_MS.feedback)) return;

  try {
    const ctx = state.context;
    const now = ctx.currentTime;

    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = "sine";
    osc.frequency.setValueAtTime(300, now);
    osc.frequency.exponentialRampToValueAtTime(250, now + 0.12);

    gain.gain.setValueAtTime(state.volume * 2, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.15);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.18);
  } catch (err) {
    // Silent fail
  }
}

/**
 * Play a sound by type name (for compatibility)
 * @param {string} soundType - click, tab_switch, toggle_on, toggle_off, success, error, panel_open, panel_close
 */
export function playSound(soundType) {
  switch (soundType) {
    case "click":
      playClick();
      break;
    case "tab_switch":
    case "tabSwitch":
      playTabSwitch();
      break;
    case "toggle_on":
      playToggleOn();
      break;
    case "toggle_off":
      playToggleOff();
      break;
    case "success":
      playSuccess();
      break;
    case "error":
      playError();
      break;
    case "panel_open":
    case "panelOpen":
      playPanelOpen();
      break;
    case "panel_close":
    case "panelClose":
      playPanelClose();
      break;
    default:
      playClick();
  }
}

/**
 * Enable or disable sounds globally
 */
export function setSoundsEnabled(enabled) {
  state.enabled = Boolean(enabled);

  if (state.enabled) {
    initAudioContext();
  }

  saveSoundPreference();
  return state.enabled;
}

/**
 * Check if sounds are enabled
 */
export function isSoundsEnabled() {
  return state.enabled;
}

/**
 * Save sound preference to config
 */
async function saveSoundPreference() {
  try {
    await fetch("/api/config/gui", {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        dashboard: {
          interface: {
            sounds_enabled: state.enabled,
          },
        },
      }),
    });
  } catch (err) {
    // Silent fail
  }
}

/**
 * Load sound preference from config
 */
export async function loadSoundPreference() {
  try {
    const response = await fetch("/api/config/gui");
    if (response.ok) {
      const result = await response.json();
      const config = result.data?.dashboard?.interface;
      if (config && typeof config.sounds_enabled === "boolean") {
        state.enabled = config.sounds_enabled;
      } else {
        // Default to enabled if not found
        state.enabled = true;
      }
    }
  } catch (err) {
    // Silent fail - default to enabled
    state.enabled = true;
  }
}

// Initialize on first user interaction (lazy init for browser autoplay policy)
function initOnInteraction() {
  const handler = () => {
    initAudioContext();
    // { once: true } handles cleanup automatically
  };
  document.addEventListener("click", handler, { once: true });
  document.addEventListener("keydown", handler, { once: true });
}

// Load preferences on module load
loadSoundPreference();
initOnInteraction();

export default {
  playSound,
  playClick,
  playTabSwitch,
  playToggleOn,
  playToggleOff,
  playSuccess,
  playError,
  playPanelOpen,
  playPanelClose,
  setSoundsEnabled,
  isSoundsEnabled,
  loadSoundPreference,
};
