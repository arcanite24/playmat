import { describe, expect, it } from 'vitest';
import type { GameView, PermanentView, PlayerView } from '../../protocol/generated/views';
import { buildBoard, buildPlayerBoard, cardKey, fitRow, ROW_FLOOR, splitOpenFirst, stackBadge, zoneKeys, type PermanentGroup, type RowFit } from './boardModel';

function permanent(id: string, overrides: Partial<PermanentView> = {}): PermanentView {
  return { id, name: 'Forest', expansionSetCode: 'M21', cardTypes: ['LAND'], ...overrides };
}

function player(id: string, permanents: PermanentView[]): PlayerView {
  return { playerId: id, name: id, battlefield: Object.fromEntries(permanents.map((p) => [p.id!, p])) };
}

describe('buildPlayerBoard', () => {
  it('puts creatures in front and lands first in the back row', () => {
    const board = buildPlayerBoard(player('me', [
      permanent('relic', { name: 'Relic', cardTypes: ['ARTIFACT'] }),
      permanent('bear', { name: 'Bear', cardTypes: ['CREATURE'], power: '2', toughness: '2' }),
      permanent('land'),
    ]), true);
    expect(board.front.map((group) => group.key)).toEqual(['bear']);
    expect(board.back.map((group) => group.key)).toEqual(['land', 'relic']);
  });

  it('stacks identical lands up to four, but never creatures or damaged permanents', () => {
    const lands = Array.from({ length: 5 }, (_, i) => permanent(`f${i}`));
    const bears = [1, 2].map((i) => permanent(`b${i}`, { name: 'Bear', cardTypes: ['CREATURE'] }));
    const board = buildPlayerBoard(player('me', [...lands, ...bears]), true);
    expect(board.back.map((group) => group.members.length)).toEqual([4, 1]);
    expect(board.front).toHaveLength(2);
  });

  it('keeps a stack together when one of its lands taps, in the same order', () => {
    const untapped = buildPlayerBoard(player('me', [permanent('a'), permanent('b')]), true);
    const tapped = buildPlayerBoard(player('me', [permanent('a'), permanent('b', { tapped: true })]), true);
    expect(tapped.back).toHaveLength(1);
    expect(tapped.back[0].key).toBe(untapped.back[0].key);
    expect(tapped.back[0].members.map((p) => p.id)).toEqual(['a', 'b']);
  });

  it('tucks attachments behind their host', () => {
    const board = buildPlayerBoard(player('me', [
      permanent('bear', { name: 'Bear', cardTypes: ['CREATURE'] }),
      permanent('aura', { name: 'Aura', cardTypes: ['ENCHANTMENT'], attachedTo: 'bear', attachedToPermanent: true }),
    ]), true);
    expect(board.front[0].attachments.map((p) => p.id)).toEqual(['aura']);
    expect(board.back).toHaveLength(0);
  });
});

describe('buildBoard', () => {
  it('seats me at the bottom and opponents in turn order after me', () => {
    const view: GameView = { players: [player('a', []), player('me', []), player('c', [])] };
    const board = buildBoard(view, 'me');
    expect(board.me?.player.playerId).toBe('me');
    expect(board.opponents.map((o) => o.player.playerId)).toEqual(['c', 'a']);
  });

  it('shows the first player at the bottom when watching', () => {
    const board = buildBoard({ players: [player('a', []), player('b', [])] }, null);
    expect(board.me?.isMe).toBe(false);
    expect(board.me?.player.playerId).toBe('a');
    expect(board.opponents.map((o) => o.player.playerId)).toEqual(['b']);
  });
});

function creatures(count: number, overrides: Partial<PermanentView> = {}): PermanentGroup[] {
  return buildPlayerBoard(player('me', Array.from({ length: count }, (_, i) => permanent(`c${i}`, { name: `C${i}`, cardTypes: ['CREATURE'], ...overrides }))), true).front;
}

/** The drawn extent of each line (stage px) and the stacked height, as the row renders a fit. */
function extent(groups: PermanentGroup[], fit: RowFit): { widths: number[]; height: number } {
  const starts = [0, ...fit.breaks, groups.length];
  const widths: number[] = [];
  let height = 0;
  for (let line = 0; line + 1 < starts.length; line++) {
    const members = groups.slice(starts[line], starts[line + 1]);
    const slots = members.reduce((sum, group) => {
      const tapped = group.members.some((member) => member.tapped);
      const offset = group.members.some((member) => member.tapped) ? 0.3 : 0.12;
      return sum + fit.cardWidth * ((tapped ? 88 / 63 : 1) + (group.members.length - 1) * offset + group.attachments.length * 0.18);
    }, 0);
    widths.push(slots + (members.length - 1) * fit.gap);
    height += fit.cardWidth * (88 / 63) + (line > 0 ? fit.lineGap : 0);
  }
  return { widths, height };
}

describe('fitRow', () => {
  it('uses the ideal width when there is room and shrinks a single line when crowded', () => {
    const groups = creatures(3);
    expect(fitRow(groups, 1000, 200, 120, 50)).toMatchObject({ cardWidth: 120, breaks: [] });
    const crowded = fitRow(groups, 300, 200, 120, 50);
    expect(crowded.cardWidth).toBeLessThan(120);
    expect(crowded.breaks).toEqual([]);
  });

  it('wraps a crowded row into lines rather than shrinking past the minimum', () => {
    // a multiplayer seat: about 335 px wide, a creature row 210 px tall
    const groups = creatures(13);
    const fit = fitRow(groups, 335, 202, 150, 58);
    expect(fit.breaks.length).toBeGreaterThan(0);
    expect(fit.gap).toBeGreaterThan(0);
    expect(fit.cardWidth).toBeGreaterThanOrEqual(ROW_FLOOR);
  });

  it('overlaps cards once even the smallest ones do not fit', () => {
    const groups = creatures(60);
    const fit = fitRow(groups, 335, 202, 150, 58);
    expect(fit.cardWidth).toBe(ROW_FLOOR);
    expect(fit.gap).toBeLessThan(0);
  });

  it('never draws a row outside its box', () => {
    for (const count of [1, 4, 9, 13, 20, 33, 60, 120]) {
      for (const tapped of [false, true]) {
        for (const [width, height] of [[335, 202], [520, 202], [1126, 202], [335, 149]]) {
          const groups = creatures(count, { tapped });
          const fit = fitRow(groups, width, height, 150, 58);
          const drawn = extent(groups, fit);
          for (const line of drawn.widths) expect(line).toBeLessThanOrEqual(width + 0.5);
          expect(drawn.height).toBeLessThanOrEqual(height + 0.5);
        }
      }
    }
  });
});

describe('card identity across zones', () => {
  it('keys a card by its physical card, so it stays the same element when its object id changes', () => {
    expect(cardKey({ id: 'spell-1', cardId: 'card-1' })).toBe('card-1');
    expect(cardKey({ id: 'ability-1' })).toBe('ability-1');
    const board = buildPlayerBoard(player('me', [permanent('perm-1', { cardId: 'card-1', cardTypes: ['CREATURE'] })]), true);
    expect(board.front[0].key).toBe('card-1');
  });

  it('keeps keys unique when copies share a card', () => {
    expect(zoneKeys([{ id: 's1', cardId: 'c1' }, { id: 's2', cardId: 'c1' }, { id: 's3' }])).toEqual(['c1', 'c1:s2', 's3']);
  });
});

describe('land stacks: open first, and how many are open', () => {
  it('puts untapped lands first and tapped after, keeping arrival order and the stack key', () => {
    const board = buildPlayerBoard(player('me', [
      permanent('a', { tapped: true }), permanent('b'), permanent('c', { tapped: true }), permanent('d'),
    ]), true);
    expect(board.back).toHaveLength(1);
    expect(board.back[0].key).toBe('a');
    expect(board.back[0].members.map((p) => p.id)).toEqual(['b', 'd', 'a', 'c']);
    expect(stackBadge(board.back[0])).toEqual({ count: 4, open: 2 });
  });

  it('counts the stack when none is tapped, and 0 open when all are', () => {
    const open = buildPlayerBoard(player('me', [permanent('a'), permanent('b'), permanent('c')]), true);
    expect(stackBadge(open.back[0])).toEqual({ count: 3 });
    const closed = buildPlayerBoard(player('me', [permanent('a', { tapped: true }), permanent('b', { tapped: true })]), true);
    expect(stackBadge(closed.back[0])).toEqual({ count: 2, open: 0 });
  });

  it('has no badge for a single card, and leaves token stacks in arrival order with a plain count', () => {
    expect(stackBadge(buildPlayerBoard(player('me', [permanent('a', { tapped: true })]), true).back[0])).toBeNull();
    const token: Partial<PermanentView> = { name: 'Clue', cardTypes: ['ARTIFACT'], isToken: true };
    const board = buildPlayerBoard(player('me', [permanent('t1', { ...token, tapped: true }), permanent('t2', token)]), true);
    expect(board.back[0].members.map((p) => p.id)).toEqual(['t1', 't2']);
    expect(stackBadge(board.back[0])).toEqual({ count: 2 });
  });

  it('splitOpenFirst is stable within each half', () => {
    expect(splitOpenFirst([permanent('x', { tapped: true }), permanent('y'), permanent('z')]).map((p) => p.id)).toEqual(['y', 'z', 'x']);
  });
});
