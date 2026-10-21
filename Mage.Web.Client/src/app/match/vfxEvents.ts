import type { GameEventView } from '../../protocol/generated/views';

/** What one batch of game events shows on the board: numbers thrown off cards and players, and bolts. */
export interface EventVisuals {
  /** total damage per card or player */
  damage: Map<string, number>;
  /** life gained per player */
  healed: Map<string, number>;
  /** life lost without damage (paid life, drain), per player */
  drained: Map<string, number>;
  /** noncombat damage, drawn from its source to what it hit */
  bolts: { from: string; to: string }[];
  /** combat damage, once per source and target: where an earned hit spark bursts (B300) */
  hits: { from: string; to: string }[];
}

function add(map: Map<string, number>, id: string | undefined, amount: number | undefined) {
  if (!id || !amount) return;
  map.set(id, (map.get(id) ?? 0) + amount);
}

export function eventVisuals(events: readonly GameEventView[]): EventVisuals {
  const visuals: EventVisuals = { damage: new Map(), healed: new Map(), drained: new Map(), bolts: [], hits: [] };
  const lost = new Map<string, number>();
  const bolted = new Set<string>();
  const struck = new Set<string>();
  for (const event of events) {
    switch (event.kind) {
      case 'DAMAGE':
        add(visuals.damage, event.targetId, event.amount);
        if (!event.combat && event.sourceId && event.targetId && event.sourceId !== event.targetId) {
          const key = `${event.sourceId}>${event.targetId}`;
          if (!bolted.has(key)) {
            bolted.add(key);
            visuals.bolts.push({ from: event.sourceId, to: event.targetId });
          }
        }
        if (event.combat && event.amount && event.sourceId && event.targetId) {
          const key = `${event.sourceId}>${event.targetId}`;
          if (!struck.has(key)) {
            struck.add(key);
            visuals.hits.push({ from: event.sourceId, to: event.targetId });
          }
        }
        break;
      case 'LIFE_GAIN':
        add(visuals.healed, event.playerId ?? event.targetId, event.amount);
        break;
      case 'LIFE_LOSS':
        add(lost, event.playerId ?? event.targetId, event.amount);
        break;
      default:
        break;
    }
  }
  // damage to a player also makes them lose life: show it once, as damage
  for (const [player, amount] of lost) {
    const rest = amount - (visuals.damage.get(player) ?? 0);
    if (rest > 0) visuals.drained.set(player, rest);
  }
  return visuals;
}
