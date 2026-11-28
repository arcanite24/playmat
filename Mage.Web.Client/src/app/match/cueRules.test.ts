import { describe, expect, it } from 'vitest';
import { cueAllowed, cueToPlay, pickCue, type CueSnapshot } from './cueRules';

const base: CueSnapshot = {
  turn: 3, active: 'opp', deciding: false, stack: 0, hand: 5, life: new Map([['me', 20], ['opp', 20]]),
  myLands: 3, attackers: 0, blockers: 0, myClock: null, over: false, won: false,
};
const next = (patch: Partial<CueSnapshot>): CueSnapshot => ({ ...base, ...patch });

describe('pickCue', () => {
  it('stays silent on the first view', () => {
    expect(pickCue(null, base, 'me', false)).toBeNull();
    expect(pickCue(next({ turn: 0 }), base, 'me', false)).toBeNull();
  });

  it('announces the result over everything else', () => {
    expect(pickCue(base, next({ over: true, won: true, stack: 2 }), 'me', false)).toBe('victory');
    expect(pickCue(base, next({ over: true }), 'me', false)).toBe('defeat');
    expect(pickCue(next({ over: true }), next({ over: true, stack: 1 }), 'me', false)).toBeNull();
  });

  it('marks your turn, not theirs', () => {
    expect(pickCue(base, next({ turn: 4, active: 'me' }), 'me', false)).toBe('turn');
    expect(pickCue(base, next({ turn: 4, active: 'opp2' }), 'me', false)).toBeNull();
  });

  it('warns once when your clock runs low', () => {
    expect(pickCue(next({ myClock: 14 }), next({ myClock: 9 }), 'me', false)).toBe('timer');
    expect(pickCue(next({ myClock: 9 }), next({ myClock: 8 }), 'me', false)).toBeNull();
    expect(pickCue(base, next({ myClock: 30 }), 'me', false)).toBeNull();
  });

  it('hears combat, spells and their results', () => {
    expect(pickCue(base, next({ attackers: 2 }), 'me', false)).toBe('attack');
    expect(pickCue(next({ attackers: 2 }), next({ attackers: 2, blockers: 1 }), 'me', false)).toBe('block');
    expect(pickCue(base, next({ stack: 1 }), 'me', false)).toBe('cast');
    expect(pickCue(next({ stack: 1 }), next({ stack: 0 }), 'me', false)).toBe('resolve');
    expect(pickCue(next({ stack: 1 }), next({ stack: 0, life: new Map([['me', 20], ['opp', 17]]) }), 'me', false)).toBe('damage');
    expect(pickCue(base, next({ life: new Map([['me', 23], ['opp', 20]]) }), 'me', false)).toBe('lifeGain');
  });

  it('chimes for a new trigger instead of the spell cue', () => {
    expect(pickCue(base, next({ stack: 1, triggers: ['a1'] }), 'me', false)).toBe('trigger');
    expect(pickCue(next({ stack: 1, triggers: ['a1'] }), next({ stack: 2, triggers: ['a1'] }), 'me', false)).toBe('cast');
    expect(pickCue(next({ stack: 1, triggers: ['a1'] }), next({ stack: 1, triggers: ['a1'] }), 'me', false)).toBeNull();
  });

  it('hears lands and draws within a turn', () => {
    expect(pickCue(base, next({ myLands: 4, hand: 4 }), 'me', false)).toBe('land');
    expect(pickCue(base, next({ hand: 6 }), 'me', false)).toBe('draw');
  });

  it('chimes for a decision unless the client answers it', () => {
    expect(pickCue(base, next({ deciding: true }), 'me', false)).toBe('decide');
    expect(pickCue(base, next({ deciding: true }), 'me', true)).toBeNull();
  });
});

describe('cueAllowed', () => {
  it('follows the sound settings', () => {
    expect(cueAllowed('draw', { sound: true, volume: 0.5 })).toBe(true);
    expect(cueAllowed('draw', { sound: false, volume: 0.5 })).toBe(false);
    expect(cueAllowed('draw', { sound: true, volume: 0 })).toBe(false);
    expect(cueAllowed('draw', { sound: true, volume: 0.5, importantCuesOnly: true })).toBe(false);
    expect(cueAllowed('turn', { sound: true, volume: 0.5, importantCuesOnly: true })).toBe(true);
  });
});

describe('cueToPlay', () => {
  it('plays the trigger chime unless "Trigger sounds" is off, then the spell cue', () => {
    expect(cueToPlay('trigger', { sound: true, volume: 0.5, triggerSounds: true })).toBe('trigger');
    expect(cueToPlay('trigger', { sound: true, volume: 0.5, triggerSounds: false })).toBe('cast');
    expect(cueToPlay('trigger', { sound: false, volume: 0.5, triggerSounds: true })).toBeNull();
    expect(cueToPlay('trigger', { sound: true, volume: 0.5, importantCuesOnly: true })).toBeNull();
  });
});
