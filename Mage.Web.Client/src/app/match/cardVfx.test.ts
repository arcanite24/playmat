import { describe, expect, it } from 'vitest';
import type { CardView, GameEventView } from '../../protocol/generated/views';
import { inkOf, resolvedCasts, signatureFor, SIGNATURES } from './cardVfx';
import { lightningPath } from './signatureShapes';

const nothing = { targetedStack: false, dealtDamage: false, removed: null, swept: false, milled: false, drew: false } as const;

const spell = (card: Partial<CardView>): CardView => ({ mageObjectType: 'SPELL', cardTypes: ['INSTANT'], controllerId: 'me', ...card });

describe('signatureFor', () => {
  it('gives the listed cards their own effect', () => {
    expect(signatureFor(spell({ name: 'Lightning Bolt' }), nothing)).toEqual({ fx: 'lightning', ink: 'red', big: true });
    expect(signatureFor(spell({ name: 'Counterspell' }), nothing)?.fx).toBe('counter');
    expect(signatureFor(spell({ name: 'Wrath of God', cardTypes: ['SORCERY'] }), nothing)?.fx).toBe('sweep');
    expect(signatureFor(spell({ name: 'Swords to Plowshares' }), nothing)?.fx).toBe('beam');
  });

  it('finds a split card by either half', () => {
    expect(signatureFor(spell({ name: 'Breaking // Entering' }), nothing)?.fx).toBe('mill');
  });

  it('gives abilities of listed permanents their effect, and none to unlisted ones', () => {
    expect(signatureFor({ name: 'Hedron Crab', mageObjectType: 'ABILITY_STACK_FROM_CARD' }, nothing)?.fx).toBe('mill');
    expect(signatureFor({ name: 'Grizzly Bears', mageObjectType: 'ABILITY_STACK_FROM_CARD', cardTypes: ['INSTANT'] }, { ...nothing, dealtDamage: true })).toBeNull();
  });

  it('reads an unlisted instant or sorcery from what it did, in its color', () => {
    const red = spell({ name: 'Some Burn', color: { red: true } });
    expect(signatureFor(red, { ...nothing, dealtDamage: true })).toEqual({ fx: 'fire', ink: 'red' });
    const blue = spell({ name: 'Some Counter', color: { blue: true } });
    expect(signatureFor(blue, { ...nothing, targetedStack: true })).toEqual({ fx: 'counter', ink: 'blue' });
    const black = spell({ name: 'Some Kill', color: { black: true } });
    expect(signatureFor(black, { ...nothing, removed: 'graveyard' })).toEqual({ fx: 'tendrils', ink: 'black' });
    const white = spell({ name: 'Some Wrath', color: { white: true }, cardTypes: ['SORCERY'] });
    expect(signatureFor(white, { ...nothing, swept: true })).toEqual({ fx: 'sweep', ink: 'white' });
    expect(signatureFor(spell({ name: 'Nothing Much', color: { green: true } }), nothing)).toBeNull();
  });

  it('leaves unlisted creature spells alone', () => {
    expect(signatureFor(spell({ name: 'Grizzly Bears', cardTypes: ['CREATURE'] }), { ...nothing, drew: true })).toBeNull();
  });

  it('covers the cards most played on the public server', () => {
    for (const name of ['Collected Company', 'Hedron Crab', 'Fraying Sanity', 'Krenko, Mob Boss', 'Wight of Precinct Six', 'Ethereal Armor', 'Path to Exile', 'Counterspell', 'Lightning Bolt', 'Ponder']) {
      expect(SIGNATURES.has(name), name).toBe(true);
    }
  });
});

describe('inkOf', () => {
  it('is the one color, gold for several, white for none', () => {
    expect(inkOf({ color: { blue: true } })).toBe('blue');
    expect(inkOf({ color: { red: true, white: true } })).toBe('gold');
    expect(inkOf({})).toBe('white');
  });
});

describe('resolvedCasts', () => {
  const bolt = spell({ id: 's1', cardId: 'c1', name: 'Lightning Bolt', targets: ['them'] });

  it('finds a spell that left the stack, with the damage it dealt', () => {
    const events: GameEventView[] = [{ kind: 'DAMAGE', sourceId: 's1', targetId: 'them', amount: 3 }];
    const casts = resolvedCasts(new Map([['s1', bolt]]), new Set(), events, new Set());
    expect(casts).toHaveLength(1);
    expect(casts[0]).toMatchObject({ id: 's1', name: 'Lightning Bolt', controllerId: 'me', targets: ['them'], damage: 3 });
    expect(casts[0].signature.fx).toBe('lightning');
  });

  it('ignores spells still on the stack', () => {
    expect(resolvedCasts(new Map([['s1', bolt]]), new Set(['s1']), [], new Set())).toEqual([]);
  });

  it('plays the counterspell, not the spell it countered', () => {
    const counter = spell({ id: 's2', name: 'Counterspell', targets: ['s1'] });
    const casts = resolvedCasts(new Map([['s1', bolt], ['s2', counter]]), new Set(), [], new Set());
    expect(casts.map((cast) => cast.name)).toEqual(['Counterspell']);
  });

  it('counts milled cards per player', () => {
    const crab = { id: 'a1', name: 'Hedron Crab', mageObjectType: 'ABILITY_STACK_FROM_CARD' as const, controllerId: 'me', targets: ['them'] };
    const events: GameEventView[] = [1, 2, 3].map(() => ({ kind: 'ZONE', fromZone: 'LIBRARY', toZone: 'GRAVEYARD', playerId: 'them' }));
    const [cast] = resolvedCasts(new Map([['a1', crab]]), new Set(), events, new Set());
    expect(cast.milled.get('them')).toBe(3);
  });

  it('tells a sweeper from what left the battlefield', () => {
    const wrath = spell({ id: 's3', name: 'Unlisted Wrath', cardTypes: ['SORCERY'], color: { white: true } });
    const events: GameEventView[] = ['p1', 'p2', 'p3'].map((id) => ({ kind: 'ZONE', fromZone: 'BATTLEFIELD', toZone: 'GRAVEYARD', targetId: id }));
    const [cast] = resolvedCasts(new Map([['s3', wrath]]), new Set(), events, new Set(['p1', 'p2', 'p3']));
    expect(cast.signature).toEqual({ fx: 'sweep', ink: 'white' });
  });
});

describe('lightningPath', () => {
  it('runs from one point to the other and is the same for the same seed', () => {
    const path = lightningPath({ x: 0, y: 0 }, { x: 300, y: 100 }, 5);
    expect(path[0]).toEqual({ x: 0, y: 0 });
    expect(path[path.length - 1]).toEqual({ x: 300, y: 100 });
    expect(lightningPath({ x: 0, y: 0 }, { x: 300, y: 100 }, 5)).toEqual(path);
  });
});
