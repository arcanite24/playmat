import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, test } from 'vitest';
import { DeckSerializer } from './DeckSerializer';
import { isCommanderFormat } from './formats';
import { metaDecksFor, pickMetaDeck, type MetaDeckIndex } from './metaDecks';

const DIR = join(__dirname, '../../../public/meta-decks');
const SETS = join(__dirname, '../../../../Mage.Sets/src');
const CORE = join(__dirname, '../../../../Mage/src/main/java');
const FORMATS = join(__dirname, '../../../../Mage.Server.Plugins/Mage.Deck.Constructed/src/mage/deck');
const index = JSON.parse(readFileSync(join(DIR, 'index.json'), 'utf8')) as MetaDeckIndex;
const formats = [
  'Constructed - Standard', 'Constructed - Pioneer', 'Constructed - Modern', 'Constructed - Legacy',
  'Constructed - Vintage', 'Constructed - Pauper', 'Constructed - Premodern',
  'Variant Magic - Duel Commander', 'Variant Magic - Commander',
];
const basics = new Set([
  'Plains', 'Island', 'Swamp', 'Mountain', 'Forest', 'Wastes',
  'Snow-Covered Plains', 'Snow-Covered Island', 'Snow-Covered Swamp', 'Snow-Covered Mountain', 'Snow-Covered Forest',
]);
const anyNumber = new Set(['Hare Apparent', 'Relentless Rats']);

// Resolve the exact printing to its registered Java implementation, without a server or external card service.
const printings = new Map<string, { name: string; implementation: string }>();
const sets = new Map<string, { code: string; date: string; type: string; rotation: boolean }>();
const legalPrintings = new Map<string, { code: string; rarity: string }[]>();
for (const file of readdirSync(join(SETS, 'mage/sets')).filter((file) => file.endsWith('.java'))) {
  const source = readFileSync(join(SETS, 'mage/sets', file), 'utf8').replace(/^\s*\/\/.*$/gm, '');
  const code = /super\("(?:[^"\\]|\\.)*", "([^"]+)"/.exec(source)?.[1];
  if (!code) continue;
  const metadata = /ExpansionSet.buildDate\((\d+), (\d+), (\d+)\), SetType\.(\w+)/.exec(source);
  if (metadata) sets.set(file.replace('.java', ''), {
    code, date: `${metadata[1]}-${metadata[2].padStart(2, '0')}-${metadata[3].padStart(2, '0')}`,
    type: metadata[4], rotation: source.includes('this.rotationSet = true'),
  });
  for (const match of source.matchAll(/new SetCardInfo\(("(?:[^"\\]|\\.)*"), (?:"[^"]+"|\d+), Rarity\.(\w+)/g)) {
    const name = JSON.parse(match[1]) as string;
    const entries = legalPrintings.get(name) ?? [];
    entries.push({ code, rarity: match[2] });
    legalPrintings.set(name, entries);
  }
  for (const match of source.matchAll(/new SetCardInfo\(("(?:[^"\\]|\\.)*"), ("[^"]+"|\d+), Rarity\.\w+, (mage\.cards\.[\w.]+)\.class/g)) {
    printings.set(`${code}:${match[2].replaceAll('"', '')}`, {
      name: JSON.parse(match[1]) as string,
      implementation: match[3].replaceAll('.', '/') + '.java',
    });
  }
}

// Check the pool at its snapshot date against this checkout's format rules; the live test checks server behavior.
const standardTypes = new Set(['CORE', 'EXPANSION', 'SUPPLEMENTAL_STANDARD_LEGAL']);
const standardStart = [...sets.values()].filter((set) => set.rotation && set.date <= index.updated)
  .map((set) => set.date).sort().reverse()[2];
const namesAdded = (source: string, field: string) => new Set([...source.matchAll(new RegExp(`${field}\\.add\\(("(?:[^"\\\\]|\\\\.)*")\\)`, 'g'))]
  .map((match) => JSON.parse(match[1]) as string));
const constructed = readFileSync(join(CORE, 'mage/cards/decks/Constructed.java'), 'utf8');

function formatRules(format: string) {
  const name = format.split(' - ')[1].replaceAll(' ', '');
  const source = readFileSync(join(FORMATS, `${name}.java`), 'utf8');
  const banned = namesAdded((source.includes('banned.clear()') ? '' : constructed) + source, 'banned');
  const restricted = namesAdded(source, 'restricted');
  const bannedCommander = namesAdded(source, 'bannedCommander');
  const premodern = new Set([...source.matchAll(/setCodes.add\(mage.sets.(\w+)\.getInstance/g)].map((match) => sets.get(match[1])!.code));
  const codes = new Set([...sets.values()].filter((set) => {
    if (name === 'Standard') return standardTypes.has(set.type) && set.date >= standardStart;
    if (name === 'Pioneer') return standardTypes.has(set.type) && set.date >= '2012-10-05';
    if (name === 'Modern') return (standardTypes.has(set.type) || set.type === 'SUPPLEMENTAL_MODERN_LEGAL') && set.date >= '2003-07-28';
    if (name === 'Premodern') return premodern.has(set.code);
    return !['CUSTOM_SET', 'JOKE_SET', 'REMIX', 'MAGIC_ARENA'].includes(set.type);
  }).map((set) => set.code));
  return { banned, restricted, bannedCommander, codes };
}

describe('shipped AI deck pool', () => {
  test('the index and deck files match, with no duplicate entries', () => {
    const files = index.decks.map((deck) => deck.file);
    expect(new Set(files).size).toBe(files.length);
    expect([...files].sort()).toEqual(readdirSync(DIR).filter((file) => file.endsWith('.dck')).sort());
    expect([...new Set(index.decks.map((deck) => deck.format))].sort()).toEqual([...formats].sort());
  });

  test.each(formats)('%s has a varied pool whose every deck can be drawn', (format) => {
    const pool = metaDecksFor(index.decks, format);
    expect(pool.length).toBeGreaterThanOrEqual(12);
    expect(pool.length).toBeLessThanOrEqual(20);
    expect([...new Set(pool.map((deck) => deck.tier))].sort()).toEqual(['fun', 'meta', 'rogue']);
    const taken = new Set<string>();
    for (let i = 0; i < pool.length; i++) {
      const picked = pickMetaDeck(index.decks, format, taken, () => 0)!;
      expect(picked).not.toBeNull();
      expect(taken.has(picked.file)).toBe(false);
      taken.add(picked.file);
    }
    expect(pickMetaDeck(index.decks, format, taken)).toBeNull();
  });

  test.each(index.decks)('$file parses with the right sizes and implemented printings', (entry) => {
    expect(entry.file).toMatch(/^[a-z0-9_-]+\.dck$/);
    expect(['meta', 'rogue', 'fun']).toContain(entry.tier);
    expect(entry.source.length).toBeGreaterThan(0);
    expect(Number.isFinite(entry.share) && entry.share >= 0).toBe(true);
    const text = readFileSync(join(DIR, entry.file), 'utf8');
    const lines = text.trim().split(/\r?\n/);
    expect(lines.shift()).toBe(`NAME:${entry.name}`);
    for (const line of lines) expect(line).toMatch(/^(SB: )?[1-9]\d* \[[^:\]]+:[^\]]+\] .+$/);
    const deck = DeckSerializer.importDeck(text);
    const all = [...deck.cards, ...deck.sideboard];
    expect(all).toHaveLength(lines.length);
    expect(deck.name).toBe(entry.name);
    const commander = isCommanderFormat(entry.format);
    const main = deck.cards.reduce((sum, card) => sum + card.amount, 0);
    const side = deck.sideboard.reduce((sum, card) => sum + card.amount, 0);
    expect(entry.cards).toBe(commander ? 100 : 60);
    expect(main + (commander ? side : 0)).toBe(entry.cards);
    if (commander) {
      expect(side).toBeGreaterThanOrEqual(1);
      expect(side).toBeLessThanOrEqual(2);
    } else {
      expect(side).toBeLessThanOrEqual(15);
    }
    const counts = new Map<string, number>();
    const rules = formatRules(entry.format);
    for (const card of all) {
      const printing = printings.get(`${card.setCode}:${card.cardNumber}`);
      expect(printing?.name, `${card.setCode}:${card.cardNumber}`).toBe(card.cardName);
      expect(existsSync(join(SETS, printing!.implementation)) || existsSync(join(CORE, printing!.implementation)), card.cardName).toBe(true);
      counts.set(card.cardName, (counts.get(card.cardName) ?? 0) + card.amount);
      expect(rules.banned.has(card.cardName), `banned: ${card.cardName}`).toBe(false);
      expect(legalPrintings.get(card.cardName)?.some((printing) => rules.codes.has(printing.code)
        && (entry.format !== 'Constructed - Pauper' || ['COMMON', 'LAND'].includes(printing.rarity))), `format legality: ${card.cardName}`).toBe(true);
    }
    for (const [name, count] of counts) {
      if (!basics.has(name) && !anyNumber.has(name)) expect(count, name).toBeLessThanOrEqual(commander ? 1 : 4);
      if (rules.restricted.has(name)) expect(count, `restricted: ${name}`).toBe(1);
    }
    if (commander) for (const card of deck.sideboard) expect(rules.bannedCommander.has(card.cardName), `banned commander: ${card.cardName}`).toBe(false);
    expect(all.some((card) => card.cardName === entry.cover.name && card.setCode === entry.cover.setCode && card.cardNumber === entry.cover.cardNumber)).toBe(true);
  });
});
