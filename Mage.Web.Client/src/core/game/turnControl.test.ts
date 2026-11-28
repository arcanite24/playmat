import { describe, expect, it } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import { autoAnswer } from './autoPass';
import { myAttackers, passLabel } from './interaction';
import { actingPlayerId, controlledTurn } from './turnControl';

const me = { playerId: 'me', name: 'Noop' };
const them = { playerId: 'ai', name: 'Bot' };
const alarm = { id: 'c1', name: 'Raise the Alarm', manaCostLeftStr: ['{1}', '{W}'] };

/** Their turn, which we control (Emrakul): the server lists their hand for us, by name. */
function controlled(priority: string, extra: Partial<GameView> = {}): GameView {
  return {
    myPlayerId: 'me',
    players: [me, them],
    activePlayerId: 'ai',
    activePlayerName: 'Bot',
    priorityPlayerName: priority,
    step: 'PRECOMBAT_MAIN',
    opponentHands: { Bot: { c1: alarm as never } },
    ...extra,
  };
}

describe('turn control', () => {
  it('finds the controlled player and their hand, as whole cards', () => {
    const turn = controlledTurn(controlled('Bot'));
    expect(turn?.player.playerId).toBe('ai');
    expect(turn?.acting).toBe(true);
    expect(turn?.hand.map((card) => card.name)).toEqual(['Raise the Alarm']);
    expect(actingPlayerId(controlled('Bot'))).toBe('ai');
  });

  it('acts as ourselves when we hold priority on their turn', () => {
    expect(controlledTurn(controlled('Noop'))?.acting).toBe(false);
    expect(actingPlayerId(controlled('Noop'))).toBe('me');
  });

  it('controls nobody without a listed hand', () => {
    expect(controlledTurn({ myPlayerId: 'me', players: [me, them], opponentHands: {} })).toBeNull();
    expect(actingPlayerId({ myPlayerId: 'me', players: [me, them] })).toBe('me');
  });

  it('plays their turn as our own: pass labels, auto-pass and attackers', () => {
    expect(passLabel(controlled('Bot'))).toBe('To Combat');
    expect(passLabel(controlled('Noop'))).toBe('Pass');
    // their spell is castable: the client doesn't pass it away
    const castable = controlled('Bot', { canPlayObjects: { objects: { c1: { basicCastAbilities: ['x'] } } } as never });
    expect(autoAnswer(castable, { kind: 'priority', text: '' } as never, { autoPass: true, autoSkipCombat: true, abilitiesOnTheirTurn: false })).toBeNull();
    // their declared attackers are the ones we chose
    const attacking = controlled('Bot', { step: 'DECLARE_ATTACKERS', combat: [{ attackers: { a1: { id: 'a1', controllerId: 'ai' } } }] as never });
    expect(myAttackers(attacking)).toEqual(['a1']);
  });
});
