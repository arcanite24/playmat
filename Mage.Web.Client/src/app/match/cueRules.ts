import type { GameSessionState } from '../../core/game/gameSession';
import { stackTriggers } from './triggerSignal';

/** Every game cue. Synthesized in sound.ts; chosen here from what changed between two game states. */
export type Cue =
  | 'turn' | 'decide' | 'cast' | 'resolve' | 'land' | 'attack' | 'block' | 'damage' | 'lifeGain' | 'draw'
  | 'victory' | 'defeat' | 'timer'
  // a triggered ability went on the stack (triggerSignal.ts)
  | 'trigger';

/** The cues that still play with "Only important cues": your turn, a decision, combat against you, the clock, the result. */
export const IMPORTANT_CUES: ReadonlySet<Cue> = new Set<Cue>(['turn', 'decide', 'timer', 'victory', 'defeat']);

/** Seconds left on your clock when the warning plays. */
export const TIMER_WARNING_SECS = 10;

export interface CueSnapshot {
  turn: number;
  active: string | null;
  deciding: boolean;
  stack: number;
  /** stack ids of the triggered abilities on the stack */
  triggers?: readonly string[];
  hand: number;
  life: Map<string, number>;
  /** lands on your battlefield */
  myLands: number;
  attackers: number;
  blockers: number;
  /** seconds left on your clock while you hold priority with the timer running; null otherwise */
  myClock: number | null;
  over: boolean;
  won: boolean;
}

export function cueSnapshot(state: GameSessionState, myId: string | null): CueSnapshot {
  const view = state.view;
  const me = view?.players?.find((player) => player.playerId === myId);
  return {
    turn: view?.turn ?? 0,
    active: view?.activePlayerId ?? null,
    deciding: state.mode === 'play' && state.interaction.mode !== 'waiting' && !state.awaitingServer,
    stack: Object.keys(view?.stack ?? {}).length,
    triggers: [...stackTriggers(view).keys()],
    hand: Object.keys(view?.myHand ?? {}).length,
    life: new Map((view?.players ?? []).map((player) => [player.playerId ?? '', player.life ?? 0])),
    myLands: Object.values(me?.battlefield ?? {}).filter((card) => (card.cardTypes ?? []).includes('LAND')).length,
    attackers: (view?.combat ?? []).reduce((sum, group) => sum + Object.keys(group.attackers ?? {}).length, 0),
    blockers: (view?.combat ?? []).reduce((sum, group) => sum + Object.keys(group.blockers ?? {}).length, 0),
    myClock: me?.timerActive && me.hasPriority && (me.priorityTimeLeftSecs ?? 0) > 0 ? me.priorityTimeLeftSecs! : null,
    over: !!state.gameOver,
    won: !!state.endInfo?.won || /you won/i.test(state.gameOver ?? ''),
  };
}

/**
 * The one cue worth hearing for a change of state, or null. One at a time, most important first, so a busy update
 * (a spell resolving into damage and a draw) never plays a chord of cues.
 */
export function pickCue(before: CueSnapshot | null, now: CueSnapshot, myId: string | null, autoPassing: boolean): Cue | null {
  // the first view of a game (or a reconnect) sets the baseline silently
  if (!before || before.turn === 0) return null;
  if (now.over && !before.over) return now.won ? 'victory' : 'defeat';
  if (now.over) return null;
  if (now.turn !== before.turn && now.active && now.active === myId) return 'turn';
  if (now.myClock !== null && now.myClock <= TIMER_WARNING_SECS && (before.myClock === null || before.myClock > TIMER_WARNING_SECS)) return 'timer';
  if (now.attackers > 0 && before.attackers === 0) return 'attack';
  if (now.blockers > before.blockers) return 'block';
  if (now.stack > before.stack) {
    // a new triggered ability chimes instead of the spell cue
    const triggered = (now.triggers ?? []).some((id) => !(before.triggers ?? []).includes(id));
    return triggered ? 'trigger' : 'cast';
  }
  const lost = [...now.life].some(([id, life]) => life < (before.life.get(id) ?? life));
  if (lost) return 'damage';
  const gained = [...now.life].some(([id, life]) => life > (before.life.get(id) ?? life));
  if (gained) return 'lifeGain';
  if (now.stack < before.stack) return 'resolve';
  if (now.myLands > before.myLands && now.turn === before.turn) return 'land';
  if (now.hand > before.hand && now.turn === before.turn) return 'draw';
  // a prompt the client answers by itself isn't a decision worth a chime
  if (now.deciding && !before.deciding && !autoPassing) return 'decide';
  return null;
}

export function cueAllowed(cue: Cue, settings: { sound: boolean; volume: number; importantCuesOnly?: boolean }): boolean {
  if (!settings.sound || settings.volume <= 0) return false;
  return !settings.importantCuesOnly || IMPORTANT_CUES.has(cue);
}

/**
 * The cue that plays for a picked one under the player's settings, or null. With "Trigger sounds" off a
 * trigger sounds like any other new stack object (the spell cue, as before the trigger chime).
 */
export function cueToPlay(cue: Cue, settings: { sound: boolean; volume: number; importantCuesOnly?: boolean; triggerSounds?: boolean }): Cue | null {
  const played = cue === 'trigger' && settings.triggerSounds === false ? 'cast' : cue;
  return cueAllowed(played, settings) ? played : null;
}
