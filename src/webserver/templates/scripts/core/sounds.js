/**
 * Dashboard sound feedback.
 *
 * Cues are synthesized through one restrained output chain so they remain
 * consistent, quiet, and free of abrupt oscillator clicks. Requests made in
 * the same interaction are coalesced by priority: a meaningful result cue
 * replaces the generic button tick instead of playing over it.
 */

/* global performance */

const MASTER_VOLUME = 0.16;
const SILENCE = 0.0001;

const state = {
  enabled: true,
  preferenceLoaded: false,
  context: null,
  master: null,
  noiseBuffer: null,
  resumePromise: null,
  pendingCue: null,
  deferredCue: null,
  flushQueued: false,
  clickTimer: null,
  lastPlayedAt: new Map(),
};

const CUES = {
  click: { priority: 10, throttleMs: 45, family: "click", render: renderClick },
  tab: { priority: 20, throttleMs: 80, family: "tab", render: renderTab },
  toggleOn: { priority: 30, throttleMs: 100, family: "toggle", render: renderToggleOn },
  toggleOff: { priority: 30, throttleMs: 100, family: "toggle", render: renderToggleOff },
  acknowledge: {
    priority: 40,
    throttleMs: 140,
    family: "acknowledge",
    render: renderAcknowledge,
  },
  success: { priority: 50, throttleMs: 180, family: "success", render: renderSuccess },
  error: { priority: 60, throttleMs: 180, family: "error", render: renderError },
};

function initAudioContext() {
  if (state.context && state.context.state !== "closed") return state.context;

  try {
    const AudioContext = window.AudioContext || window.webkitAudioContext;
    if (!AudioContext) return null;

    const context = new AudioContext();
    const master = context.createGain();
    const limiter = context.createDynamicsCompressor();

    master.gain.value = MASTER_VOLUME;
    limiter.threshold.value = -24;
    limiter.knee.value = 18;
    limiter.ratio.value = 6;
    limiter.attack.value = 0.003;
    limiter.release.value = 0.12;

    master.connect(limiter);
    limiter.connect(context.destination);

    state.context = context;
    state.master = master;
    state.noiseBuffer = createNoiseBuffer(context);
    return context;
  } catch (error) {
    console.warn("[Sounds] Audio feedback unavailable:", error);
    return null;
  }
}

function createNoiseBuffer(context) {
  const frameCount = Math.ceil(context.sampleRate * 0.08);
  const buffer = context.createBuffer(1, frameCount, context.sampleRate);
  const channel = buffer.getChannelData(0);

  for (let index = 0; index < frameCount; index += 1) {
    channel[index] = Math.random() * 2 - 1;
  }

  return buffer;
}

function resumeContext() {
  const context = initAudioContext();
  if (!context) return Promise.resolve(false);
  if (context.state === "running") return Promise.resolve(true);
  if (state.resumePromise) return state.resumePromise;

  state.resumePromise = context
    .resume()
    .then(() => context.state === "running")
    .catch(() => false)
    .finally(() => {
      state.resumePromise = null;
    });

  return state.resumePromise;
}

function applyEnvelope(gainParam, start, attack, duration, peak) {
  const releaseStart = Math.min(start + attack, start + duration * 0.45);
  gainParam.cancelScheduledValues(start);
  gainParam.setValueAtTime(SILENCE, start);
  gainParam.exponentialRampToValueAtTime(Math.max(peak, SILENCE), releaseStart);
  gainParam.exponentialRampToValueAtTime(SILENCE, start + duration);
}

function tone({
  start,
  duration,
  frequency,
  endFrequency = frequency,
  peak,
  type = "triangle",
  attack = 0.004,
  filterFrequency = 1800,
}) {
  const context = state.context;
  if (!context || !state.master) return;

  const oscillator = context.createOscillator();
  const filter = context.createBiquadFilter();
  const gain = context.createGain();

  oscillator.type = type;
  oscillator.frequency.setValueAtTime(frequency, start);
  oscillator.frequency.exponentialRampToValueAtTime(endFrequency, start + duration);

  filter.type = "lowpass";
  filter.frequency.setValueAtTime(filterFrequency, start);
  filter.Q.value = 0.7;

  applyEnvelope(gain.gain, start, attack, duration, peak);

  oscillator.connect(filter);
  filter.connect(gain);
  gain.connect(state.master);
  oscillator.start(start);
  oscillator.stop(start + duration + 0.01);
}

function transient({ start, duration = 0.018, peak = 0.08, frequency = 850, attack = 0.002 }) {
  const context = state.context;
  if (!context || !state.master || !state.noiseBuffer) return;

  const source = context.createBufferSource();
  const filter = context.createBiquadFilter();
  const gain = context.createGain();

  source.buffer = state.noiseBuffer;
  filter.type = "bandpass";
  filter.frequency.setValueAtTime(frequency, start);
  filter.Q.value = 0.9;
  applyEnvelope(gain.gain, start, attack, duration, peak);

  source.connect(filter);
  filter.connect(gain);
  gain.connect(state.master);
  source.start(start, 0, duration);
}

function renderClick(start) {
  transient({ start, duration: 0.016, peak: 0.08, frequency: 760 });
  tone({
    start,
    duration: 0.028,
    frequency: 430,
    endFrequency: 350,
    peak: 0.13,
    filterFrequency: 1150,
  });
}

function renderTab(start) {
  transient({ start, duration: 0.014, peak: 0.055, frequency: 980 });
  tone({
    start,
    duration: 0.042,
    frequency: 540,
    endFrequency: 455,
    peak: 0.12,
    filterFrequency: 1350,
  });
}

function renderToggleOn(start) {
  transient({ start, duration: 0.013, peak: 0.045, frequency: 820 });
  tone({ start, duration: 0.055, frequency: 350, peak: 0.1, filterFrequency: 1200 });
  tone({
    start: start + 0.018,
    duration: 0.06,
    frequency: 470,
    peak: 0.105,
    filterFrequency: 1400,
  });
}

function renderToggleOff(start) {
  transient({ start, duration: 0.013, peak: 0.04, frequency: 720 });
  tone({ start, duration: 0.052, frequency: 470, peak: 0.09, filterFrequency: 1350 });
  tone({
    start: start + 0.018,
    duration: 0.058,
    frequency: 350,
    peak: 0.095,
    filterFrequency: 1150,
  });
}

function renderAcknowledge(start) {
  transient({ start, duration: 0.018, peak: 0.05, frequency: 700 });
  tone({
    start,
    duration: 0.075,
    frequency: 380,
    endFrequency: 420,
    peak: 0.13,
    filterFrequency: 1250,
  });
}

function renderSuccess(start) {
  transient({ start, duration: 0.02, peak: 0.035, frequency: 920 });
  tone({
    start,
    duration: 0.12,
    frequency: 392,
    peak: 0.12,
    type: "sine",
    filterFrequency: 1500,
  });
  tone({
    start: start + 0.05,
    duration: 0.14,
    frequency: 587,
    peak: 0.13,
    type: "sine",
    filterFrequency: 1700,
  });
}

function renderError(start) {
  transient({ start, duration: 0.024, peak: 0.045, frequency: 430 });
  tone({
    start,
    duration: 0.14,
    frequency: 233,
    endFrequency: 196,
    peak: 0.13,
    filterFrequency: 850,
  });
  tone({
    start: start + 0.018,
    duration: 0.13,
    frequency: 277,
    endFrequency: 220,
    peak: 0.075,
    type: "sine",
    filterFrequency: 900,
  });
}

function canRender(cue) {
  const now = performance.now();
  const previous = state.lastPlayedAt.get(cue.family) || 0;
  if (now - previous < cue.throttleMs) return false;
  state.lastPlayedAt.set(cue.family, now);
  return true;
}

function renderCue(cue) {
  if (!state.enabled || !state.preferenceLoaded) return;

  const context = initAudioContext();
  if (!context) return;

  if (context.state !== "running") {
    if (!state.deferredCue || cue.priority >= state.deferredCue.priority) {
      state.deferredCue = cue;
    }
    void resumeContext().then((ready) => {
      const deferred = state.deferredCue;
      state.deferredCue = null;
      if (ready && deferred && state.enabled && canRender(deferred)) {
        deferred.render(context.currentTime + 0.004);
      }
    });
    return;
  }

  if (canRender(cue)) {
    cue.render(context.currentTime + 0.004);
  }
}

function flushPendingCue() {
  if (state.clickTimer !== null) {
    clearTimeout(state.clickTimer);
    state.clickTimer = null;
  }
  state.flushQueued = false;
  const cue = state.pendingCue;
  state.pendingCue = null;
  if (cue) renderCue(cue);
}

function requestCue(name) {
  if (!state.enabled || !state.preferenceLoaded) return;
  const cue = CUES[name];
  if (!cue) return;

  if (!state.pendingCue || cue.priority >= state.pendingCue.priority) {
    state.pendingCue = cue;
  }

  if (name === "click") {
    if (state.clickTimer === null && !state.flushQueued) {
      // Leave a short arbitration window for async tab guards and component
      // handlers to replace the generic tick with a more meaningful cue.
      state.clickTimer = window.setTimeout(flushPendingCue, 24);
    }
  } else if (!state.flushQueued) {
    if (state.clickTimer !== null) {
      clearTimeout(state.clickTimer);
      state.clickTimer = null;
    }
    state.flushQueued = true;
    void Promise.resolve().then(flushPendingCue);
  }
}

export function playClick() {
  requestCue("click");
}

export function playTabSwitch() {
  requestCue("tab");
}

export function playToggleOn() {
  requestCue("toggleOn");
}

export function playToggleOff() {
  requestCue("toggleOff");
}

export function playAcknowledge() {
  requestCue("acknowledge");
}

export function playSuccess() {
  requestCue("success");
}

export function playError() {
  requestCue("error");
}

export function playSound(soundType) {
  const cueNames = {
    click: "click",
    tab_switch: "tab",
    tabSwitch: "tab",
    toggle_on: "toggleOn",
    toggle_off: "toggleOff",
    acknowledge: "acknowledge",
    submitted: "acknowledge",
    success: "success",
    error: "error",
  };
  requestCue(cueNames[soundType]);
}

export function setSoundsEnabled(enabled) {
  const nextEnabled = Boolean(enabled);
  const context = state.context;
  if (
    !nextEnabled &&
    state.enabled &&
    state.preferenceLoaded &&
    context?.state === "running" &&
    canRender(CUES.toggleOff)
  ) {
    CUES.toggleOff.render(context.currentTime + 0.004);
  }

  state.enabled = nextEnabled;
  state.preferenceLoaded = true;
  if (!state.enabled) {
    state.pendingCue = null;
    state.deferredCue = null;
    if (state.clickTimer !== null) {
      clearTimeout(state.clickTimer);
      state.clickTimer = null;
    }
  } else {
    void resumeContext().then((ready) => {
      if (ready && state.enabled) requestCue("toggleOn");
    });
  }

  void saveSoundPreference();
  return state.enabled;
}

export function isSoundsEnabled() {
  return state.enabled;
}

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
  } catch {
    // Sound preference failure must not interrupt the dashboard.
  }
}

export async function loadSoundPreference() {
  try {
    const response = await fetch("/api/config/gui");
    if (response.ok) {
      const result = await response.json();
      const guiConfig = result.data?.data || result.data || result;
      const config = guiConfig?.dashboard?.interface;
      state.enabled = config?.sounds_enabled !== false;
    }
  } catch {
    state.enabled = true;
  } finally {
    state.preferenceLoaded = true;
  }
}

function initOnInteraction() {
  const handler = () => {
    document.removeEventListener("click", handler);
    document.removeEventListener("keydown", handler);
    if (state.enabled) void resumeContext();
  };

  document.addEventListener("click", handler);
  document.addEventListener("keydown", handler);
}

void loadSoundPreference();
initOnInteraction();

export default {
  playSound,
  playClick,
  playTabSwitch,
  playToggleOn,
  playToggleOff,
  playAcknowledge,
  playSuccess,
  playError,
  setSoundsEnabled,
  isSoundsEnabled,
  loadSoundPreference,
};
