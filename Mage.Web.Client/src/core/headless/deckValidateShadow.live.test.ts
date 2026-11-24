/**
 * deckValidate in shadow (docs/rust/Catalog.md): the formats Rust answers beside plain Constructed (Freeform,
 * Freeform Unlimited, Limited) checked against Java's answer on real decks, short ones and copy limits included.
 * Needs a local server started with -Dxmage.catalog.export=<file>, and a gateway in front of it with
 * XMAGE_ROUTE_CATALOG=shadow and XMAGE_PLATFORM_CATALOG=<file>; the gateway logs any difference
 * ("Rust answered differently"), and the test reads its log:
 *   MAGE_LIVE_SERVER=1 GATEWAY_LOG=<gateway log> npx vitest run src/core/headless/deckValidateShadow
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, test } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import { RpcClient } from '../rpc/RpcClient';
import { DeckSerializer } from '../decks/DeckSerializer';
import type { DeckCardLists } from '../decks/types';
import { deckList } from './botGame';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17180';
const live = process.env.MAGE_LIVE_SERVER === '1';
const GATEWAY_LOG = process.env.GATEWAY_LOG ?? '';

function decks(): DeckCardLists[] {
  const starters = join(__dirname, '../../../public/starter-decks');
  const shadow = join(__dirname, '../../../../rust/crates/playmat-engine/shadow-decks/tier30');
  return [
    ...readdirSync(starters).filter((f) => f.endsWith('.dck')).map((f) => DeckSerializer.importDeck(readFileSync(join(starters, f), 'utf8'))),
    ...readdirSync(shadow).filter((f) => f.endsWith('.txt')).map((f) => deckList(f, readFileSync(join(shadow, f), 'utf8'))),
  ];
}

describe.skipIf(!live || !GATEWAY_LOG)('deckValidate in shadow', () => {
  test('Freeform, Freeform Unlimited and Limited answer as Java does', async () => {
    const rpc = new RpcClient({ stateDiffs: true });
    const api = createApi(rpc);
    await rpc.connect(SERVER_URL);
    const before = readFileSync(GATEWAY_LOG, 'utf8').split('\n').length;
    try {
      expect(await api.connectUser(`dv${Date.now() % 100000}`, 'testpass')).toBe(true);
      const all = decks();
      expect(all.length).toBeGreaterThan(10);
      const short = (d: DeckCardLists): DeckCardLists => ({ ...d, cards: d.cards.slice(0, Math.ceil(d.cards.length / 2)) });
      const dwarves: DeckCardLists = { ...all[0], cards: [...all[0].cards, { cardName: 'Seven Dwarves', setCode: '', cardNumber: '', amount: 8 }] };
      let calls = 0;
      for (const format of ['Constructed - Freeform', 'Constructed - Freeform Unlimited', 'Limited']) {
        for (const deck of [...all, ...all.map(short), dwarves]) {
          await api.deckValidate(format, deck as never);
          calls++;
        }
      }
      // the gateway compares after Java answers: a moment for the last ones
      await new Promise((r) => setTimeout(r, 1000));
      const differences = readFileSync(GATEWAY_LOG, 'utf8').split('\n').slice(before)
        .filter((line) => line.includes('shadow deckValidate'));
      console.log(`${calls} calls, ${differences.length} differences`);
      expect(differences.map((line) => line.slice(0, 400))).toEqual([]);
    } finally {
      await api.disconnectSession(false).catch(() => undefined);
      rpc.disconnect();
    }
  }, 300_000);
});
