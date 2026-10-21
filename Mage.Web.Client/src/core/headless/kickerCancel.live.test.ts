/**
 * Cancelling the mana payment of a kicked spell undoes the whole cast: the card goes back to hand and nothing
 * resolves. Casts Burst Lightning with kicker at the AI, answers Cancel on the payment, and watches what happens.
 *   MAGE_LIVE_SERVER=1 npx vitest run src/core/headless/kickerCancel
 */
import { describe, expect, test } from 'vitest';
import { deck, playBotGame } from './botGame';
import type { Command } from '../game/interaction';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17172';
const live = process.env.MAGE_LIVE_SERVER === '1';

const BURN = deck('Kicker', [
  [30, 'Mountain'],
  [30, 'Burst Lightning'],
]);

describe.skipIf(!live)('a kicked spell cancelled while casting', () => {
  test.each([['target'], ['pay'], ['payAfterTap']] as const)('cancelled at %s: the spell goes back to hand and does not resolve', async (cancelAt) => {
    const log: string[] = [];
    let phase: 'wait' | 'casting' | 'cancelled' | 'done' = 'wait';
    let castId: string | null = null;
    let lifeBefore = 0;
    let watchUntilTurn = 0;
    let after: { inHand: boolean; stack: string[]; life: number } | null = null;
    let tapped = false;

    await playBotGame({
      serverUrl: SERVER_URL,
      userName: `kick${Date.now() % 100000}`,
      deck: BURN,
      maxTurns: 14,
      timeoutMs: 600_000,
      intercept(state): Command | undefined {
        const view = state.view;
        if (!view) return undefined;
        const me = view.players?.find((p) => p.playerId === view.myPlayerId);
        const opponent = view.players?.find((p) => p.playerId !== view.myPlayerId);
        const hand = view.myHand ?? {};
        const untapped = Object.values(me?.battlefield ?? {}).filter((card) => card.cardTypes?.includes('LAND') && !card.tapped).length;
        const mode = state.interaction.mode;
        if (mode !== 'waiting') log.push(`t${view.turn} ${view.step} ${mode} "${state.interaction.headline}" phase=${phase} life=${opponent?.life} stack=[${Object.values(view.stack ?? {}).map((s) => s.name).join(',')}]`);

        if (phase === 'cancelled') {
          // the next time we get priority: what is left of the cast
          if (mode === 'priority' || mode === 'waiting') {
            after = { inHand: !!castId && !!hand[castId], stack: Object.values(view.stack ?? {}).map((s) => s.name ?? '?'), life: opponent?.life ?? 0 };
            watchUntilTurn = (view.turn ?? 0) + 2;
            phase = 'done';
          }
        }
        if (phase === 'done') {
          if (after) after.life = Math.min(after.life, opponent?.life ?? after.life);
          if ((view.turn ?? 0) >= watchUntilTurn && mode === 'priority') return { type: 'action', action: 'CONCEDE' };
          // keep the remaining burn in hand: only lands from here on
          if (mode === 'priority') {
            const land = [...state.interaction.clickable.keys()].find((id) => hand[id]?.cardTypes?.includes('LAND'));
            return land ? { type: 'uuid', id: land } : { type: 'boolean', value: false };
          }
          return undefined;
        }

        if (mode === 'priority' && view.activePlayerId === view.myPlayerId) {
          const land = [...state.interaction.clickable.keys()].find((id) => hand[id]?.cardTypes?.includes('LAND'));
          if (land) return { type: 'uuid', id: land };
          const bolt = [...state.interaction.clickable.keys()].find((id) => hand[id]?.name === 'Burst Lightning');
          if (phase === 'wait' && bolt && untapped >= 5 && view.step?.includes('MAIN')) {
            castId = bolt;
            lifeBefore = opponent?.life ?? 0;
            phase = 'casting';
            return { type: 'uuid', id: bolt };
          }
          return { type: 'boolean', value: false };
        }
        if (mode === 'priority') return { type: 'boolean', value: false };
        if (phase === 'casting' && state.prompt?.kind === 'ask') return { type: 'boolean', value: true }; // pay kicker
        if (phase === 'casting' && cancelAt === 'target' && (mode === 'target' || mode === 'pickCards')) {
          phase = 'cancelled';
          return { type: 'uuid', id: null };
        }
        if (phase === 'casting' && mode === 'payMana') {
          if (cancelAt === 'payAfterTap' && !tapped) {
            tapped = true;
            return undefined; // the bot taps one source
          }
          phase = 'cancelled';
          return { type: 'boolean', value: false };
        }
        return undefined;
      },
    });
    console.log(`[kicker cancel at ${cancelAt}] life before ${lifeBefore}, after ${JSON.stringify(after)}\n${log.join('\n')}`);
    expect(after).not.toBeNull();
    expect(after!.inHand).toBe(true);
    expect(after!.stack).toEqual([]);
    expect(after!.life).toBe(lifeBefore);
  }, 900_000);
});
