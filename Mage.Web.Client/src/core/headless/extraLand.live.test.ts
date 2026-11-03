/**
 * Playmat reports #161 and #162, played through the client core against a live server:
 * - with Dryad of the Ilysian Grove out, a second land can be played in the same turn;
 * - with Leyline of the Guildpact and Leyline of Mutation out, five Forests cast a seven-mana spell for {W}{U}{B}{R}{G}.
 *   MAGE_LIVE_SERVER=1 npx vitest run src/core/headless/extraLand
 */
import { describe, expect, test } from 'vitest';
import { deck, playBotGame } from './botGame';
import type { Command } from '../game/interaction';
import type { GameView } from '../../protocol/generated/views';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17172';
const live = process.env.MAGE_LIVE_SERVER === '1';

function mine(view: GameView) {
  const me = view.players?.find((player) => player.playerId === view.myPlayerId);
  const permanents = Object.values(me?.battlefield ?? {});
  return {
    lands: permanents.filter((card) => card.cardTypes?.includes('LAND')).length,
    untapped: permanents.filter((card) => card.cardTypes?.includes('LAND') && !card.tapped).length,
    has: (name: string) => permanents.some((card) => card.name === name),
  };
}

describe.skipIf(!live)('extra lands and alternative costs', () => {
  test('Dryad of the Ilysian Grove: two lands in one turn (#161)', async () => {
    const log: string[] = [];
    let landsThisTurn = 0;
    let turn = 0;
    let best = 0;
    let dryadTurns = 0;
    await playBotGame({
      serverUrl: SERVER_URL,
      userName: `dry${Date.now() % 100000}`,
      deck: deck('Dryad', [[24, 'Dryad of the Ilysian Grove'], [36, 'Forest']]),
      maxTurns: 14,
      timeoutMs: 600_000,
      intercept(state): Command | undefined {
        const view = state.view;
        if (!view || state.interaction.mode !== 'priority') return undefined;
        if (view.activePlayerId !== view.myPlayerId || !view.step?.includes('MAIN')) return { type: 'boolean', value: false };
        if (view.turn !== turn) {
          turn = view.turn ?? 0;
          landsThisTurn = 0;
          if (mine(view).has('Dryad of the Ilysian Grove')) dryadTurns++;
        }
        const hand = view.myHand ?? {};
        const clickable = [...state.interaction.clickable.keys()];
        const land = clickable.find((id) => hand[id]?.cardTypes?.includes('LAND'));
        const landsInHand = Object.values(hand).filter((card) => card.cardTypes?.includes('LAND')).length;
        if (mine(view).has('Dryad of the Ilysian Grove')) {
          log.push(`t${view.turn} ${view.step} played=${landsThisTurn} landsInHand=${landsInHand} landClickable=${!!land}`);
        }
        if (land && landsThisTurn < 2) {
          landsThisTurn++;
          best = Math.max(best, landsThisTurn);
          return { type: 'uuid', id: land };
        }
        const dryad = clickable.find((id) => hand[id]?.name === 'Dryad of the Ilysian Grove');
        if (dryad && !mine(view).has('Dryad of the Ilysian Grove')) return { type: 'uuid', id: dryad };
        return { type: 'boolean', value: false };
      },
    });
    console.log(log.join('\n'));
    expect(dryadTurns, 'Dryad never reached the battlefield').toBeGreaterThan(0);
    expect(best).toBe(2);
  }, 600_000);

  test('Leyline of Mutation with Leyline of the Guildpact: five Forests cast a seven-drop (#162)', async () => {
    const log: string[] = [];
    let wurmClickableWithFive = false;
    let sawFiveLandsAndWurm = false;
    let cast = false;
    let owed = '';
    let wurmOut = false;
    await playBotGame({
      serverUrl: SERVER_URL,
      userName: `ley${Date.now() % 100000}`,
      deck: deck('Leylines', [[9, 'Leyline of the Guildpact'], [9, 'Leyline of Mutation'], [12, 'Pelakka Wurm'], [30, 'Forest']]),
      maxTurns: 30,
      timeoutMs: 600_000,
      intercept(state): Command | undefined {
        const view = state.view;
        if (!view) return undefined;
        const mode = state.interaction.mode;
        // the alternative cost menu: pay {W}{U}{B}{R}{G}
        const WUBRG = /\{W\}\{U\}\{B\}\{R\}\{G\}/;
        if (cast && state.prompt?.kind === 'chooseChoice') {
          const keyed = Object.entries(state.prompt.choice.keyChoices ?? {}).find(([, label]) => WUBRG.test(String(label)));
          if (keyed) return { type: 'string', value: keyed[0] };
          const plain = (state.prompt.choice.choices ?? []).find((label) => WUBRG.test(label));
          if (plain) return { type: 'string', value: plain };
        }
        // each Forest is every basic land type: make the color the payment still owes
        if (state.prompt?.kind === 'playMana') owed = state.interaction.headline;
        if (cast && state.prompt?.kind === 'chooseChoice' && state.prompt.choice.manaColorChoice) {
          const names: Record<string, string> = { W: 'White', U: 'Blue', B: 'Black', R: 'Red', G: 'Green' };
          const symbol = (owed.match(/\{([WUBRG])\}/) ?? [])[1];
          if (symbol) return { type: 'string', value: names[symbol] };
        }
        if (cast && state.prompt?.kind === 'chooseAbility') {
          const ability = state.prompt.choices.find((choice) => WUBRG.test(choice.text));
          if (ability) return { type: 'uuid', id: ability.id };
        }
        if (mode !== 'priority') return undefined;
        if (cast && mine(view).has('Pelakka Wurm')) {
          wurmOut = true;
          return { type: 'action', action: 'CONCEDE' };
        }
        if (view.activePlayerId !== view.myPlayerId || !view.step?.includes('MAIN')) return { type: 'boolean', value: false };
        const hand = view.myHand ?? {};
        const clickable = [...state.interaction.clickable.keys()];
        const board = mine(view);
        const find = (name: string) => clickable.find((id) => hand[id]?.name === name);
        const land = clickable.find((id) => hand[id]?.cardTypes?.includes('LAND'));
        const both = board.has('Leyline of the Guildpact') && board.has('Leyline of Mutation');
        const wurmInHand = Object.values(hand).some((card) => card.name === 'Pelakka Wurm');
        if (both && wurmInHand && board.untapped === 5) {
          sawFiveLandsAndWurm = true;
          const wurm = find('Pelakka Wurm');
          log.push(`t${view.turn} untapped=${board.untapped} wurmClickable=${!!wurm}`);
          if (wurm) {
            wurmClickableWithFive = true;
            cast = true;
            return { type: 'uuid', id: wurm };
          }
        }
        // never more than five lands, so the Wurm has to be paid with exactly five
        if (land && board.lands < 5) return { type: 'uuid', id: land };
        if (!board.has('Leyline of the Guildpact') && find('Leyline of the Guildpact')) return { type: 'uuid', id: find('Leyline of the Guildpact')! };
        if (board.has('Leyline of the Guildpact') && !board.has('Leyline of Mutation') && find('Leyline of Mutation')) return { type: 'uuid', id: find('Leyline of Mutation')! };
        return { type: 'boolean', value: false };
      },
    });
    console.log(log.join('\n'));
    expect(sawFiveLandsAndWurm, 'never had both Leylines, five lands and the Wurm').toBe(true);
    expect(wurmClickableWithFive).toBe(true);
    expect(wurmOut, 'the Wurm was paid with five Forests and resolved').toBe(true);
  }, 600_000);
});
