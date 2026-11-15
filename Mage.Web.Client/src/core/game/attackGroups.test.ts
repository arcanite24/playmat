import { describe, expect, test } from 'vitest';
import type { GameView, PermanentView } from '../../protocol/generated/views';
import { attackGroups, groupSize, nextAttackClick } from './attackGroups';
import { parsePrompt } from './prompt';

const ME = 'me';

function permanent(id: string, name: string, extra: Partial<PermanentView> = {}): PermanentView {
  return { id, name, power: '1', toughness: '1', isToken: true, cardTypes: ['CREATURE'], ...extra };
}

function board(permanents: PermanentView[], attacking: string[] = []): GameView {
  return {
    myPlayerId: ME,
    step: 'DECLARE_ATTACKERS',
    players: [
      { playerId: ME, name: 'Me', battlefield: Object.fromEntries(permanents.map((p) => [p.id!, p])) },
      { playerId: 'them', name: 'Them', battlefield: { x: permanent('x', 'Insect') } },
    ],
    combat: attacking.length ? [{ attackers: Object.fromEntries(attacking.map((id) => [id, { controllerId: ME }])) }] : [],
  } as GameView;
}

function attackPrompt(ids: string[]) {
  return parsePrompt('GAME_SELECT', { message: 'Select attackers', options: { possibleAttackers: ids, specialButton: 'All attack' } as never });
}

describe('attack groups', () => {
  const swarm = ['s1', 's2', 's3', 's4'].map((id) => permanent(id, 'Scute Swarm'));
  const others = [permanent('i1', 'Insect'), permanent('i2', 'Insect'), permanent('big', 'Scute Swarm', { power: '3', toughness: '3' })];

  test('identical creatures that can attack are grouped; small groups and the opponent are left out', () => {
    const view = board([...swarm, ...others]);
    const groups = attackGroups(view, attackPrompt(['s1', 's2', 's3', 's4', 'i1', 'i2', 'big', 'x']));
    expect(groups.map((group) => [group.name, group.power, group.ready])).toEqual([['Scute Swarm', '1', ['s1', 's2', 's3', 's4']]]);
    expect(attackGroups(view, attackPrompt([]))).toEqual([]);
    expect(attackGroups(view, parsePrompt('GAME_SELECT', { message: 'Play spells' }))).toEqual([]);
  });

  test('creatures already attacking count in their group', () => {
    const view = board(swarm, ['s1', 's2']);
    const [group] = attackGroups(view, attackPrompt(['s3', 's4']));
    expect(group).toMatchObject({ ready: ['s3', 's4'], attacking: ['s1', 's2'] });
    expect(groupSize(group)).toBe(4);
  });

  test('the next click declares more, or takes back the last declared, until the count is reached', () => {
    const view = board(swarm, ['s1']);
    const groups = attackGroups(view, attackPrompt(['s2', 's3', 's4']));
    const key = groups[0].key;
    expect(nextAttackClick(groups, new Map([[key, 3]]))).toBe('s2');
    expect(nextAttackClick(groups, new Map([[key, 3]]), new Set(['s2']))).toBe('s3');
    expect(nextAttackClick(groups, new Map([[key, 0]]))).toBe('s1');
    expect(nextAttackClick(groups, new Map([[key, 1]]))).toBeNull();
    expect(nextAttackClick(groups, new Map())).toBeNull();
  });
});
