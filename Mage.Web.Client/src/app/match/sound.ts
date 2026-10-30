import { useSettings } from '../stores/settings';
import { cueToPlay, type Cue } from './cueRules';

export type { Cue } from './cueRules';

/**
 * Game cues, synthesized with Web Audio: no sound files to license or load. Each cue is short and quiet, tuned to
 * sit under conversation: wood, felt, paper and small bells, felt more than heard, like cards on a mat. All cues go
 * through one bus (a gentle compressor and a short room) so they sound like one set.
 */

export interface Bus {
  ctx: AudioContext;
  input: GainNode;
  noise: AudioBuffer;
}

let bus: Bus | null = null;

/** The one audio bus every synthesized sound goes through (game cues, Career's sounds and music). */
export function audio(): Bus | null {
  if (typeof window === 'undefined' || !('AudioContext' in window)) return null;
  if (!bus) {
    const ctx = new AudioContext();
    const input = ctx.createGain();
    // a small room: a short, dark echo under the dry sound
    const room = ctx.createDelay(0.5);
    room.delayTime.value = 0.085;
    const feedback = ctx.createGain();
    feedback.gain.value = 0.28;
    const dark = ctx.createBiquadFilter();
    dark.type = 'lowpass';
    dark.frequency.value = 2400;
    const wet = ctx.createGain();
    wet.gain.value = 0.16;
    input.connect(room).connect(dark).connect(feedback).connect(room);
    dark.connect(wet);
    // keep peaks soft when cues overlap
    const glue = ctx.createDynamicsCompressor();
    glue.threshold.value = -18;
    glue.ratio.value = 3;
    glue.attack.value = 0.004;
    glue.release.value = 0.2;
    input.connect(glue);
    wet.connect(glue);
    glue.connect(ctx.destination);
    // one second of white noise, reused by every noisy cue
    const noise = ctx.createBuffer(1, ctx.sampleRate, ctx.sampleRate);
    const data = noise.getChannelData(0);
    for (let i = 0; i < data.length; i++) data[i] = Math.random() * 2 - 1;
    bus = { ctx, input, noise };
  }
  // browsers start the context suspended until a user gesture; game cues always follow one
  if (bus.ctx.state === 'suspended') void bus.ctx.resume().catch(() => undefined);
  return bus;
}

interface ToneOptions {
  freq: number;
  start: number;
  length: number;
  type?: OscillatorType;
  gain?: number;
  /** glide to this frequency over the tone */
  slide?: number;
  attack?: number;
  detune?: number;
}

export function tone({ ctx }: Bus, out: AudioNode, { freq, start, length, type = 'sine', gain = 0.3, slide, attack = 0.008, detune = 0 }: ToneOptions) {
  const osc = ctx.createOscillator();
  const env = ctx.createGain();
  osc.type = type;
  osc.detune.value = detune;
  osc.frequency.setValueAtTime(freq, start);
  if (slide) osc.frequency.exponentialRampToValueAtTime(slide, start + length);
  env.gain.setValueAtTime(0.0001, start);
  env.gain.exponentialRampToValueAtTime(gain, start + attack);
  env.gain.exponentialRampToValueAtTime(0.0001, start + length);
  osc.connect(env).connect(out);
  osc.start(start);
  osc.stop(start + length + 0.05);
}

/** A small bell: a few inharmonic partials that die away at different speeds. */
export function bell(b: Bus, out: AudioNode, freq: number, start: number, length: number, gain: number) {
  const partials: [number, number, number][] = [[1, 1, 1], [2.01, 0.42, 0.6], [3.02, 0.2, 0.4], [4.17, 0.1, 0.28]];
  for (const [ratio, level, decay] of partials) {
    tone(b, out, { freq: freq * ratio, start, length: length * decay, gain: gain * level, attack: 0.004 });
  }
}

/** Filtered noise: felt, paper, breath. */
export function noise(b: Bus, out: AudioNode, { start, length, from, to, gain = 0.25, type = 'bandpass', q = 1.2 }: {
  start: number;
  length: number;
  from: number;
  to: number;
  gain?: number;
  type?: BiquadFilterType;
  q?: number;
}) {
  const { ctx } = b;
  const source = ctx.createBufferSource();
  source.buffer = b.noise;
  const filter = ctx.createBiquadFilter();
  filter.type = type;
  filter.Q.value = q;
  filter.frequency.setValueAtTime(from, start);
  filter.frequency.exponentialRampToValueAtTime(to, start + length);
  const env = ctx.createGain();
  env.gain.setValueAtTime(0.0001, start);
  env.gain.exponentialRampToValueAtTime(gain, start + Math.min(0.02, length / 3));
  env.gain.exponentialRampToValueAtTime(0.0001, start + length);
  source.connect(filter).connect(env).connect(out);
  source.start(start, Math.random() * 0.4, length + 0.05);
}

/** A knock on wood: a quick pitched click with a little body. */
export function knock(b: Bus, out: AudioNode, freq: number, start: number, gain: number) {
  tone(b, out, { freq, start, length: 0.07, type: 'triangle', gain, attack: 0.002, slide: freq * 0.82 });
  noise(b, out, { start, length: 0.03, from: freq * 3, to: freq * 2, gain: gain * 0.5, q: 3 });
}

/** A thud on felt: low and soft, more pressure than pitch. */
export function thud(b: Bus, out: AudioNode, start: number, gain: number, freq = 95) {
  tone(b, out, { freq, start, length: 0.22, gain, attack: 0.003, slide: freq * 0.5 });
  noise(b, out, { start, length: 0.08, from: 700, to: 180, gain: gain * 0.45, type: 'lowpass', q: 0.7 });
}

export const NOTE = { C4: 261.63, D4: 293.66, E4: 329.63, G4: 392, A4: 440, Bb4: 466.16, C5: 523.25, D5: 587.33, E5: 659.25, G5: 783.99, C6: 1046.5 };

function render(b: Bus, out: AudioNode, cue: Cue, t: number) {
  switch (cue) {
    case 'turn':
      // your turn: two bells a fifth apart, rising
      bell(b, out, NOTE.C5, t, 1.1, 0.16);
      bell(b, out, NOTE.G5, t + 0.13, 1.3, 0.13);
      break;
    case 'decide':
      // a decision is yours: a light knock and a tiny bell
      knock(b, out, 820, t, 0.12);
      bell(b, out, NOTE.E5 * 2, t + 0.05, 0.35, 0.05);
      break;
    case 'cast':
      // a spell leaves the hand: breath rising into a shimmer
      noise(b, out, { start: t, length: 0.34, from: 500, to: 4200, gain: 0.12 });
      tone(b, out, { freq: NOTE.G4, start: t + 0.04, length: 0.36, gain: 0.06, slide: NOTE.G5 });
      [NOTE.C6, NOTE.E5 * 2].forEach((freq, i) => tone(b, out, { freq, start: t + 0.18 + i * 0.05, length: 0.3, gain: 0.025 }));
      break;
    case 'resolve':
      // it happens: the shimmer settles onto the mat
      tone(b, out, { freq: NOTE.E5, start: t, length: 0.3, gain: 0.05, slide: NOTE.C5 });
      noise(b, out, { start: t, length: 0.22, from: 2600, to: 500, gain: 0.07 });
      thud(b, out, t + 0.12, 0.14, 120);
      break;
    case 'land':
      // a land put down: a card on felt, low and earthy
      noise(b, out, { start: t, length: 0.06, from: 3500, to: 1500, gain: 0.06, type: 'highpass', q: 0.7 });
      thud(b, out, t + 0.03, 0.26, 80);
      break;
    case 'attack':
      // to battle: two drum hits
      thud(b, out, t, 0.34, 98);
      thud(b, out, t + 0.17, 0.3, 92);
      noise(b, out, { start: t + 0.17, length: 0.12, from: 1200, to: 300, gain: 0.08 });
      break;
    case 'block':
      // a shield: a short, hard clank over a thud
      [420, 1130, 1810].forEach((freq, i) => tone(b, out, { freq, start: t, length: 0.18 - i * 0.04, type: 'triangle', gain: 0.07 - i * 0.015, attack: 0.002 }));
      thud(b, out, t, 0.2, 110);
      break;
    case 'damage':
      // an impact: a crack and a low drop
      noise(b, out, { start: t, length: 0.12, from: 2200, to: 260, gain: 0.2, q: 0.9 });
      tone(b, out, { freq: 140, start: t, length: 0.28, gain: 0.36, slide: 52, attack: 0.002 });
      break;
    case 'lifeGain':
      // healing: three soft notes rising
      [NOTE.C5, NOTE.E5, NOTE.G5].forEach((freq, i) => tone(b, out, { freq, start: t + i * 0.08, length: 0.45, gain: 0.07, attack: 0.02 }));
      break;
    case 'draw':
      // a card slides off the library
      noise(b, out, { start: t, length: 0.11, from: 1800, to: 5200, gain: 0.08, type: 'highpass', q: 0.8 });
      break;
    case 'timer':
      // your clock is running out: a clock's double tick, twice
      [0, 0.12, 0.5, 0.62].forEach((offset, i) => knock(b, out, i % 2 === 0 ? 1650 : 1350, t + offset, 0.13));
      break;
    case 'trigger':
      // a triggered ability: a quick rising pair of small bells over a pinch of shimmer, brighter and
      // shorter than a spell's breath
      noise(b, out, { start: t, length: 0.16, from: 3000, to: 7000, gain: 0.05, type: 'highpass', q: 0.8 });
      bell(b, out, NOTE.E5 * 2, t + 0.01, 0.45, 0.06);
      bell(b, out, NOTE.A4 * 4, t + 0.08, 0.6, 0.055);
      break;
    case 'victory':
      // a bright arpeggio landing on a held chord
      [NOTE.C5, NOTE.E5, NOTE.G5, NOTE.C6].forEach((freq, i) => bell(b, out, freq, t + i * 0.11, 1.4, 0.11));
      [NOTE.C5, NOTE.E5, NOTE.G5].forEach((freq) => tone(b, out, { freq, start: t + 0.45, length: 1.6, type: 'triangle', gain: 0.045, attack: 0.08 }));
      break;
    case 'defeat':
      // a soft fall in minor
      [NOTE.G4, NOTE.E4 * 0.944, NOTE.C4].forEach((freq, i) => tone(b, out, { freq, start: t + i * 0.2, length: 0.9, gain: 0.12, attack: 0.03 }));
      tone(b, out, { freq: NOTE.C4 / 2, start: t + 0.4, length: 1.4, type: 'triangle', gain: 0.06, attack: 0.1 });
      break;
  }
}

/** Plays a cue if the player's sound settings allow it. */
export function playCue(cue: Cue) {
  const settings = useSettings.getState().settings;
  const played = cueToPlay(cue, settings);
  if (!played) return;
  previewCue(played, settings.volume);
}

/** Plays a cue regardless of the on/off settings (the volume preview in Settings). */
export function previewCue(cue: Cue, volume = useSettings.getState().settings.volume) {
  const b = audio();
  if (!b || volume <= 0) return;
  const out = b.ctx.createGain();
  // a curve that feels even across the slider
  out.gain.value = volume * volume * 0.9;
  out.connect(b.input);
  render(b, out, cue, b.ctx.currentTime + 0.01);
  setTimeout(() => out.disconnect(), 2500);
}
