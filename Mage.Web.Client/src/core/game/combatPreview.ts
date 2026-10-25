import type { CardView, GameView, PermanentView } from '../../protocol/generated/views';
import { keywordMarks } from './keywords';

/**
 * An estimate of how combat ends for the blocks (or attacks) as they stand, from what the board shows: who dies,
 * what gets through, and each player's life after combat damage.
 *
 * Handled: first strike and double strike (two damage steps; a creature killed in the first deals no regular damage),
 * deathtouch (1 damage is lethal), trample (lethal to every blocker, the rest to the player; all of it once its
 * blockers are gone), lifelink, infect (-1/-1 counters on creatures, poison on players), indestructible, menace (a
 * single blocker is flagged as illegal), damage already marked this turn.
 * Not handled: tricks, triggers, static effects that change damage (prevention, protection, "can't be blocked"),
 * banding. A blocker blocking several attackers has its damage counted against each.
 * Assumption: an attacker with several blockers divides its damage to kill as many of them as it can, the ones
 * needing the least damage first (the attacker's choice; this is the most likely one, not the only one).
 */

export interface Fighter {
  id: string;
  /** empty for a face-down creature whose name this player can't see */
  name: string;
  power: number;
  toughness: number;
  /** damage already marked on it this turn */
  marked: number;
  first: boolean;
  double: boolean;
  deathtouch: boolean;
  trample: boolean;
  lifelink: boolean;
  infect: boolean;
  indestructible: boolean;
  menace: boolean;
  faceDown: boolean;
  controllerId?: string;
}

function toInt(value: unknown): number {
  const number = Number.parseInt(String(value ?? ''), 10);
  return Number.isNaN(number) ? 0 : number;
}

/** A creature for combat arithmetic: its current power, toughness, marked damage and combat keywords. */
export function fighterOf(card: CardView | PermanentView, id = card.id ?? ''): Fighter {
  const marks = new Set(keywordMarks(card).map((mark) => mark.name));
  const double = marks.has('Double strike');
  return {
    id,
    name: card.name || card.displayName || '',
    power: Math.max(0, toInt(card.power)),
    toughness: toInt(card.toughness),
    marked: (card as PermanentView).damage ?? 0,
    first: double || marks.has('First strike'),
    double,
    deathtouch: marks.has('Deathtouch'),
    trample: marks.has('Trample'),
    lifelink: marks.has('Lifelink'),
    infect: marks.has('Infect'),
    indestructible: marks.has('Indestructible'),
    menace: marks.has('Menace'),
    faceDown: !!card.faceDown,
    controllerId: card.controllerId,
  };
}

export interface FightResult {
  attackerDies: boolean;
  /** ids of the blockers that die, in blocker order */
  deadBlockers: string[];
  /** combat damage the attacker deals to the player or planeswalker it attacks (unblocked, or trample) */
  toDefender: number;
  /** life the attacker's controller gains (attacker lifelink) */
  attackerGain: number;
  /** life the defender gains (blockers' lifelink) */
  defenderGain: number;
}

interface Hurt {
  damage: number;
  /** -1/-1 counters from infect */
  shrink: number;
  /** dealt damage by a deathtouch source */
  touched: boolean;
}

function dead(fighter: Fighter, hurt: Hurt): boolean {
  const toughness = fighter.toughness - hurt.shrink;
  if (toughness <= 0) return true; // 0 toughness: indestructible doesn't help
  if (fighter.indestructible) return false;
  return fighter.marked + hurt.damage >= toughness || (hurt.touched && hurt.damage + hurt.shrink > 0);
}

/** Damage still needed to destroy it (deathtouch: one point); 0 when it already has lethal damage. */
function lethalLeft(fighter: Fighter, hurt: Hurt, deathtouch: boolean): number {
  if (dead({ ...fighter, indestructible: false }, hurt)) return 0;
  if (deathtouch) return 1;
  return Math.max(0, fighter.toughness - hurt.shrink - fighter.marked - hurt.damage);
}

function deal(source: Fighter, target: Hurt, amount: number) {
  if (amount <= 0) return;
  if (source.infect) target.shrink += amount;
  else target.damage += amount;
  if (source.deathtouch) target.touched = true;
}

/**
 * One attacker against its blockers (none: unblocked), without tricks. Two damage steps when any of them has first
 * or double strike. The attacker divides its damage to kill as many blockers as it can, cheapest first; an
 * indestructible blocker gets lethal damage last (trample needs it before the rest goes through).
 */
export function fight(attacker: Fighter, blockers: readonly Fighter[]): FightResult {
  const result: FightResult = { attackerDies: false, deadBlockers: [], toDefender: 0, attackerGain: 0, defenderGain: 0 };
  if (blockers.length === 0) {
    const hits = attacker.double ? 2 : 1;
    result.toDefender = attacker.power * hits;
    if (attacker.lifelink) result.attackerGain = attacker.power * hits;
    return result;
  }
  const attackerHurt: Hurt = { damage: 0, shrink: 0, touched: false };
  const hurt = new Map(blockers.map((blocker) => [blocker.id, { damage: 0, shrink: 0, touched: false } as Hurt]));
  let alive = [...blockers];
  let attackerAlive = true;
  const twoSteps = attacker.first || blockers.some((blocker) => blocker.first);
  for (const step of twoSteps ? ['first', 'regular'] as const : ['regular'] as const) {
    const strikes = (fighter: Fighter) => (!twoSteps ? true : step === 'first' ? fighter.first : !fighter.first || fighter.double);
    const hitters = alive.filter(strikes);
    // the attacker's damage
    if (attackerAlive && strikes(attacker) && attacker.power > 0) {
      let left = attacker.power;
      let dealt = 0;
      if (alive.length === 0) {
        // every blocker is gone: a trampler deals all of it to the player, others deal none
        if (attacker.trample) {
          result.toDefender += left;
          dealt += left;
        }
      } else {
        const order = [...alive].sort((a, b) => Number(a.indestructible) - Number(b.indestructible)
          || lethalLeft(a, hurt.get(a.id)!, attacker.deathtouch) - lethalLeft(b, hurt.get(b.id)!, attacker.deathtouch));
        const given = new Map<string, number>();
        for (const blocker of order) {
          const need = lethalLeft(blocker, hurt.get(blocker.id)!, attacker.deathtouch);
          if (need > left) break;
          given.set(blocker.id, need);
          left -= need;
        }
        const allLethal = given.size === order.length;
        if (attacker.trample && allLethal) {
          result.toDefender += left;
          dealt += left;
        } else if (left > 0) {
          // not enough to kill the next one: the rest goes on it (it may die to the second strike)
          const next = order.find((blocker) => !given.has(blocker.id) || given.get(blocker.id) === 0) ?? order[0];
          given.set(next.id, (given.get(next.id) ?? 0) + left);
        }
        for (const [id, amount] of given) {
          deal(attacker, hurt.get(id)!, amount);
          dealt += amount;
        }
      }
      if (attacker.lifelink) result.attackerGain += dealt;
    }
    // the blockers' damage, all to the attacker
    if (attackerAlive) {
      for (const blocker of hitters) {
        if (blocker.power <= 0) continue;
        deal(blocker, attackerHurt, blocker.power);
        if (blocker.lifelink) result.defenderGain += blocker.power;
      }
    }
    alive = alive.filter((blocker) => !dead(blocker, hurt.get(blocker.id)!));
    attackerAlive = attackerAlive && !dead(attacker, attackerHurt);
  }
  result.attackerDies = !attackerAlive;
  result.deadBlockers = blockers.filter((blocker) => !alive.includes(blocker)).map((blocker) => blocker.id);
  return result;
}

export interface PairPreview {
  attacker: Fighter;
  blockers: Fighter[];
  result: FightResult;
  /** who the attacker attacks: a player id, or a planeswalker or battle */
  defenderId: string;
  defenderName: string;
  /** menace with one blocker: the server will refuse it */
  menaceAlone: boolean;
}

export interface DefenderTotal {
  id: string;
  name: string;
  isPlayer: boolean;
  isMe: boolean;
  damage: number;
  poison: number;
  /** life before and after combat damage (players only) */
  life: number | null;
  lifeAfter: number | null;
}

export interface CombatPreview {
  pairs: PairPreview[];
  defenders: DefenderTotal[];
  /** creature id -> dies or survives */
  fate: Map<string, 'dies' | 'survives'>;
  /** life change of each player from lifelink, by player id */
  gains: Map<string, number>;
  /** blockers blocking more than one attacker, and how many */
  sharedBlockers: { name: string; count: number }[];
  /** a face-down creature is in combat: it may be turned face up */
  faceDown: boolean;
  /** at least one attacker has more than one blocker: the damage split is assumed */
  assumedSplit: boolean;
}

function battlefieldCard(view: GameView, id: string): PermanentView | undefined {
  for (const player of view.players ?? []) {
    const permanent = player.battlefield?.[id];
    if (permanent) return permanent;
  }
  return undefined;
}

function controllerOf(view: GameView, id: string): string | undefined {
  return (view.players ?? []).find((player) => !!player.battlefield?.[id])?.playerId;
}

/**
 * The estimate for the combat in the view (the attackers and blockers declared so far), or null when nobody attacks.
 * Each creature is read from the battlefield when it is there (current power, toughness, damage), else from the combat
 * group.
 */
export function combatPreview(view: GameView | null | undefined): CombatPreview | null {
  const groups = view?.combat ?? [];
  if (!view || groups.length === 0) return null;
  const me = view.myPlayerId;
  const pairs: PairPreview[] = [];
  const blockCount = new Map<string, number>();
  for (const group of groups) {
    const blockers = Object.entries(group.blockers ?? {}).map(([id, card]) => {
      blockCount.set(id, (blockCount.get(id) ?? 0) + 1);
      return { ...fighterOf(battlefieldCard(view, id) ?? card, id), controllerId: card.controllerId ?? controllerOf(view, id) };
    });
    for (const [id, card] of Object.entries(group.attackers ?? {})) {
      const attacker = { ...fighterOf(battlefieldCard(view, id) ?? card, id), controllerId: card.controllerId ?? controllerOf(view, id) };
      const defenderId = group.defenderId ?? '';
      const permanent = battlefieldCard(view, defenderId);
      pairs.push({
        attacker,
        blockers,
        result: fight(attacker, blockers),
        defenderId,
        defenderName: group.defenderName ?? permanent?.name ?? 'defender',
        menaceAlone: attacker.menace && blockers.length === 1,
      });
    }
  }
  if (pairs.length === 0) return null;

  const fate = new Map<string, 'dies' | 'survives'>();
  const totals = new Map<string, DefenderTotal>();
  const gains = new Map<string, number>();
  const addGain = (playerId: string | undefined, amount: number) => {
    if (playerId && amount > 0) gains.set(playerId, (gains.get(playerId) ?? 0) + amount);
  };
  for (const pair of pairs) {
    if (!fate.has(pair.attacker.id) || pair.result.attackerDies) fate.set(pair.attacker.id, pair.result.attackerDies ? 'dies' : 'survives');
    for (const blocker of pair.blockers) {
      const dies = pair.result.deadBlockers.includes(blocker.id);
      if (!fate.has(blocker.id) || dies) fate.set(blocker.id, dies ? 'dies' : 'survives');
    }
    addGain(pair.attacker.controllerId, pair.result.attackerGain);
    addGain(pair.blockers[0]?.controllerId ?? (pair.blockers.length > 0 ? pair.defenderId : undefined), pair.result.defenderGain);
    let total = totals.get(pair.defenderId);
    if (!total) {
      const player = (view.players ?? []).find((candidate) => candidate.playerId === pair.defenderId);
      total = {
        id: pair.defenderId,
        name: player ? player.name ?? 'Opponent' : pair.defenderName,
        isPlayer: !!player,
        isMe: !!player && player.playerId === me,
        damage: 0,
        poison: 0,
        life: player?.life ?? null,
        lifeAfter: null,
      };
      totals.set(pair.defenderId, total);
    }
    if (pair.attacker.infect && total.isPlayer) total.poison += pair.result.toDefender;
    else total.damage += pair.result.toDefender;
  }
  for (const total of totals.values()) {
    if (total.life !== null) total.lifeAfter = total.life - total.damage + (gains.get(total.id) ?? 0);
  }

  const sharedBlockers = [...blockCount].filter(([, count]) => count > 1).map(([id, count]) => ({
    name: pairs.flatMap((pair) => pair.blockers).find((blocker) => blocker.id === id)?.name ?? '',
    count,
  }));
  return {
    pairs,
    defenders: [...totals.values()],
    fate,
    gains,
    sharedBlockers,
    faceDown: pairs.some((pair) => pair.attacker.faceDown || pair.blockers.some((blocker) => blocker.faceDown)),
    assumedSplit: pairs.some((pair) => pair.blockers.length > 1),
  };
}
