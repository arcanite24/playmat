import { audio, bell, knock, noise, NOTE, thud, tone, type Bus } from '../match/sound';
import { useSettings } from '../stores/settings';

/**
 * Career's sounds, synthesized on the same bus as the game cues so they sound like one set: coins, levels, unlocks,
 * packs and cards. Short and quiet like the rest; the Sound settings (on/off, volume) apply to them too.
 */
export type CareerCue =
  | 'tap' | 'whoosh' | 'coin' | 'coins' | 'xp' | 'levelUp' | 'unlock' | 'tear' | 'flip' | 'rare' | 'mythic' | 'versus' | 'win' | 'loss'
  // the capsule machines (career/capsules)
  | 'tokenIn' | 'crank' | 'drop' | 'twist' | 'pop' | 'setDone';

function render(b: Bus, out: AudioNode, cue: CareerCue, t: number) {
  switch (cue) {
    case 'tap':
      // a finger on a felt button
      knock(b, out, 640, t, 0.07);
      break;
    case 'whoosh':
      // a card slid across the mat to another screen
      noise(b, out, { start: t, length: 0.24, from: 900, to: 3400, gain: 0.05, type: 'bandpass', q: 0.8 });
      break;
    case 'coin':
      // one coin on the table: a bright, short ring
      [2637, 3951].forEach((freq, i) => tone(b, out, { freq, start: t, length: 0.18 - i * 0.05, type: 'triangle', gain: 0.05 - i * 0.02, attack: 0.002 }));
      break;
    case 'coins':
      // a little pile landing, coin after coin
      [0, 0.07, 0.15, 0.21, 0.3].forEach((offset, i) => {
        const freq = [2637, 2489, 2794, 2349, 2637][i];
        tone(b, out, { freq, start: t + offset, length: 0.16, type: 'triangle', gain: 0.045, attack: 0.002 });
        tone(b, out, { freq: freq * 1.5, start: t + offset, length: 0.1, type: 'triangle', gain: 0.018, attack: 0.002 });
      });
      break;
    case 'xp':
      // a soft rising shimmer while the bar fills
      noise(b, out, { start: t, length: 0.5, from: 1800, to: 6000, gain: 0.035, type: 'highpass', q: 0.6 });
      tone(b, out, { freq: NOTE.E5, start: t, length: 0.5, gain: 0.03, slide: NOTE.E5 * 2 });
      break;
    case 'levelUp':
      // a bright fanfare in bells, landing on a held chord
      [NOTE.G4, NOTE.C5, NOTE.E5, NOTE.G5, NOTE.C6].forEach((freq, i) => bell(b, out, freq, t + i * 0.09, 1.2, 0.1));
      [NOTE.C5, NOTE.E5, NOTE.G5].forEach((freq) => tone(b, out, { freq, start: t + 0.45, length: 1.8, type: 'triangle', gain: 0.04, attack: 0.1 }));
      thud(b, out, t + 0.45, 0.16, 110);
      break;
    case 'unlock':
      // a lock turns, then a bell
      knock(b, out, 1200, t, 0.1);
      knock(b, out, 900, t + 0.08, 0.1);
      bell(b, out, NOTE.G5, t + 0.2, 1.1, 0.1);
      bell(b, out, NOTE.C6, t + 0.32, 1.3, 0.08);
      break;
    case 'tear':
      // a pack's wrapper torn open, left to right
      noise(b, out, { start: t, length: 0.42, from: 2600, to: 7000, gain: 0.11, type: 'highpass', q: 0.9 });
      noise(b, out, { start: t + 0.05, length: 0.3, from: 800, to: 2200, gain: 0.05, q: 2 });
      break;
    case 'flip':
      // a card turned over
      noise(b, out, { start: t, length: 0.09, from: 2400, to: 5200, gain: 0.07, type: 'highpass', q: 0.8 });
      knock(b, out, 520, t + 0.05, 0.04);
      break;
    case 'rare':
      // something good: two bells and a glint
      bell(b, out, NOTE.E5, t, 1, 0.1);
      bell(b, out, NOTE.G5 * 2, t + 0.1, 1.2, 0.07);
      break;
    case 'mythic':
      // the big one: a low hit, then bells climbing over a shimmer
      thud(b, out, t, 0.3, 70);
      noise(b, out, { start: t + 0.05, length: 0.9, from: 1200, to: 8000, gain: 0.05, type: 'highpass', q: 0.5 });
      [NOTE.C5, NOTE.G5, NOTE.C6, NOTE.E5 * 2].forEach((freq, i) => bell(b, out, freq, t + 0.12 + i * 0.1, 1.5, 0.09));
      break;
    case 'versus':
      // two drums and a rising swell: the duel is set
      thud(b, out, t, 0.32, 82);
      thud(b, out, t + 0.22, 0.36, 76);
      noise(b, out, { start: t + 0.2, length: 0.8, from: 300, to: 2400, gain: 0.07, type: 'lowpass', q: 0.7 });
      tone(b, out, { freq: NOTE.C4 / 2, start: t + 0.22, length: 1.2, type: 'triangle', gain: 0.07, attack: 0.3, slide: NOTE.G4 / 2 });
      break;
    case 'win':
      [NOTE.C5, NOTE.E5, NOTE.G5, NOTE.C6].forEach((freq, i) => bell(b, out, freq, t + i * 0.11, 1.4, 0.11));
      [NOTE.C5, NOTE.E5, NOTE.G5].forEach((freq) => tone(b, out, { freq, start: t + 0.45, length: 1.6, type: 'triangle', gain: 0.045, attack: 0.08 }));
      break;
    case 'tokenIn':
      // a token down the slot: a ring, then it rattles to the bottom
      tone(b, out, { freq: 3135, start: t, length: 0.12, type: 'triangle', gain: 0.04, attack: 0.002 });
      [0.08, 0.15, 0.2].forEach((offset, i) => knock(b, out, 1500 - i * 260, t + offset, 0.05 - i * 0.012));
      break;
    case 'crank':
      // one click of the crank's ratchet
      knock(b, out, 1900, t, 0.05);
      noise(b, out, { start: t, length: 0.025, from: 5200, to: 4200, gain: 0.03, type: 'highpass', q: 1 });
      break;
    case 'drop':
      // the capsule drops down the chute and bounces in the tray
      noise(b, out, { start: t, length: 0.18, from: 600, to: 1600, gain: 0.04, q: 0.9 });
      [0.16, 0.34, 0.45, 0.52].forEach((offset, i) => knock(b, out, 420 + i * 40, t + offset, 0.11 / (i + 1)));
      break;
    case 'twist':
      // plastic halves turning against each other
      noise(b, out, { start: t, length: 0.22, from: 1400, to: 900, gain: 0.05, q: 4 });
      break;
    case 'pop':
      // the capsule pops open, light spilling out
      thud(b, out, t, 0.08, 220);
      knock(b, out, 980, t, 0.08);
      noise(b, out, { start: t + 0.02, length: 0.5, from: 3000, to: 9000, gain: 0.04, type: 'highpass', q: 0.5 });
      [NOTE.C6, NOTE.G5 * 2].forEach((freq, i) => bell(b, out, freq, t + 0.08 + i * 0.07, 0.9, 0.06));
      break;
    case 'setDone':
      // a set finished: five bells, one a capsule, then the chord
      [NOTE.C5, NOTE.D5, NOTE.E5, NOTE.G5, NOTE.C6].forEach((freq, i) => bell(b, out, freq, t + i * 0.08, 1.1, 0.09));
      [NOTE.C5, NOTE.E5, NOTE.G5, NOTE.C6].forEach((freq) => tone(b, out, { freq, start: t + 0.42, length: 1.6, type: 'triangle', gain: 0.035, attack: 0.08 }));
      thud(b, out, t + 0.42, 0.14, 100);
      break;
    case 'loss':
      [NOTE.G4, NOTE.E4 * 0.944, NOTE.C4].forEach((freq, i) => tone(b, out, { freq, start: t + i * 0.2, length: 0.9, gain: 0.1, attack: 0.03 }));
      break;
  }
}

/** Plays a Career sound if the player's sound settings allow it. */
export function playCareerCue(cue: CareerCue) {
  const { sound, volume } = useSettings.getState().settings;
  if (!sound || volume <= 0) return;
  const b = audio();
  if (!b) return;
  const out = b.ctx.createGain();
  out.gain.value = volume * volume * 0.9;
  out.connect(b.input);
  render(b, out, cue, b.ctx.currentTime + 0.01);
  setTimeout(() => out.disconnect(), 2800);
}
