/**
 * Rust-hosted games under a restless player (docs/rust/GameHost.md, step D): random deck pairs from the decks the
 * engine knows, the app's default preferences and auto-answers, and a bot that also blocks, takes blocks back,
 * mulligans, orders triggers, and presses the F-keys now and then. Every game must end; a prompt the bot can't
 * answer, or an answer the host never takes up, fails the game with what was on screen.
 *
 *   MAGE_LIVE_SERVER=1 MAGE_SERVER_URL=ws://127.0.0.1:17180 SOAK_GAMES=24 SOAK_SEED=1 \
 *     npx vitest run src/core/headless/rustHostSoak
 *
 * SOAK_PAIR: two deck names (e.g. meta/pioneer-jund-sacrifice.txt,tier30/b-zombies.txt) every game plays.
 * SOAK_MINE: one deck name the bot always plays, against random decks.
 * SOAK_SERVER_LOG: the Java server's log; the test then checks each game was hosted on Rust, not Java.
 * SOAK_TIME_LIMIT, SOAK_BUFFER_TIME: the tables' priority timer (MatchTimeLimit and MatchBufferTime names, e.g.
 *   MIN___5 and SEC__05); the views must then show the player's clock.
 * SOAK_RUN_OUT: with a time limit, the bot stops answering after this turn, so its time runs out and it loses.
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, test } from 'vitest';
import { autoAnswer } from '../game/autoPass';
import { DEFAULT_SETTINGS, toUserData } from '../../app/stores/settings';
import { deckList, playBotGame } from './botGame';
import type { Command } from '../game/interaction';
import type { MatchBufferTime, MatchTimeLimit, PlayerAction } from '../../protocol/generated/views';
import { isTriggerOrderPrompt } from '../game/autoAnswer';
import type { GameSessionState } from '../game/gameSession';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17180';
const live = process.env.MAGE_LIVE_SERVER === '1';
const GAMES = Number(process.env.SOAK_GAMES ?? 12);
const SEED = Number(process.env.SOAK_SEED ?? Date.now() % 100000);
const SERVER_LOG = process.env.SOAK_SERVER_LOG ?? '';
const ROOT = join(__dirname, '../../../../rust/crates/playmat-engine/shadow-decks');
const DIRS = (process.env.SOAK_DECKS ?? 'meta,tier30').split(',');
const TIME_LIMIT = process.env.SOAK_TIME_LIMIT as MatchTimeLimit | undefined;
const BUFFER_TIME = process.env.SOAK_BUFFER_TIME as MatchBufferTime | undefined;
const RUN_OUT = process.env.SOAK_RUN_OUT === undefined ? undefined : Number(process.env.SOAK_RUN_OUT);
/** a clock without a limit: Java's Integer.MAX_VALUE */
const NO_LIMIT = 2147483647;

const COMMANDS: Record<string, Command> = {
  pass: { type: 'boolean', value: false },
  noAttacks: { type: 'boolean', value: true },
  noBlocks: { type: 'boolean', value: true },
};
const F_KEYS: PlayerAction[] = ['PASS_PRIORITY_UNTIL_NEXT_TURN', 'PASS_PRIORITY_UNTIL_TURN_END_STEP', 'PASS_PRIORITY_UNTIL_STACK_RESOLVED'];

/** mulberry32: the same seed plays the same choices */
function random(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function decks() {
  return DIRS.flatMap((dir) => {
    const path = join(ROOT, dir);
    if (!existsSync(path)) return [];
    return readdirSync(path).filter((f) => f.endsWith('.txt')).map((f) => deckList(`${dir}/${f}`, readFileSync(join(path, f), 'utf8')));
  });
}

/** The restless part of the player: answers some prompts its own way, the bot the rest. */
function restless(rand: () => number) {
  const blocked = new Set<string>();
  let mulligans = 0;
  let undone = 0;
  /** "always last" was just set for a trigger: the app then answers the question with no id */
  let pendingLast = false;
  return (state: GameSessionState): Command | undefined => {
    const { interaction, view } = state;
    const prompt = state.prompt;
    if (isTriggerOrderPrompt(prompt)) {
      if (pendingLast) {
        pendingLast = false;
        return { type: 'uuid', id: null };
      }
      const id = prompt.targets[0];
      const roll = rand();
      // the app's "always first" and "always last" (triggerOrderCommands): the action, then the answer
      if (id && roll < 0.2) return { type: 'action', action: 'TRIGGER_AUTO_ORDER_ABILITY_FIRST', data: id };
      if (id && roll < 0.4) {
        pendingLast = true;
        return { type: 'action', action: 'TRIGGER_AUTO_ORDER_ABILITY_LAST', data: id };
      }
      return undefined;
    }
    pendingLast = false;
    if (interaction.mode === 'mulligan') {
      if (mulligans < 1 && rand() < 0.3) {
        mulligans++;
        return { type: 'boolean', value: true };
      }
      return undefined;
    }
    if (interaction.mode === 'declareBlockers' && interaction.prompt?.kind === 'declareBlockers') {
      const free = interaction.prompt.possibleBlockers.filter((id) => !interaction.selected.has(id) && !blocked.has(id));
      // take a block back now and then
      const declared = [...interaction.selected];
      if (declared.length > 0 && undone < 2 && rand() < 0.3) {
        undone++;
        return { type: 'uuid', id: declared[0] };
      }
      if (free.length > 0 && rand() < 0.6) {
        blocked.add(free[0]);
        return { type: 'uuid', id: free[0] };
      }
      blocked.clear();
      return interaction.mainButton?.command;
    }
    if (interaction.mode === 'priority' && view?.activePlayerId !== view?.myPlayerId && rand() < 0.03) {
      return { type: 'action', action: F_KEYS[Math.floor(rand() * F_KEYS.length)] };
    }
    return undefined;
  };
}

describe.skipIf(!live)('Rust-hosted games, soaked', () => {
  test(`${GAMES} games of random decks (seed ${SEED})`, async () => {
    const all = decks();
    expect(all.length).toBeGreaterThan(1);
    const rand = random(SEED);
    const settings = DEFAULT_SETTINGS;
    const results: string[] = [];
    const failures: string[] = [];
    for (let game = 0; game < GAMES; game++) {
      let mine = all[Math.floor(rand() * all.length)];
      let theirs = all[Math.floor(rand() * all.length)];
      // SOAK_PAIR="dir/mine.txt,dir/theirs.txt": every game that pair
      const pair = process.env.SOAK_PAIR?.split(',');
      if (pair) [mine, theirs] = pair.map((name) => all.find((d) => d.name === name)!);
      // SOAK_MINE="dir/mine.txt": the bot always plays that deck, the AI a random one
      if (process.env.SOAK_MINE) mine = all.find((d) => d.name === process.env.SOAK_MINE)!;
      const userName = `soak${game}_${(SEED + game) % 10000}`;
      const own = restless(random(SEED * 1000 + game));
      // the lowest clock the player's views showed
      let clock = NO_LIMIT;
      try {
        const stats = await playBotGame({
          serverUrl: SERVER_URL,
          userName,
          deck: mine,
          aiDeck: theirs,
          maxTurns: 30,
          timeoutMs: 600_000,
          // a player who stops answering looks locked: no lock check then
          stuckMs: RUN_OUT === undefined ? 20_000 : undefined,
          userData: toUserData(settings),
          table: TIME_LIMIT ? { matchTimeLimit: TIME_LIMIT, matchBufferTime: BUFFER_TIME ?? 'NONE' } : undefined,
          ...(RUN_OUT === undefined ? {} : { holdAfterTurn: RUN_OUT, holdUntil: Date.now() + 3_600_000 }),
          intercept(state) {
            const me = state.view?.players?.find((p) => p.playerId === state.view?.myPlayerId);
            if (me?.timerActive) clock = Math.min(clock, me.priorityTimeLeftSecs ?? NO_LIMIT);
            const answer = autoAnswer(state.view, state.prompt, {
              autoPass: settings.autoPass,
              autoSkipCombat: settings.autoSkipCombat,
              abilitiesOnTheirTurn: settings.abilitiesOnTheirTurn,
              combatStops: settings.combatStops,
              beginCombatStop: !!settings.stops.yourTurn?.beforeCombat,
            });
            return own(state) ?? (answer ? COMMANDS[answer] : undefined);
          },
        });
        results.push(`game ${game} ${mine.name} vs AI ${theirs.name}: turn ${stats.maxTurn}, ${stats.commands} answers, `
          + `${stats.spellsCast} spells, ${stats.attacks} attacks, over ${stats.gameOver}`);
        if (!stats.gameOver) failures.push(`game ${game}: not over`);
        if (TIME_LIMIT && clock === NO_LIMIT) failures.push(`game ${game}: the views showed no priority timer`);
        if (!TIME_LIMIT && clock !== NO_LIMIT) failures.push(`game ${game}: a clock (${clock}s) without a time limit`);
        if (TIME_LIMIT) results.push(`  clock down to ${clock}s`);
      } catch (e) {
        failures.push(`game ${game} ${mine.name} vs AI ${theirs.name} (${userName}): ${(e as Error).message}`);
      }
      if (SERVER_LOG) {
        const log = readFileSync(SERVER_LOG, 'utf8');
        if (!log.includes(`Rust host: plays ${userName}'s game`)) {
          const why = log.split('\n').filter((l) => l.includes(`${userName}'s game`)).pop() ?? 'no line';
          failures.push(`game ${game} (${userName}) wasn't hosted on Rust: ${why.slice(0, 300)}`);
        }
        // each game so far ended on the timer (a fresh log)
        if (RUN_OUT !== undefined && (log.match(/ended after \d+ turns, timed out/g) ?? []).length < game + 1) {
          failures.push(`game ${game} (${userName}) didn't end on the timer`);
        }
      }
    }
    console.log(`[soak seed ${SEED}]\n${results.join('\n')}\n${failures.length} failed:\n${failures.join('\n')}`);
    expect(failures).toEqual([]);
  }, 3_600_000 * 4);
});
