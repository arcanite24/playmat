/**
 * A Career duel on the Rust host (docs/rust/GameHost.md, Career tables): a new Career's first roster opponent is
 * played on Rust, not Java, to its end, and the Career pays it as it pays a game Java played.
 * Needs a local server with accounts and the Career on (authenticationActivated="true", -testMode=false), started
 * with XMAGE_RUST_HOST_USERS=*, and a gateway with XMAGE_RUST_HOST=on in front of it:
 *   MAGE_LIVE_SERVER=1 MAGE_SERVER_URL=ws://127.0.0.1:17180 MATCH_SERVER_LOG=<server log> \
 *     npx vitest run src/core/headless/rustHostCareer
 * A server takes 3 new accounts an hour from one address: each run makes one.
 */
import { readFileSync } from 'node:fs';
import { describe, expect, test } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import type { TableClientMessage } from '../../protocol/generated/views';
import { EventBus } from '../rpc/EventBus';
import { RpcClient } from '../rpc/RpcClient';
import { GameSession } from '../game/gameSession';
import { botPlay, type BotCounters } from './botGame';
import { DeckSerializer } from '../decks/DeckSerializer';
import type { DeckCardLists } from '../decks/types';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17180';
const live = process.env.MAGE_LIVE_SERVER === '1';
const SERVER_LOG = process.env.MATCH_SERVER_LOG ?? '';

function starterDeck(starterId: string): DeckCardLists {
  const text = readFileSync(new URL(`../../../public/starter-decks/${starterId}.dck`, import.meta.url), 'utf8');
  return { ...DeckSerializer.importDeck(text), id: 'career', format: 'Constructed - Freeform' };
}

// roster opponents that play with a personality (Mage.Server career/opponents.json)
const PERSONALITIES = ['pip', 'ilse', 'widow', 'grondel'];

async function playOne(api: ReturnType<typeof createApi>, bus: EventBus, userName: string, opponentId: string,
                       deck: unknown, hint: boolean) {
  const started = new Promise<TableClientMessage>((resolve) => bus.on('START_GAME', (m) => resolve(m)));
  await api.careerPlay(opponentId, deck as never);
  const start = await started;
  const gameId = start.gameId!;
  const session = new GameSession(api, bus, { gameId, playerId: start.playerId ?? null, mode: 'play' });
  await api.gameJoin(gameId);
  let hintsBefore: number | undefined;
  let hintsAfter: number | undefined;
  const stats: BotCounters = { commands: 0, landsPlayed: 0, spellsCast: 0, attacks: 0, maxTurn: 0 };
  // a hint is asked on turn 3, while the game waits for the player: the bot holds there, then plays on
  const options: Parameters<typeof botPlay>[1] = { serverUrl: SERVER_URL, userName, maxTurns: 25, timeoutMs: 600_000, stuckMs: 30_000,
    ...(hint ? { holdAfterTurn: 2, holdUntil: Date.now() + 600_000 } : {}) } as never;
  const playing = botPlay(session, options, stats, {});
  if (hint) {
    playing.catch(() => undefined);
    for (let i = 0; i < 1200; i++) {
      const state = session.getState();
      if (state.gameOver || ((state.view?.turn ?? 0) >= 3 && state.interaction.mode === 'priority')) break;
      await new Promise((r) => setTimeout(r, 250));
    }
    expect(session.getState().gameOver, 'the game reaches turn 3').toBeFalsy();
    hintsBefore = (await api.soloAssists(gameId)).hintsLeft;
    const advice = await api.gameAdvise(gameId, null);
    console.log(`advice: ${advice.kind} ${advice.text}`);
    expect(advice.kind).not.toBe('NONE');
    // the Career counts the hint, whoever plays the game (hostGameAssisted for the Rust host)
    for (let i = 0; i < 20; i++) {
      hintsAfter = (await api.soloAssists(gameId)).hintsLeft;
      if (hintsAfter !== hintsBefore) break;
      await new Promise((r) => setTimeout(r, 250));
    }
    expect(hintsAfter).toBe((hintsBefore ?? 0) - 1);
    delete options.holdAfterTurn;
  }
  await playing;
  expect(session.getState().gameOver, 'the game ends').toBeTruthy();
  session.dispose();
  return { gameId, turns: stats.maxTurn, hintsBefore, hintsAfter };
}

async function careerDuel(prefix: string, pick: (ids: string[]) => string | undefined, hint: boolean) {
  // MAGE_CAREER_USER plays on an account an earlier run made (a server takes 3 new accounts an hour from one address)
  const userName = process.env.MAGE_CAREER_USER ?? `${prefix}${Date.now() % 1000000}`;
  const rpc = new RpcClient({ stateDiffs: true });
  const bus = new EventBus(rpc);
  const api = createApi(rpc);
  await rpc.connect(SERVER_URL);
  try {
    await api.authRegister(userName, 'careerpass1', `${userName}@example.test`).catch(() => undefined);
    expect(await api.connectUser(userName, 'careerpass1')).toBe(true);
    const starters = await api.careerStarters();
    expect(starters.length).toBeGreaterThan(0);
    if (!(await api.careerState()).profile) await api.careerStart(starters[0].id!);
    const before = (await api.careerState()).profile!;
    const open = (await api.careerOpponents()).filter((o) => o.unlocked);
    const id = pick(open.map((o) => o.id!));
    const opponent = open.find((o) => o.id === id);
    expect(opponent, 'an open opponent').toBeTruthy();
    // a new Career has no deck on the server yet: it plays the starter it opened with, as the app does
    const deck = (await api.careerDeck()) ?? starterDeck(starters[0].id!);
    // an opponent plays one of its decks at random: play on until one has only cards Rust has (each game counts)
    let gameId = '';
    let played = 0;
    for (let attempt = 0; attempt < 5 && !gameId; attempt++) {
      const result = await playOne(api, bus, userName, opponent!.id!, deck, hint);
      played++;
      if (!SERVER_LOG || readFileSync(SERVER_LOG, 'utf8').includes(`Rust host: plays ${userName}'s game ${result.gameId}`)) {
        gameId = result.gameId;
        console.log(`${opponent!.name} (${opponent!.id}): turn ${result.turns}, hints ${result.hintsBefore}→${result.hintsAfter}`);
      }
    }
    expect(gameId, 'a game the Rust host plays').toBeTruthy();
    // the Career counts every game, won or lost, as for one Java played
    let after = (await api.careerState()).profile!;
    const games = (p: typeof after) => (p.wins ?? 0) + (p.losses ?? 0);
    for (let i = 0; i < 20 && games(after) < games(before) + played; i++) {
      await new Promise((r) => setTimeout(r, 500));
      after = (await api.careerState()).profile!;
    }
    console.log(`wins ${before.wins}→${after.wins}, losses ${before.losses}→${after.losses}, coins ${before.coins}→${after.coins}`);
    expect(games(after)).toBe(games(before) + played);
    if (SERVER_LOG) {
      const lines = readFileSync(SERVER_LOG, 'utf8').split('\n');
      const at = lines.findIndex((line) => line.includes(`Rust host: plays ${userName}'s game ${gameId}`));
      const paid = lines.slice(at).find((line) => line.includes(`Career: ${userName} `) && line.includes('paid'));
      expect(paid, 'the Career paid the hosted game').toBeTruthy();
      console.log(paid!.replace(/\s+=>.*/, ''));
    }
  } finally {
    await api.disconnectSession(false).catch(() => undefined);
    rpc.disconnect();
  }
}

describe.skipIf(!live)('a Rust-hosted Career duel', () => {
  test('is played on Rust and paid by the Career', () => careerDuel('cr', (ids) => ids[0], false), 900_000);

  test('plays an opponent with a personality, and a hint is counted by the Career', () =>
    careerDuel('cp', (ids) => ids.find((id) => PERSONALITIES.includes(id)), true), 900_000);
});
