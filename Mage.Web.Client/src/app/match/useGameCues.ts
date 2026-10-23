import { useEffect, useRef } from 'react';
import type { GameSessionState } from '../../core/game/gameSession';
import { useSettings } from '../stores/settings';
import { notify } from '../stores/toasts';
import { ignoreMatchKey, matchKeysBlocked } from './keyboard';
import { cueSnapshot, pickCue, type CueSnapshot } from './cueRules';
import { playCue } from './sound';

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
    if (cue) playCue(cue);
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
