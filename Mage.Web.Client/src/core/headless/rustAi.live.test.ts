/**
 * The Rust AI (docs/rust/Ai.md) in the server's games, as a player meets it: the client core with the app's default
 * preferences and auto-pass, Selesnya Company against Pioneer mono-black Zombies, each deck on each side. Needs a
 * local server started with XMAGE_AI_BACKEND=rust, XMAGE_RUST_AI_USERS=* and XMAGE_RUST_AI_BIN (the bridge binary);
 * the server's log says whether each game kept the Rust AI ("Rust AI stops in ..." when it didn't).
 *   MAGE_LIVE_SERVER=1 RUST_AI_GAMES=4 npx vitest run src/core/headless/rustAi
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, test } from 'vitest';
import { autoAnswer } from '../game/autoPass';
import { DEFAULT_SETTINGS, toUserData } from '../../app/stores/settings';
import { deck, playBotGame } from './botGame';
import type { Command } from '../game/interaction';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17172';
const live = process.env.MAGE_LIVE_SERVER === '1';
const GAMES = Number(process.env.RUST_AI_GAMES ?? 4);
const DECKS = join(__dirname, '../../../../rust/crates/playmat-engine/shadow-decks/tier30');

function deckFile(file: string) {
  const cards = readFileSync(join(DECKS, file), 'utf8').split(/\r?\n/).flatMap((line): [number, string][] => {
    const m = /^(\d+)\s+(.+)$/.exec(line.trim());
    return m ? [[Number(m[1]), m[2]]] : [];
  });
  return deck(file, cards);
}

const COMMANDS: Record<string, Command> = {
  pass: { type: 'boolean', value: false },
  noAttacks: { type: 'boolean', value: true },
  noBlocks: { type: 'boolean', value: true },
};

describe.skipIf(!live)('the Rust AI in the server', () => {
  test('plays Selesnya Company against mono-black Zombies', async () => {
    const company = deckFile('gw-company.txt');
    const zombies = deckFile('b-zombies.txt');
    const settings = DEFAULT_SETTINGS;
    const results: string[] = [];
    for (let game = 0; game < GAMES; game++) {
      const [mine, theirs] = game % 2 === 0 ? [company, zombies] : [zombies, company];
      const stats = await playBotGame({
        serverUrl: SERVER_URL,
        userName: `rust${game}_${Date.now() % 10000}`,
        deck: mine,
        aiDeck: theirs,
        maxTurns: 30,
        timeoutMs: 900_000,
        userData: toUserData(settings),
        // what the app answers for the player (app/match/useAutoPass)
        intercept(state) {
          const answer = autoAnswer(state.view, state.prompt, {
            autoPass: settings.autoPass,
            autoSkipCombat: settings.autoSkipCombat,
            abilitiesOnTheirTurn: settings.abilitiesOnTheirTurn,
            combatStops: settings.combatStops,
            beginCombatStop: !!settings.stops.yourTurn?.beforeCombat,
          });
          return answer ? COMMANDS[answer] : undefined;
        },
      });
      results.push(`game ${game} ${mine.name} vs AI ${theirs.name}: turn ${stats.maxTurn}, ${stats.commands} answers, over ${stats.gameOver}`);
      expect(stats.gameOver).toBe(true);
    }
    console.log(`[rust ai]\n${results.join('\n')}`);
  }, 3_600_000 * 2);
});
