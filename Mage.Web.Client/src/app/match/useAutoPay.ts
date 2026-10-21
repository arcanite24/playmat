import { useEffect, useMemo, useRef } from 'react';
import { commandersByPlayer } from '../../core/game/commander';
import { abilityFor, colorChoiceFor, handNeeds, landsPartOf, manaOf, manaTier, nextTap, orderReadings, parseCostOptions, type ManaCostParts, type ManaReach, type ManaSource, type ManaTap } from '../../core/game/autoPay';
import { parsePayment } from '../../core/game/payment';
import type { GameSession, GameSessionState } from '../../core/game/gameSession';
import type { PlayerBoard } from './boardModel';

const TAP_DELAY_MS = 220;

/**
 * Pays mana costs for the player when the payment is unambiguous: taps one source, waits for the server to ask
 * for the rest, and repeats. When the server asks which of a dual land's abilities to use, answers with the color
 * the plan tapped it for. Stops (leaving the payment to the player) as soon as a real choice would be needed or a
 * tap doesn't change what's still owed.
 */
export interface AutoPayOptions extends ManaReach {
  /** hybrid, twobrid and Phyrexian symbols (Phyrexian with mana only) */
  hybrid?: boolean;
  /** the cost of a spell whose X was just announced */
  afterX?: boolean;
}

export function useAutoPay(session: GameSession, state: GameSessionState, me: PlayerBoard | null, enabled: boolean, options: AutoPayOptions = {}) {
  const lastAsk = useRef<{ text: string; tries: number } | null>(null);
  const lastTap = useRef<(ManaTap & { answered?: boolean }) | null>(null);
  /** the player announced X for the spell being paid */
  const xAnnounced = useRef(false);
  const { interaction, awaitingServer, view } = state;
  const prompt = interaction.prompt;
  const { hybrid = false, afterX = true, anyColor = false, painful = false } = options;
  const commander = useMemo(() => !!state.playerId && commandersByPlayer(view).has(state.playerId), [view, state.playerId]);

  useEffect(() => {
    if (prompt?.kind === 'amount' && /announce the value for/i.test(prompt.text)) xAnnounced.current = true;
    else if (interaction.mode === 'priority') xAnnounced.current = false;
    // the server asks which mana ability of the land we just tapped: the one making the planned color
    if (enabled && !awaitingServer && prompt?.kind === 'chooseAbility' && lastTap.current && !lastTap.current.answered) {
      const ability = abilityFor(prompt.choices, lastTap.current.color);
      lastTap.current = ability ? { ...lastTap.current, answered: true } : null;
      if (ability) void session.respond({ type: 'uuid', id: ability }).catch(() => undefined);
      return;
    }
    // ...or which color an any-color source should make
    if (enabled && !awaitingServer && prompt?.kind === 'chooseChoice' && prompt.choice.manaColorChoice && lastTap.current) {
      const color = colorChoiceFor(prompt.choice.choices ?? [], lastTap.current.color);
      lastTap.current = null;
      if (color) void session.respond({ type: 'string', value: color }).catch(() => undefined);
      return;
    }
    if (!enabled || !me || awaitingServer || interaction.mode !== 'payMana' || prompt?.kind !== 'playMana' || prompt.isX || (xAnnounced.current && !afterX)) {
      if (interaction.mode !== 'payMana') {
        lastAsk.current = null;
        if (prompt?.kind !== 'chooseAbility' && prompt?.kind !== 'chooseChoice') lastTap.current = null;
      }
      return;
    }
    // a new payment request: the last tap was answered, so a later ability question is the player's own
    lastTap.current = null;
    const text = prompt.text;
    // the same request again after a tap means the tap didn't help: hand the payment back to the player
    if (lastAsk.current?.text === text && lastAsk.current.tries >= 1) return;
    const readings = parseCostOptions(text, hybrid);
    if (!readings) return;

    const permanents = new Map(me.back.concat(me.front).flatMap((group) => group.members).map((permanent) => [permanent.id!, permanent]));
    // delve, convoke, improvise: lands pay what those can't, and the rest waits for the player's own choice of cards
    const untapped = (type: string) => [...permanents.values()].filter((permanent) => !permanent.tapped && (permanent.cardTypes ?? []).includes(type as never)).length;
    const payers = { graveyard: Object.keys(me.player.graveyard ?? {}).length, creatures: untapped('CREATURE'), artifacts: untapped('ARTIFACT') };
    const costs = readings.map((fullCost) => (prompt.specialLabel ? landsPartOf(fullCost, prompt.specialLabel, payers) : fullCost));
    // a manland paying for its own animation must not tap itself: it would come alive tapped
    const payingFor = parsePayment(prompt).sourceId;
    const sources: ManaSource[] = [];
    for (const id of interaction.clickable.keys()) {
      const permanent = permanents.get(id);
      if (!permanent || permanent.tapped || id === payingFor) continue;
      const colors = manaOf(permanent, { anyColor, painful, commander });
      if (colors === undefined) continue;
      sources.push({ id, colors, isLand: (permanent.cardTypes ?? []).includes('LAND'), tier: manaTier(permanent) });
    }
    // generic mana leaves untapped the colors the rest of the hand still needs this turn
    const handCosts = Object.values(view?.myHand ?? {}).map((card) => (card.manaCostLeftStr ?? []).join(''));
    if (costs.some((cost) => !cost)) return; // lands have nothing to pay: the special payment is the player's
    let next: ManaTap | null = null;
    // a hybrid cost: the reading the plainest sources can pay, and of those the one plain sources cover best
    const tiers = [...new Set(sources.map((source) => source.tier ?? 0))].sort((a, b) => a - b);
    search: for (const tier of tiers) {
      const pool = sources.filter((source) => (source.tier ?? 0) <= tier);
      for (const cost of orderReadings(costs as ManaCostParts[], pool)) {
        const owed = cost.generic + Object.values(cost.colored).reduce((sum, value) => sum + (value ?? 0), 0);
        next = nextTap(cost, pool, handNeeds(handCosts, pool.length - owed));
        if (next) break search;
      }
    }
    if (!next) return;
    const tap = next;

    const timer = setTimeout(() => {
      lastAsk.current = { text, tries: lastAsk.current?.text === text ? lastAsk.current.tries + 1 : 1 };
      lastTap.current = tap;
      session.click(tap.id);
    }, TAP_DELAY_MS);
    return () => clearTimeout(timer);
  }, [enabled, me, awaitingServer, interaction, prompt, session, view, hybrid, afterX, anyColor, painful, commander]);
}
