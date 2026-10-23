/**
 * The AI's format decks (public/meta-decks): metagame snapshots and original rogue and casual lists, with printings
 * the server knows. Every deck must be legal in its format, or the AI can't take its seat.
 *
 * Check them (needs a running local server):
 *   MAGE_LIVE_SERVER=1 npx vitest run src/core/headless/metaDecks
 * Refresh only the meta tier from MTGGoldfish first, then check (preserves original rogue and fun lists; meta decks
 * the server can't play are left out and listed):
 *   MAGE_LIVE_SERVER=1 META_DECKS_REFRESH=1 npx vitest run src/core/headless/metaDecks
 */
import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, test } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import { RpcClient } from '../rpc/RpcClient';
import { DeckSerializer } from '../decks/DeckSerializer';
import type { DeckCardInfo, DeckCardLists } from '../decks/types';
import { isCommanderFormat } from '../decks/formats';
import { toWire } from '../../app/decks/deckModel';
import type { MetaDeck, MetaDeckIndex } from '../decks/metaDecks';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17172';
const live = process.env.MAGE_LIVE_SERVER === '1';
const refresh = process.env.META_DECKS_REFRESH === '1';
const DIR = join(__dirname, '../../../public/meta-decks');
const GOLDFISH = 'https://www.mtggoldfish.com';

/**
 * MTGGoldfish's formats the server has a deck type for, and how many archetypes to keep. Historic, Explorer, Timeless
 * and Alchemy are Arena formats: next to no paper metagame, Alchemy's rebalanced cards aren't in XMage, and XMage's
 * Historic keeps an old ban list and Arena-only printings. Goldfish's Brawl is 100-card Historic Brawl, which XMage's
 * 60-card Brawl doesn't take, and Penny Dreadful has no XMage deck type.
 */
const FORMATS: { page: string; deckType: string; keep: number }[] = [
  { page: 'standard', deckType: 'Constructed - Standard', keep: 6 },
  { page: 'pioneer', deckType: 'Constructed - Pioneer', keep: 6 },
  { page: 'modern', deckType: 'Constructed - Modern', keep: 6 },
  { page: 'legacy', deckType: 'Constructed - Legacy', keep: 6 },
  { page: 'vintage', deckType: 'Constructed - Vintage', keep: 5 },
  { page: 'pauper', deckType: 'Constructed - Pauper', keep: 6 },
  { page: 'premodern', deckType: 'Constructed - Premodern', keep: 6 },
  { page: 'duel_commander', deckType: 'Variant Magic - Duel Commander', keep: 4 },
  { page: 'commander', deckType: 'Variant Magic - Commander', keep: 5 },
];

const unescape = (text: string) => text
  .replace(/&#39;/g, "'").replace(/&quot;/g, '"').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&');

async function page(path: string): Promise<string> {
  const response = await fetch(GOLDFISH + path, { headers: { 'User-Agent': 'Mozilla/5.0' } });
  const text = await response.text();
  if (!response.ok || text.includes('<title>Just a moment...</title>')) throw new Error(`MTGGoldfish refused ${path} (${response.status})`);
  return text;
}

interface Archetype { slug: string; name: string; share: number; keyCards: string[] }

/** The archetype tiles of a format's metagame page, most played first. */
function archetypes(html: string): Archetype[] {
  return html.split("<div class='archetype-tile' id=").slice(1).flatMap((tile) => {
    const link = /href="\/archetype\/([^"#]+)#paper">([^<]+)<\/a>/.exec(tile);
    const share = /metagame-percentage'>.*?statistic-value'>\s*([\d.]+)%/s.exec(tile);
    if (!link || unescape(link[2]).trim() === 'Other') return [];
    const keyCards = [...tile.matchAll(/<li>([^<]+)<\/li>/g)].map((match) => unescape(match[1]).trim());
    return [{ slug: link[1], name: unescape(link[2]).trim(), share: share ? Number(share[1]) : 0, keyCards }];
  });
}

/** An archetype page's featured list, as Goldfish's text export ("4 Card", then "sideboard"). */
function featuredList(html: string): string {
  const value = /id="deck_input_deck" value="([^"]*)"/.exec(html);
  if (!value) throw new Error('no deck list on the page');
  return unescape(value[1]);
}

function parseList(text: string): { cards: DeckCardInfo[]; sideboard: DeckCardInfo[] } {
  const cards: DeckCardInfo[] = [];
  const sideboard: DeckCardInfo[] = [];
  let zone = cards;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (/^sideboard$/i.test(line)) { zone = sideboard; continue; }
    const match = /^(\d+)\s+(.+)$/.exec(line);
    // a printing note ("Plains <272> [FDN]") is dropped, and an Alchemy rebalance ("A-Card") plays as the paper card
    const name = match?.[2].replace(/\s*<[^>]*>/g, '').replace(/\s*\[[^\]]*\]/g, '').replace(/^A-/, '').replace(/\s+\/\s+/g, ' // ').trim();
    if (match && name) zone.push({ cardName: name, amount: Number(match[1]) });
  }
  return { cards, sideboard };
}

const slugOf = (text: string) => text.toLowerCase().normalize('NFD').replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');

async function connect() {
  const rpc = new RpcClient({ heartbeatMs: 0 });
  const api = createApi(rpc);
  await rpc.connect(SERVER_URL);
  expect(await api.connectUser(`meta_${Date.now() % 100000}`, 'testpass')).toBe(true);
  return { rpc, api };
}

describe.skipIf(!live)('the AI\'s format decks', () => {
  test.skipIf(!refresh)('refresh from MTGGoldfish', async () => {
    const { rpc, api } = await connect();
    const previous = JSON.parse(readFileSync(join(DIR, 'index.json'), 'utf8')) as MetaDeckIndex;
    const decks: MetaDeck[] = previous.decks.filter((deck) => deck.tier === 'rogue' || deck.tier === 'fun');
    const skipped: string[] = [];
    mkdirSync(DIR, { recursive: true });
    for (const format of FORMATS) {
      const commander = isCommanderFormat(format.deckType);
      const tiles = archetypes(await page(`/metagame/${format.page}/full`));
      let kept = 0;
      for (const tile of tiles) {
        if (kept >= format.keep) break;
        const problems: string[] = [];
        const list = parseList(featuredList(await page(`/archetype/${tile.slug}`)));
        // Goldfish lists a commander deck as its 100 cards; the commander (the archetype's name) goes to the command zone
        if (commander) {
          const names = tile.name.split(/\s+\+\s+/);
          // some lists already keep it in the sideboard
          const listed = list.sideboard.splice(0);
          for (const name of names) {
            const at = list.cards.findIndex((card) => card.cardName === name);
            if (at >= 0) list.sideboard.push(...list.cards.splice(at, 1));
            else if (listed.some((card) => card.cardName === name)) list.sideboard.push(...listed.filter((card) => card.cardName === name));
            else problems.push(`commander ${name} not in the list`);
          }
          if (list.sideboard.length !== names.length) problems.push('the list has a sideboard');
        }
        const all = [...list.cards, ...list.sideboard];
        const found = await api.lookupCards(all.map((card) => ({ ...card, setCode: '', cardNumber: '' })));
        all.forEach((card, i) => {
          const view = found[i];
          if (!view) { problems.push(`unknown card ${card.cardName}`); return; }
          card.setCode = view.expansionSetCode;
          card.cardNumber = view.cardNumber;
        });
        const deck: DeckCardLists = { name: tile.name, format: format.deckType, ...list };
        if (problems.length === 0) {
          const result = await api.deckValidate(format.deckType, toWire(deck));
          if (!result.valid) for (const error of result.errors ?? []) problems.push(`${error.group} ${error.message}`);
        }
        if (problems.length > 0) {
          skipped.push(`${format.page} ${tile.name}: ${problems.slice(0, 4).join('; ')}`);
          continue;
        }
        const coverName = commander ? list.sideboard[0].cardName : tile.keyCards.find((name) => all.some((card) => card.cardName === name));
        const cover = all.find((card) => card.cardName === coverName) ?? list.cards[0];
        const file = `${format.page}-${slugOf(tile.name)}.dck`;
        if (decks.some((deck) => deck.file === file)) continue;
        writeFileSync(join(DIR, file), `NAME:${tile.name}\n${DeckSerializer.exportDeck(deck)}`);
        decks.push({
          file,
          name: tile.name,
          format: format.deckType,
          tier: 'meta',
          share: tile.share,
          cards: all.reduce((sum, card) => sum + card.amount, 0) - (commander ? 0 : list.sideboard.reduce((sum, card) => sum + card.amount, 0)),
          cover: { name: cover.cardName, setCode: cover.setCode!, cardNumber: cover.cardNumber! },
          source: `${GOLDFISH}/archetype/${tile.slug}`,
        });
        kept++;
      }
    }
    const keptFiles = new Set(decks.map((deck) => deck.file));
    for (const file of readdirSync(DIR)) {
      if (file.endsWith('.dck') && !keptFiles.has(file)) rmSync(join(DIR, file));
    }
    const index: MetaDeckIndex = { updated: new Date().toISOString().slice(0, 10), source: 'MTGGoldfish metagame snapshots and Playmat original lists', decks };
    writeFileSync(join(DIR, 'index.json'), `${JSON.stringify(index, null, 2)}\n`);
    console.log(`${decks.length} decks written; left out:\n${skipped.join('\n') || '(none)'}`);
    rpc.disconnect();
  }, 600_000);

  test('are legal in their format and every card is known', async () => {
    const { rpc, api } = await connect();
    const index = JSON.parse(readFileSync(join(DIR, 'index.json'), 'utf8')) as MetaDeckIndex;
    const deckTypes = await api.getDeckTypes();
    const problems: string[] = [];
    const files = new Set(readdirSync(DIR).filter((file) => file.endsWith('.dck')));
    for (const entry of index.decks) {
      if (!deckTypes.includes(entry.format)) problems.push(`${entry.name}: the server has no ${entry.format}`);
      if (!files.delete(entry.file)) { problems.push(`${entry.name}: ${entry.file} is missing`); continue; }
      const deck = DeckSerializer.importDeck(readFileSync(join(DIR, entry.file), 'utf8'));
      const all = [...deck.cards, ...deck.sideboard];
      const found = await api.lookupCards(all.map((card) => ({ ...card, setCode: card.setCode ?? '', cardNumber: card.cardNumber ?? '' })));
      all.forEach((card, i) => {
        if (found[i]?.name !== card.cardName) problems.push(`${entry.name}: ${card.setCode} #${card.cardNumber} is ${found[i]?.name ?? 'unknown'}, not ${card.cardName}`);
      });
      const result = await api.deckValidate(entry.format, toWire(deck));
      if (!result.valid) for (const error of result.errors ?? []) problems.push(`${entry.name}: ${error.group} ${error.message}`);
    }
    for (const file of files) problems.push(`${file} is not in index.json`);
    expect(problems).toEqual([]);
    rpc.disconnect();
  }, 120_000);
});
