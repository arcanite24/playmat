import { describe, expect, it } from 'vitest';
import type { GameView, PermanentView } from '../../protocol/generated/views';
import { deriveInteraction, type Interaction } from './interaction';
import { predict, withPredictedAttack, withPrediction } from './prediction';

const land = (id: string, rules: string[] = ['{T}: Add {G}.'], tapped = false): PermanentView => ({ id, name: id, cardTypes: ['LAND'], rules, tapped }) as PermanentView;
const bear: PermanentView = { id: 'bear', name: 'Bear', cardTypes: ['CREATURE'] } as PermanentView;

function viewWith(permanents: PermanentView[], extra: Partial<GameView> = {}): GameView {
  return {
    myPlayerId: 'me',
    players: [
      { playerId: 'me', battlefield: Object.fromEntries(permanents.map((permanent) => [permanent.id!, permanent])) },
      { playerId: 'them', battlefield: { theirs: land('theirs') } },
    ],
    ...extra,
  } as GameView;
}

function clicking(mode: Interaction['mode'], ids: string[], selected: string[] = []): Interaction {
  const base = deriveInteraction(null, null);
  return { ...base, mode, clickable: new Map(ids.map((id) => [id, { type: 'uuid', id }])), selected: new Set(selected) };
}

describe('predict', () => {
  it('taps a land that only makes one kind of mana, paying or holding priority', () => {
    const view = viewWith([
      land('forest'),
      { ...land('plains', []), subTypes: ['PLAINS'] } as PermanentView,
      land('gate', ['Selesnya Guildgate enters tapped.', '<i>({T}: Add {W}.)</i>']),
    ]);
    expect(predict(view, clicking('payMana', ['forest']), 'forest')).toEqual({ id: 'forest', tapped: true });
    expect(predict(view, clicking('priority', ['plains']), 'plains')).toEqual({ id: 'plains', tapped: true });
    expect(predict(view, clicking('priority', ['gate']), 'gate')).toEqual({ id: 'gate', tapped: true });
  });

  it('only presses a land that asks a question first, a tapped land, or one that is not ours', () => {
    const view = viewWith([
      land('foundry', ['As Sacred Foundry enters, you may pay 2 life.', '({T}: Add {R} or {W}.)']),
      land('city', ['{T}: Add one mana of any color.']),
      land('mutavault', ['{T}: Add {C}.', '{1}: Until end of turn, Mutavault becomes a 2/2 creature.']),
      land('fetch', ['{T}, Pay 1 life, Sacrifice Flooded Strand: Search your library for a Plains or Island card.']),
      land('tapped', [], true),
    ]);
    for (const id of ['foundry', 'city', 'mutavault', 'fetch', 'tapped', 'theirs']) {
      expect(predict(view, clicking('priority', [id]), id)).toEqual({ id });
    }
  });

  it('steps an attacker forward, or back when it was already attacking', () => {
    const view = viewWith([bear]);
    expect(predict(view, clicking('declareAttackers', ['bear']), 'bear')).toEqual({ id: 'bear', attack: true, tapped: true });
    const attacking = viewWith([bear], { combat: [{ defenderId: 'them', attackers: { bear }, blockers: {} }] } as Partial<GameView>);
    expect(predict(attacking, clicking('declareAttackers', ['bear'], ['bear']), 'bear')).toEqual({ id: 'bear', attack: false, tapped: false });
    // vigilance: it attacks without tapping
    const knight = { ...bear, id: 'knight', rules: ['Vigilance'] } as PermanentView;
    expect(predict(viewWith([knight]), clicking('declareAttackers', ['knight']), 'knight')).toEqual({ id: 'knight', attack: true });
    const angel = { ...bear, id: 'angel', rules: ['Flying, vigilance'] } as PermanentView;
    expect(predict(viewWith([angel]), clicking('declareAttackers', ['angel']), 'angel')).toEqual({ id: 'angel', attack: true });
  });

  it('predicts nothing for what is not clickable, and only a press for targets', () => {
    const view = viewWith([land('forest')]);
    expect(predict(view, clicking('priority', []), 'forest')).toBeNull();
    expect(predict(view, clicking('target', ['forest']), 'forest')).toEqual({ id: 'forest' });
  });
});

describe('drawing a prediction', () => {
  it('taps the land in a copy of the view, leaving other players alone', () => {
    const view = viewWith([land('forest')]);
    const drawn = withPrediction(view, { id: 'forest', tapped: true })!;
    expect(drawn.players![0].battlefield!.forest.tapped).toBe(true);
    expect(view.players![0].battlefield!.forest.tapped).toBe(false);
    expect(drawn.players![1]).toBe(view.players![1]);
    expect(withPrediction(view, { id: 'forest' })).toBe(view);
  });

  it('adds or removes a predicted attacker, keeping the set when nothing changes', () => {
    const none = new Set<string>();
    expect([...withPredictedAttack(none, { id: 'bear', attack: true })]).toEqual(['bear']);
    expect(withPredictedAttack(new Set(['bear']), { id: 'bear', attack: false }).size).toBe(0);
    const already = new Set(['bear']);
    expect(withPredictedAttack(already, { id: 'bear', attack: true })).toBe(already);
    expect(withPredictedAttack(none, null)).toBe(none);
  });
});
