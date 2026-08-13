/**
 * Demo capture audio bus.
 *
 * Music and one-off sound effects for a promotional recording, mixed into the
 * dashboard's own audio graph (see `core/sounds.js`) rather than a second
 * AudioContext — that is what lets the interaction cues, the music and the
 * narration be captured as ONE stream by the recorder.
 *
 * Volume moves are scheduled ramps on the audio clock, so `setVolume` resolves
 * exactly when the ramp it scheduled has finished. No fade is ever guessed.
 */

const assetVersion = window.__ASSET_VERSION__ || "";
const assetQuery = assetVersion ? `?v=${encodeURIComponent(assetVersion)}` : "";

const sounds = await import(`../core/sounds.js${assetQuery}`);

const buses = new Map();
const players = new Map();

/**
 * Media files live on disk beside the scene, not in the binary. The Electron
 * demo bridge serves them over its own local origin and tells the runtime where.
 */
function mediaUrl(file) {
  const base = window.__SB_DEMO_MEDIA_BASE__;
  if (/^https?:/i.test(file)) return file;
  if (!base) throw new Error("No demo media base URL — is the Electron demo bridge running?");
  return `${base}/${encodeURIComponent(file)}`;
}

function graph() {
  const audio = sounds.ensureAudioGraph();
  if (!audio) throw new Error("Audio graph unavailable in this renderer");
  return audio;
}

/** One gain node per named bus ("music", "sfx"), created on first use. */
function bus(name) {
  if (buses.has(name)) return buses.get(name);
  const { context, outputBus } = graph();
  const gain = context.createGain();
  gain.gain.value = 0;
  gain.connect(outputBus);
  buses.set(name, gain);
  return gain;
}

/** Schedule a linear ramp and resolve when the audio clock has passed its end. */
function ramp(param, value, fadeMs, context) {
  const seconds = Math.max(0, fadeMs) / 1000;
  const now = context.currentTime;
  param.cancelScheduledValues(now);
  param.setValueAtTime(param.value, now);
  param.linearRampToValueAtTime(Math.max(0, value), now + seconds);
  return new Promise((resolve) => setTimeout(resolve, seconds * 1000));
}

/**
 * Start a music bed. Resolves once playback has actually begun and the fade-in
 * has completed — a scene that continues after this can rely on the bed being
 * at its stated level.
 */
export async function playMusic({
  file,
  volume = 0.35,
  loop = true,
  fadeMs = 800,
  bus: busName = "music",
} = {}) {
  await sounds.resumeAudio();
  const { context } = graph();
  const target = bus(busName);

  await stopMusic({ bus: busName, fadeMs: fadeMs > 0 ? 200 : 0 });

  const element = new Audio(mediaUrl(file));
  element.crossOrigin = "anonymous";
  element.loop = Boolean(loop);
  element.preload = "auto";

  await new Promise((resolve, reject) => {
    element.addEventListener("canplaythrough", resolve, { once: true });
    element.addEventListener(
      "error",
      () => reject(new Error(`Demo media failed to load: ${file}`)),
      { once: true }
    );
    element.load();
  });

  const source = context.createMediaElementSource(element);
  source.connect(target);
  players.set(busName, { element, source });

  target.gain.value = 0;
  await element.play();
  await ramp(target.gain, volume, fadeMs, context);

  return { file, volume, loop, bus: busName, duration: element.duration };
}

/** Fade a bus to a new level. `bus: "cues"` moves the app's interaction sounds. */
export async function setVolume({ bus: busName = "music", value = 0.35, fadeMs = 400 } = {}) {
  if (busName === "cues") {
    sounds.setCueVolume(value, fadeMs);
    await new Promise((resolve) => setTimeout(resolve, Math.max(0, fadeMs)));
    return { bus: busName, value };
  }

  const { context } = graph();
  await ramp(bus(busName).gain, value, fadeMs, context);
  return { bus: busName, value };
}

/** Fade out and release a bus's player. Safe to call when nothing is playing. */
export async function stopMusic({ bus: busName = "music", fadeMs = 600 } = {}) {
  const player = players.get(busName);
  if (!player) return { bus: busName, stopped: false };

  const { context } = graph();
  await ramp(bus(busName).gain, 0, fadeMs, context);

  player.element.pause();
  player.source.disconnect();
  players.delete(busName);
  return { bus: busName, stopped: true };
}

/**
 * A one-off sound. `cue` plays one of the dashboard's own interaction sounds;
 * `file` plays an audio file from the scene's media directory. Resolves when the
 * sound has finished, so narration lines can simply be awaited in order.
 */
export async function playSfx({ cue = null, file = null, volume = 0.8 } = {}) {
  await sounds.resumeAudio();

  if (cue) {
    sounds.playSound(cue);
    return { cue };
  }
  if (!file) throw new Error("playSfx needs a cue or a file");

  const { context } = graph();
  const element = new Audio(mediaUrl(file));
  element.crossOrigin = "anonymous";

  await new Promise((resolve, reject) => {
    element.addEventListener("canplaythrough", resolve, { once: true });
    element.addEventListener(
      "error",
      () => reject(new Error(`Demo media failed to load: ${file}`)),
      { once: true }
    );
    element.load();
  });

  const gain = context.createGain();
  gain.gain.value = volume;
  context.createMediaElementSource(element).connect(gain);
  gain.connect(bus("sfx"));
  bus("sfx").gain.value = 1;

  await element.play();
  await new Promise((resolve) => element.addEventListener("ended", resolve, { once: true }));
  gain.disconnect();

  return { file, duration: element.duration };
}
