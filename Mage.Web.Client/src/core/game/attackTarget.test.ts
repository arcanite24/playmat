import { describe, expect, it } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import { attackDropTargets, defaultDefender } from './attackTarget';
import type { DefenderOption } from './multiplayer';

const option = (id: string, kind: DefenderOption['kind']): DefenderOption => ({ id, kind, name: id, value: null, controllerName: null });

describe('defaultDefender', () => {
  it('attacks the one opponent', () => {
    expect(defaultDefender([option('opp', 'player'), option('jace', 'planeswalker')])).toBe('opp');
  });
  it('leaves a choice of players to the player', () => {
    expect(defaultDefender([option('a', 'player'), option('b', 'player')])).toBeNull();
    expect(defaultDefender([option('jace', 'planeswalker')])).toBeNull();
  });
});

describe('attackDropTargets', () => {
  it('offers the other players\' planeswalkers and every battle', () => {
    const view = {
      players: [
        { playerId: 'me', battlefield: { mine: { cardTypes: ['PLANESWALKER'] }, siege: { cardTypes: ['BATTLE'] } } },
        { playerId: 'opp', battlefield: { jace: { cardTypes: ['PLANESWALKER'] }, bear: { cardTypes: ['CREATURE'] } } },
      ],
    } as unknown as GameView;
    expect([...attackDropTargets(view, 'me')].sort()).toEqual(['jace', 'siege']);
  });
});
