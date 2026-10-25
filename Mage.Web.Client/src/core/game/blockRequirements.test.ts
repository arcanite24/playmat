import { describe, expect, it } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import { blockWarning } from './blockRequirements';
import type { DeclareBlockersPrompt } from './prompt';

const fear = { id: 'a1', name: 'Fear of Being Hunted', rules: ['Haste', 'This creature must be blocked if able.'] };
const bear = { id: 'a2', name: 'Bear', rules: [] };
const wall = { id: 'b1', name: 'Wall', rules: [] };

function view(blockers: Record<string, string>): GameView {
  return {
    myPlayerId: 'me',
    players: [{ playerId: 'me', battlefield: { b1: wall } }],
    combat: [
      { attackers: { a1: fear }, blockers: Object.fromEntries(Object.entries(blockers).filter(([, a]) => a === 'a1').map(([b]) => [b, wall])) },
      { attackers: { a2: bear }, blockers: Object.fromEntries(Object.entries(blockers).filter(([, a]) => a === 'a2').map(([b]) => [b, wall])) },
    ],
  } as unknown as GameView;
}
const prompt = { kind: 'declareBlockers', possibleBlockers: ['b1'] } as unknown as DeclareBlockersPrompt;

describe('blockWarning', () => {
  it('warns when a must-be-blocked attacker is left unblocked', () => {
    expect(blockWarning(view({}), prompt)?.name).toBe('Fear of Being Hunted');
    expect(blockWarning(view({ b1: 'a2' }), { ...prompt, possibleBlockers: [] })?.name).toBe('Fear of Being Hunted');
  });
  it('accepts blocks that meet the requirement', () => {
    expect(blockWarning(view({ b1: 'a1' }), { ...prompt, possibleBlockers: [] })).toBeNull();
  });
  it('knows a flier needs flying or reach', () => {
    const flier = { ...fear, rules: ['Flying', 'This creature must be blocked if able.'] };
    const v = view({});
    v.combat![0].attackers = { a1: flier };
    expect(blockWarning(v, prompt)).toBeNull();
  });
});
