import { useEffect, useRef } from 'react';
import type { GameSessionState } from '../../core/game/gameSession';
import { useSettings } from '../stores/settings';
import { notify } from '../stores/toasts';
import { ignoreMatchKey, matchKeysBlocked } from './keyboard';
import { cueSnapshot, pickCue, type CueSnapshot } from './cueRules';
import { playCue } from './sound';
import { beatsOf } from './beats';
import type { Cue } from './cueRules';

/**
 * Plays a cue when something worth hearing happens: your turn, a decision, spells, combat, damage and healing,
 * lands, draws, your clock running low, the result. M mutes and unmutes.
 */
export function useGameCues(state: GameSessionState, myId: string | null, autoPassing = false) {
  const previous = useRef<CueSnapshot | null>(null);
  useEffect(() => {
    const now = cueSnapshot(state, myId);
    const cue = pickCue(previous.current, now, myId, autoPassing);
    previous.current = now;
    if (cue) playCue(cue, cueDelay(cue, state));
  }, [state, myId, autoPassing]);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key !== 'm' && event.key !== 'M') return;
      if (event.metaKey || event.ctrlKey || event.altKey || ignoreMatchKey(event) || matchKeysBlocked()) return;
      const { settings, update } = useSettings.getState();
      update({ sound: !settings.sound });
      notify(settings.sound ? 'Sound off' : 'Sound on', 'Press M to switch it back.');
    }
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, []);
}

/**
 * When a cue plays: on the beat of what it belongs to in the effects drawn for the same view (beats.ts). Vfx sits
 * below the match stage, so its effect has noted the view's beats before this hook's effect runs. A hit sounds when
 * it lands, a card put down when its flight ends.
 */
function cueDelay(cue: Cue, state: GameSessionState): number {
  const beats = beatsOf(state.view);
  switch (cue) {
    case 'damage':
      return beats.impact;
    case 'land':
      return beats.land;
    case 'resolve':
      // a permanent spell resolves as it lands on the battlefield
      return beats.land || beats.impact;
    default:
      return 0;
  }
}
