import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { parseCardLine, parseList } from './parseList';

const fixture = (name: string) => readFileSync(new URL(`./fixtures/${name}`, import.meta.url), 'utf8');
const names = (cards: { cardName: string }[]) => cards.map((card) => card.cardName);
const total = (cards: { amount: number }[]) => cards.reduce((sum, card) => sum + card.amount, 0);

describe('parseCardLine', () => {
  it('reads the common card line formats', () => {
    expect(parseCardLine('4 Lightning Bolt')?.card).toEqual({ amount: 4, cardName: 'Lightning Bolt', setCode: null, cardNumber: null });
    expect(parseCardLine('4x Lightning Bolt')?.card.amount).toBe(4);
    expect(parseCardLine('1 Sol Ring (CMR) 472')?.card).toEqual({ amount: 1, cardName: 'Sol Ring', setCode: 'CMR', cardNumber: '472' });
    expect(parseCardLine('1 Sol Ring (cmr) 472')?.card.setCode).toBe('CMR');
    expect(parseCardLine('1 Arcane Signet (ELD)')?.card).toEqual({ amount: 1, cardName: 'Arcane Signet', setCode: 'ELD', cardNumber: null });
    expect(parseCardLine('4 [M11:149] Lightning Bolt')?.card).toEqual({ amount: 4, cardName: 'Lightning Bolt', setCode: 'M11', cardNumber: '149' });
    expect(parseCardLine('4 [M11] Lightning Bolt')?.card).toEqual({ amount: 4, cardName: 'Lightning Bolt', setCode: 'M11', cardNumber: null });
    expect(parseCardLine('4 [] Ragavan, Nimble Pilferer')?.card).toEqual({ amount: 4, cardName: 'Ragavan, Nimble Pilferer', setCode: null, cardNumber: null });
  });

  it('reads collector numbers with letters, stars and dashes', () => {
    expect(parseCardLine('1 Lightning Bolt (PLST) 2XM-129')?.card.cardNumber).toBe('2XM-129');
    expect(parseCardLine('1 Thoughtseize (PRM) 123★')?.card.cardNumber).toBe('123★');
    expect(parseCardLine('1 Brazen Borrower (ELD) 39p')?.card.cardNumber).toBe('39p');
  });

  it('drops finish markers and colors, and notices commander and maybeboard marks', () => {
    expect(parseCardLine('1 Fire // Ice (MH2) 290 *F*')?.card).toEqual({ amount: 1, cardName: 'Fire // Ice', setCode: 'MH2', cardNumber: '290' });
    expect(parseCardLine("1x Atraxa, Praetors' Voice *CMDR*")).toMatchObject({ section: 'commander', card: { cardName: "Atraxa, Praetors' Voice" } });
    expect(parseCardLine('1x Sol Ring (cmr) 472 [Ramp] ^Have,#37d67a^')).toMatchObject({ section: undefined, card: { cardName: 'Sol Ring', cardNumber: '472' } });
    expect(parseCardLine('1x Doubling Season (2xm) 152 [Maybeboard{noDeck}{noPrice}]')?.section).toBe('maybe');
    expect(parseCardLine('1x Atraxa (2x2) 190 [Commander{top}]')?.section).toBe('commander');
  });

  it('writes split cards one way', () => {
    expect(parseCardLine('1 Fire/Ice')?.card.cardName).toBe('Fire // Ice');
    expect(parseCardLine('1 Wear / Tear')?.card.cardName).toBe('Wear // Tear');
    expect(parseCardLine('1 Wear///Tear')?.card.cardName).toBe('Wear // Tear');
  });

  it('marks SB: lines as sideboard and rejects lines without a count', () => {
    expect(parseCardLine('SB:  2 [] Mystical Dispute')).toMatchObject({ sideboard: true, card: { amount: 2, cardName: 'Mystical Dispute' } });
    expect(parseCardLine('Lightning Bolt')).toBeNull();
    expect(parseCardLine('0 Lightning Bolt')).toBeNull();
  });
});

describe('parseList: site exports', () => {
  it('Moxfield Arena export with a commander and a maybeboard', () => {
    const list = parseList(fixture('moxfield-arena-commander.txt'));
    expect(names(list.commanders)).toEqual(["Atraxa, Praetors' Voice"]);
    expect(total(list.main)).toBe(99);
    expect(names(list.main)).toContain('Fire // Ice');
    expect(names(list.main)).toContain('Delver of Secrets // Insectile Aberration');
    expect(names(list.maybe)).toEqual(['Doubling Season']);
    expect(list.side).toEqual([]);
  });

  it('MTGGoldfish Arena export with a sideboard', () => {
    const list = parseList(fixture('mtggoldfish-arena.txt'));
    expect(total(list.main)).toBe(28);
    expect(total(list.side)).toBe(3);
    expect(list.main.find((card) => card.cardName === 'Wear // Tear')).toMatchObject({ setCode: 'DGM', cardNumber: '135' });
  });

  it('TappedOut text with *CMDR*', () => {
    const list = parseList(fixture('tappedout-txt.txt'));
    expect(names(list.commanders)).toEqual(["Atraxa, Praetors' Voice"]);
    expect(total(list.main)).toBe(99);
    expect(names(list.main)).toContain('Fire // Ice');
  });

  it('Deckstats Arena export with // headers and a maybeboard', () => {
    const list = parseList(fixture('deckstats-arena.txt'));
    expect(total(list.main)).toBe(60);
    expect(names(list.side)).toEqual(['Duress']);
    expect(names(list.maybe)).toEqual(['Liliana of the Veil']);
  });

  it('AetherHub Arena export with a companion', () => {
    const list = parseList(fixture('aetherhub-arena.txt'));
    expect(names(list.companion)).toEqual(['Lurrus of the Dream-Den']);
    expect(total(list.main)).toBe(60);
    expect(total(list.side)).toBe(3);
  });

  it('Archidekt text export with categories', () => {
    const list = parseList(fixture('archidekt-text.txt'));
    expect(names(list.commanders)).toEqual(["Atraxa, Praetors' Voice"]);
    expect(names(list.maybe)).toEqual(['Doubling Season']);
    expect(names(list.side)).toEqual(['Smothering Tithe']);
    expect(total(list.main)).toBe(98);
  });

  it('MTGO text: a blank line starts the sideboard', () => {
    const list = parseList(fixture('mtgo-plain.txt'));
    expect(total(list.main)).toBe(60);
    expect(names(list.side)).toEqual(['Blood Moon', "Brotherhood's End"]);
  });

  it('MTGO commander text: the last block is the commander', () => {
    const list = parseList(fixture('mtgo-commander.txt'));
    expect(names(list.commanders)).toEqual(["Atraxa, Praetors' Voice"]);
    expect(list.side).toEqual([]);
  });

  it('Arena About block names the deck', () => {
    const list = parseList(fixture('arena-about.txt'));
    expect(list.name).toBe('Mono Red Aggro');
    expect(total(list.main)).toBe(60);
  });

  it('XMage .dck with its name and layout lines', () => {
    const list = parseList(fixture('xmage.dck'));
    expect(list.name).toBe('Burn');
    expect(list.main).toEqual([
      { amount: 4, cardName: 'Lightning Bolt', setCode: 'M11', cardNumber: '149' },
      { amount: 56, cardName: 'Mountain', setCode: 'ANA', cardNumber: '22' },
    ]);
    expect(list.side).toEqual([{ amount: 3, cardName: 'Skullcrack', setCode: 'RTR', cardNumber: '120' }]);
  });

  it('MTGTop8 .dec', () => {
    const list = parseList(fixture('mtgtop8.dec'));
    expect(names(list.main)).toEqual(['Ragavan, Nimble Pilferer', 'Lightning Bolt']);
    expect(list.side).toEqual([{ amount: 1, cardName: 'Wear // Tear', setCode: 'DGM', cardNumber: null }]);
  });

  it('skips type headers and keeps the zone', () => {
    const list = parseList('Deck\nCreatures (4)\n4 Goblin Guide\nLands (20)\n20 Mountain\nSideboard (2)\n2 Smash to Smithereens');
    expect(names(list.main)).toEqual(['Goblin Guide', 'Mountain']);
    expect(names(list.side)).toEqual(['Smash to Smithereens']);
  });

  it('keeps an Arena export with Arena-only set codes and Alchemy names, and reports unreadable lines', () => {
    const list = parseList('Deck\n4 Lightning Bolt (Y22) 12\n2 A-Alrund\'s Epiphany (Y22) 5\n20 Mountain (ANA) 22\n2x\n0 Island\n');
    expect(list.main).toEqual([
      { amount: 4, cardName: 'Lightning Bolt', setCode: 'Y22', cardNumber: '12' },
      { amount: 2, cardName: "A-Alrund's Epiphany", setCode: 'Y22', cardNumber: '5' },
      { amount: 20, cardName: 'Mountain', setCode: 'ANA', cardNumber: '22' },
    ]);
    expect(list.skipped).toEqual(['2x', '0 Island']);
  });
});
