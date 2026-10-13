import { describe, expect, it } from 'vitest';
import type { CardView, GameView, PermanentView, PlayerView, PhaseStep } from '../../protocol/generated/views';
import { positionScore, reviewGame, turnSpans, type ReviewFrame } from './review';

function creature(id: string, name: string, power: number, toughness: number, extra: Partial<PermanentView> = {}): PermanentView {
  return { id, name, power: String(power), toughness: String(toughness), cardTypes: ['CREATURE'], ...extra } as PermanentView;
}
function land(id: string, tapped = false): PermanentView {
  return { id, name: 'Forest', cardTypes: ['LAND'], tapped } as PermanentView;
}
function seat(name: string, life: number, perms: PermanentView[], handCount = 3): PlayerView {
  return { name, life, handCount, battlefield: Object.fromEntries(perms.map((perm) => [perm.id!, perm])) } as PlayerView;
}
function frame(turn: number, active: string, step: PhaseStep, players: PlayerView[], extra: Partial<GameView> = {}, myHand: CardView[] | null = null): ReviewFrame {
  return {
    view: { turn, activePlayerName: active, step, players, ...extra } as GameView,
    myHand: myHand ? Object.fromEntries(myHand.map((card) => [card.id!, card])) : null,
  };
}

describe('post-game review', () => {
  it('groups frames into turns', () => {
    const frames = [frame(0, 'Me', 'UNTAP', []), frame(1, 'Me', 'UPKEEP', []), frame(1, 'Me', 'DRAW', []), frame(2, 'Foe', 'UPKEEP', [])];
    expect(turnSpans(frames)).toEqual([
      { turn: 1, active: 'Me', first: 1, last: 2 },
      { turn: 2, active: 'Foe', first: 3, last: 3 },
    ]);
  });

  it('finds lethal left on the board', () => {
    const attackers = [creature('a', 'Hill Giant', 3, 3), creature('b', 'Grizzly Bears', 2, 2), creature('c', 'Runeclaw Bear', 2, 2)];
    const frames = [
      frame(3, 'Me', 'PRECOMBAT_MAIN', [seat('Me', 20, attackers), seat('Foe', 4, [creature('x', 'Wall', 0, 4)])]),
      frame(3, 'Me', 'DECLARE_ATTACKERS', [seat('Me', 20, attackers), seat('Foe', 4, [creature('x', 'Wall', 0, 4)])]),
      frame(3, 'Me', 'END_TURN', [seat('Me', 20, attackers), seat('Foe', 2, [creature('x', 'Wall', 0, 4)])]),
    ];
    const review = reviewGame(frames, 'Me');
    const missed = review.findings.find((finding) => finding.kind === 'missedLethal');
    expect(missed).toMatchObject({ turn: 3, frame: 1 });
    expect(missed!.detail).toContain('Foe was at 4');
  });

  it('doesn\'t count creatures that can\'t attack yet', () => {
    const frames = [
      frame(3, 'Me', 'BEGIN_COMBAT', [seat('Me', 20, [creature('a', 'Giant', 5, 5, { summoningSickness: true })]), seat('Foe', 4, [])]),
      frame(3, 'Me', 'END_TURN', [seat('Me', 20, [creature('a', 'Giant', 5, 5)]), seat('Foe', 4, [])]),
    ];
    expect(reviewGame(frames, 'Me').findings.filter((finding) => finding.kind === 'missedLethal')).toEqual([]);
  });

  it('finds mana left unspent with a play in hand', () => {
    const lands = [land('l1'), land('l2'), land('l3')];
    const giant = { id: 'h1', name: 'Hill Giant', manaValue: 3, cardTypes: ['CREATURE'] } as CardView;
    const bolt = { id: 'h2', name: 'Lightning Bolt', manaValue: 1, cardTypes: ['INSTANT'] } as CardView;
    const frames = [
      frame(5, 'Me', 'PRECOMBAT_MAIN', [seat('Me', 20, lands), seat('Foe', 20, [])], {}, [giant, bolt]),
      frame(5, 'Me', 'END_TURN', [seat('Me', 20, lands), seat('Foe', 20, [])], {}, [giant, bolt]),
    ];
    const unspent = reviewGame(frames, 'Me').findings.find((finding) => finding.kind === 'unspentMana');
    expect(unspent).toMatchObject({ turn: 5, frame: 1 });
    expect(unspent!.detail).toContain('Hill Giant');
  });

  it('finds a blocker thrown away at a high life total', () => {
    const bear = creature('b', 'Grizzly Bears', 2, 2);
    const giant = creature('g', 'Craw Wurm', 6, 4);
    const combat = [{ attackers: { g: giant }, blockers: { b: bear }, isBlocked: true }];
    const frames = [
      frame(4, 'Foe', 'UPKEEP', [seat('Me', 18, [bear]), seat('Foe', 20, [giant])]),
      frame(4, 'Foe', 'DECLARE_BLOCKERS', [seat('Me', 18, [bear]), seat('Foe', 20, [giant])], { combat }),
      frame(4, 'Foe', 'END_TURN', [seat('Me', 18, []), seat('Foe', 20, [giant])]),
    ];
    const bad = reviewGame(frames, 'Me').findings.find((finding) => finding.kind === 'badBlock');
    expect(bad).toMatchObject({ turn: 4, frame: 1 });
    expect(bad!.detail).toContain('Grizzly Bears blocked Craw Wurm');
  });

  it('names the turning point', () => {
    const army = [creature('a', 'A', 3, 3), creature('b', 'B', 3, 3), creature('c', 'C', 3, 3)];
    const frames = [
      frame(1, 'Me', 'END_TURN', [seat('Me', 20, army), seat('Foe', 20, [])]),
      frame(2, 'Foe', 'END_TURN', [seat('Me', 20, army), seat('Foe', 20, [])]),
      frame(3, 'Me', 'END_TURN', [seat('Me', 20, army), seat('Foe', 18, [])]),
      frame(4, 'Foe', 'END_TURN', [seat('Me', 12, []), seat('Foe', 18, [creature('d', 'Dragon', 6, 6)])]),
    ];
    const review = reviewGame(frames, 'Me');
    expect(review.scores.map((entry) => entry.turn)).toEqual([1, 2, 3, 4]);
    expect(positionScore(frames[3].view, 'Me')).toBeLessThan(0);
    const point = review.findings.find((finding) => finding.kind === 'turningPoint');
    expect(point).toMatchObject({ turn: 4, frame: 3 });
    expect(point!.detail).toContain('you lost 8 life');
    expect(point!.detail).toContain('you lost A, B, C');
  });
});
