import { useEffect, useRef, useState, type CSSProperties } from 'react';
import type { CardView, CombatGroupView, GameView, PermanentView } from '../../protocol/generated/views';
import { useSettings } from '../stores/settings';
import { resolvedCasts } from './cardVfx';
import { holdFlight, landingIn, lastPlacement, motionAllowed } from './flip';
import { BOLT_MS, noteBeats, signatureImpact } from './beats';
import { cardKey } from './boardModel';
import { SignatureVfx } from './SignatureVfx';
import { lifetimeOf, type Point, type SignatureEffect } from './signatureShapes';
import { eventVisuals } from './vfxEvents';
import { fighterOf } from '../../core/game/combatPreview';
import { combatSide, hitWeight, stableDefender, strikesFromCombat, strikesFromEvents, WEIGHT_RANK, type HitWeight, type Strike } from '../../core/game/combatStrikes';
import { recoil, shake, swing, TABLE_SHAKE, thump } from './combatMotion';
import { matchLayout } from './matchLayout';
import { TriggerSignals } from './TriggerSignals';
import { useObjectCenter, useStage } from './stageContext';
import styles from './Vfx.module.css';

/**
 * Game effects, printed in the mat's inks: damage splashes in oxblood with the number thrown off the card, healing
 * in sage, shockwaves where cards land, bursts where spells resolve, lunges when combat damage is dealt and ink
 * dissolving where creatures die. Damage, healing and the bolts of noncombat damage come from the game events a view
 * carries (exact even when a creature dies in the same update); the rest, and servers without events, from the
 * difference between two game views. A player who earned hit sparks (B300) throws a brief burst of them, in the
 * spark's color, where their creatures' combat damage lands.
 *
 * Combat (#413) is drawn strike by strike (combatStrikes.ts): each attacker swings at each creature it hits and its
 * blockers swing back, a double striker swings twice, trample carries on to the player, and the number and splash land
 * when the swing connects. With fancy combat effects (the default) a hit is weighed by its damage: a big one winds up,
 * holds on contact, knocks its target back and shakes the table, and the keywords show what they did (a slash for
 * first strike, a dark mark for deathtouch, motes of life for lifelink). The board gets heavier landings for big
 * creatures, puffs for tokens, counters popping onto cards and beams for planeswalker abilities.
 *
 * The spells and abilities people play most also get an effect of their own when they resolve (cardVfx.ts,
 * SignatureVfx.tsx): lightning for Lightning Bolt, a glyph for Counterspell, a sweep of light for Wrath of God. The
 * Spell effects setting chooses those, only the plain effects, or no spell effects at all.
 */

/** at most this many signature effects play at once: a storm turn shouldn't bury the board */
const MAX_SIGNATURES = 4;
/** attackers swing one after another, this far apart, up to MAX_STAGGER of them; the rest swing with the last */
const STAGGER = 70;
const MAX_STAGGER = 6;
/** an attacker hitting several blockers swings at each in turn, this far apart */
const SWEEP = 170;
/** the gap between first strike damage and regular damage arriving in one update */
const PHASE_GAP = 140;

type EffectKind =
  | 'damage' | 'heal' | 'enter' | 'resolve' | 'death' | 'charge' | 'bolt' | 'spark'
  | 'dust' | 'crack' | 'slash' | 'void' | 'motes' | 'carry' | 'shield' | 'lethal'
  | 'exile' | 'shatter' | 'puff' | 'counter' | 'pt' | 'beam' | 'attach';

interface Effect {
  key: number;
  kind: EffectKind;
  x: number;
  y: number;
  value?: number;
  /** shockwaves and dissolves follow the card's size */
  size?: number;
  /** where a bolt, a stream of motes, a trample carry or a beam lands */
  to?: { x: number; y: number };
  /** a hit spark's color */
  color?: string;
  /** a counter's text ("+1/+1", "−3") */
  label?: string;
  /** a variant: an infect hit, a deathtouch death, a growing or shrinking creature, a heavy landing */
  tone?: 'infect' | 'void' | 'up' | 'down' | 'gold' | 'big';
  /** the direction of a slash, in degrees */
  angle?: number;
}

interface Snapshot {
  permanents: Map<string, PermanentView>;
  life: Map<string, number>;
  players: Set<string>;
  stack: Set<string>;
  /** the stack's objects, for the signature effects of whatever leaves it */
  stackCards: Map<string, CardView>;
  attackers: Set<string>;
  blockers: Set<string>;
  combat: CombatGroupView[];
  step: string | undefined;
}

const LIFETIME: Record<EffectKind, number> = {
  damage: 1300, heal: 1300, enter: 700, resolve: 800, death: 900, charge: 600, bolt: 480, spark: 760,
  dust: 700, crack: 1100, slash: 420, void: 800, motes: 900, carry: 520, shield: 650, lethal: 1400,
  exile: 950, shatter: 950, puff: 800, counter: 1100, pt: 700, beam: 700, attach: 600,
};

function snapshot(view: GameView): Snapshot {
  const permanents = new Map<string, PermanentView>();
  const life = new Map<string, number>();
  const players = new Set<string>();
  for (const player of view.players ?? []) {
    players.add(player.playerId!);
    life.set(player.playerId!, player.life ?? 0);
    for (const permanent of Object.values(player.battlefield ?? {})) permanents.set(permanent.id!, permanent);
  }
  const attackers = new Set<string>();
  const blockers = new Set<string>();
  for (const group of view.combat ?? []) {
    Object.keys(group.attackers ?? {}).forEach((id) => attackers.add(id));
    Object.keys(group.blockers ?? {}).forEach((id) => blockers.add(id));
  }
  return {
    permanents, life, players, stack: new Set(Object.keys(view.stack ?? {})), stackCards: new Map(Object.entries(view.stack ?? {})),
    attackers, blockers, combat: view.combat ?? [], step: view.step,
  };
}

const toInt = (value: unknown) => {
  const parsed = Number.parseInt(String(value ?? ''), 10);
  return Number.isFinite(parsed) ? parsed : 0;
};

const angleOf = (from: Point, to: Point) => (Math.atan2(to.y - from.y, to.x - from.x) * 180) / Math.PI;

export function Vfx({ view, myPlayerId, sparks }: {
  view: GameView | null;
  myPlayerId: string | null;
  /** players who earned hit sparks: player id to the spark's color */
  sparks?: ReadonlyMap<string, string>;
}) {
  const [effects, setEffects] = useState<Effect[]>([]);
  const [signatures, setSignatures] = useState<SignatureEffect[]>([]);
  const cardEffects = useSettings((state) => state.settings.cardEffects);
  const fancy = useSettings((state) => state.settings.combatFx) !== 'simple';
  // where each stack object was drawn on the last view: a resolved spell's effect starts there
  const stackSpots = useRef(new Map<string, Point>());
  // where each permanent stood on the last view: a creature that dies from a hit is struck where it stood (the card's
  // last placement already is its graveyard pile, where it flies)
  const boardSpots = useRef(new Map<string, Point>());
  // read when a view arrives: a look arriving later doesn't replay the last view's effects
  const sparksRef = useRef(sparks);
  useEffect(() => {
    sparksRef.current = sparks;
  }, [sparks]);
  const [hurt, setHurt] = useState(0);
  const previous = useRef<Snapshot | null>(null);
  const nextKey = useRef(0);
  const timers = useRef(new Set<ReturnType<typeof setTimeout>>());
  const center = useObjectCenter();
  const stage = useStage();

  useEffect(() => () => {
    for (const timer of timers.current) clearTimeout(timer);
  }, []);

  useEffect(() => {
    if (!view) return;
    const now = snapshot(view);
    const before = previous.current;
    previous.current = now;
    // measured once this view has painted, for the next one
    const measureStack = () => {
      const spots = new Map<string, Point>();
      for (const id of now.stack) {
        const at = center(id);
        if (at) spots.set(id, at);
      }
      stackSpots.current = spots;
      const board = new Map<string, Point>();
      let moving = false;
      for (const id of now.permanents.keys()) {
        // a card flying in or swinging (scripted animations, not its own CSS ones) is measured once it has settled
        const busy = stage.element?.querySelector(`[data-object-id="${CSS.escape(id)}"]`)?.getAnimations()
          .some((animation) => animation.playState === 'running' && !(animation instanceof CSSAnimation) && !(animation instanceof CSSTransition));
        const at = busy ? boardSpots.current.get(id) : center(id);
        moving ||= !!busy;
        if (at) board.set(id, at);
      }
      boardSpots.current = board;
      if (moving) {
        const timer = setTimeout(() => {
          timers.current.delete(timer);
          if (previous.current !== now) return;
          for (const id of now.permanents.keys()) {
            const at = center(id);
            if (at) boardSpots.current.set(id, at);
          }
        }, 1400);
        timers.current.add(timer);
      }
    };
    if (!before || !motionAllowed()) {
      requestAnimationFrame(measureStack);
      return;
    }

    const after = (ms: number, run: () => void) => {
      if (ms <= 0) {
        run();
        return;
      }
      const timer = setTimeout(() => {
        timers.current.delete(timer);
        run();
      }, ms);
      timers.current.add(timer);
    };
    const show = (list: Effect[]) => {
      setEffects((current) => [...current, ...list]);
      for (const effect of list) {
        after(LIFETIME[effect.kind], () => setEffects((current) => current.filter((item) => item.key !== effect.key)));
      }
    };
    // effects that start now (on the next frame, with the rest of this view's) and effects timed to a hit
    const spawned: Effect[] = [];
    const add = (effect: Omit<Effect, 'key'>) => spawned.push({ ...effect, key: nextKey.current++ });
    const addAt = (ms: number, effect: Omit<Effect, 'key'>) => {
      if (ms <= 0) add(effect);
      else after(ms, () => show([{ ...effect, key: nextKey.current++ }]));
    };
    let flush = false;
    const flushAt = (ms: number) => {
      if (ms <= 0) flush = true;
      else after(ms, () => setHurt((value) => value + 1));
    };
    const elementOf = (id: string) => stage.element?.querySelector(`[data-object-id="${CSS.escape(id)}"]`) ?? null;
    // the spark color of whoever controls a creature (now, or before it died in this update)
    const sparkOf = (id: string) => {
      const controller = (now.permanents.get(id) ?? before.permanents.get(id))?.controllerId;
      return controller ? sparksRef.current?.get(controller) ?? null : null;
    };

    // where a card or player is now, or where a card stood before it left (a creature killed by this damage)
    const pointOf = (id: string) => {
      const at = center(id) ?? boardSpots.current.get(id);
      if (at) return at;
      const placement = lastPlacement(now.permanents.get(id)?.cardId ?? before.permanents.get(id)?.cardId ?? id) ?? lastPlacement(id);
      return placement ? { x: placement.x + placement.width / 2, y: placement.y + placement.height / 2 } : null;
    };
    // signature effects of the spells and abilities that resolved
    const casts = cardEffects === 'signature'
      ? resolvedCasts(before.stackCards, now.stack, view.events, new Set(before.permanents.keys())).slice(0, MAX_SIGNATURES)
      : [];
    const signed = new Set(casts.map((cast) => cast.id));
    const spawnedSignatures: SignatureEffect[] = [];
    let quake = 0;
    for (const cast of casts) {
      const from = stackSpots.current.get(cast.id) ?? matchLayout(stage).stackPoint;
      const caster = cast.controllerId ? pointOf(cast.controllerId) : null;
      const hit = new Set(cast.targets);
      // what it damaged counts as hit, targeted or not (a Lava Spike's player, a Fiery Confluence's creatures)
      for (const event of view.events ?? []) {
        if (event.kind === 'DAMAGE' && event.targetId && (event.sourceId === cast.id || event.sourceId === before.stackCards.get(cast.id)?.cardId)) hit.add(event.targetId);
      }
      const targets = [...hit].map((id) => (before.stack.has(id) ? stackSpots.current.get(id) ?? null : pointOf(id))).filter((point): point is Point => !!point);
      const spills = [...cast.milled].map(([player, count]) => ({ at: pointOf(player), count })).filter((spill): spill is { at: Point; count: number } => !!spill.at);
      spawnedSignatures.push({ key: nextKey.current++, fx: cast.signature.fx, ink: cast.signature.ink, big: !!cast.signature.big, from, caster, targets, spills, width: stage.width, height: stage.height });
      if (cast.signature.big) quake = Math.max(quake, 6);
    }
    const plainSpells = cardEffects !== 'off';
    // when a resolving spell's effect reaches each card or player it hit: its number and shake wait for it
    const spellHitAt = new Map<string, number>();
    for (const cast of casts) {
      const impact = signatureImpact(cast.signature.fx);
      const reached = new Set(cast.targets);
      for (const event of view.events ?? []) {
        if (event.kind === 'DAMAGE' && event.targetId && (event.sourceId === cast.id || event.sourceId === before.stackCards.get(cast.id)?.cardId)) reached.add(event.targetId);
      }
      for (const id of reached) spellHitAt.set(id, Math.max(spellHitAt.get(id) ?? 0, impact));
    }

    // --- combat: strike by strike ------------------------------------------------------------------------------
    const combatNow = combatSide(now.combat);
    const combatBefore = combatSide(before.combat);
    // this view's combat over the last one's: creatures that died in this damage step are still in combat here
    const combatEither = combatSide([...before.combat, ...now.combat]);
    const lookup = (id: string): CardView | PermanentView | undefined => now.permanents.get(id) ?? before.permanents.get(id) ?? combatEither.cards.get(id);
    const strikes: Strike[] = view.events
      ? strikesFromEvents(view.events, combatEither, lookup)
      : strikesFromCombat(combatNow, now.step, before.step, lookup, (attacker) => stableDefender(attacker, combatNow, combatBefore, now.players, new Set(now.permanents.keys())));
    // a creature that died in this combat damage stays in its place, swings and takes its hit, then goes to its
    // graveyard: its flight there is held until then (when it flies: a graveyard pile shows its top card)
    const held = new Map<string, { element: Element; release: (ms: number) => void }>();
    const involved = new Set(strikes.flatMap((item) => [item.from, item.to]));
    for (const [id, permanent] of before.permanents) {
      if (now.permanents.has(id) || !involved.has(id)) continue;
      const hold = holdFlight(cardKey(permanent));
      if (hold) held.set(id, hold);
    }
    const bodyOf = (id: string) => elementOf(id) ?? held.get(id)?.element ?? null;
    // when each held card is done: its swing over and its last hit landed
    const doneAt = new Map<string, number>();
    const done = (id: string, ms: number) => doneAt.set(id, Math.max(doneAt.get(id) ?? 0, ms));
    // when each creature, player or planeswalker was hit last: deaths, lifelink and life totals wait for it
    const hitAt = new Map<string, number>();
    const killedBy = new Map<string, Strike>();
    const healDelay = new Map<string, number>();
    let biggest: { weight: HitWeight; at: number } | null = null;
    // phase 1 (regular damage after first strike, in the same update) waits for phase 0 to finish
    let phaseStart = 0;
    for (const phase of [0, 1]) {
      const round = strikes.filter((item) => item.phase === phase);
      if (round.length === 0) continue;
      let phaseEnd = phaseStart;
      // an attacker's swing at each blocker it hit, in turn; its blockers swing back at its first swing
      const swingsOf = new Map<string, number>();
      const firstSwing = new Map<string, number>();
      // trample: the swing that went through a blocker lands on the defender just after
      const contactOn = new Map<string, number>();
      const ordered = [...round].sort((a, b) => (a.role === b.role ? 0 : a.role === 'attack' ? -1 : 1));
      // a swing is as heavy as all the damage its creature deals in this step: a 9/9 hitting two small blockers and
      // trampling over still swings like a 9/9
      const dealt = new Map<string, number>();
      for (const item of round) dealt.set(item.from, (dealt.get(item.from) ?? 0) + item.amount);
      const swingWeight = (item: Strike) => hitWeight(dealt.get(item.from) ?? item.amount);
      for (const item of ordered) {
        const from = pointOf(item.from);
        const to = pointOf(item.to);
        const slot = phaseStart + Math.min(item.order, MAX_STAGGER) * STAGGER;
        let contact: number;
        if (item.through) {
          contact = (contactOn.get(`${item.from}>${item.through}`) ?? slot + 300) + 140;
          const via = pointOf(item.through);
          if (fancy && via && to) addAt(contact - 140, { kind: 'carry', ...via, to });
        } else if (item.role === 'attack') {
          const sweep = swingsOf.get(item.from) ?? 0;
          swingsOf.set(item.from, sweep + 1);
          const delay = slot + sweep * SWEEP;
          if (!firstSwing.has(item.from)) firstSwing.set(item.from, delay);
          const timing = from && to ? swing(bodyOf(item.from), from, to, { weight: swingWeight(item), fancy, delay, scale: stage.scale, target: bodyOf(item.to), role: 'attack' }) : { contact: delay + 300, end: delay + 520 };
          contact = timing.contact;
          contactOn.set(`${item.from}>${item.to}`, contact);
          phaseEnd = Math.max(phaseEnd, timing.end);
          done(item.from, timing.end);
        } else {
          const delay = firstSwing.get(item.to) ?? slot;
          const timing = from && to ? swing(bodyOf(item.from), from, to, { weight: swingWeight(item), fancy, delay, scale: stage.scale, target: bodyOf(item.to), role: 'block' }) : { contact: delay + 300, end: delay + 520 };
          contact = timing.contact;
          phaseEnd = Math.max(phaseEnd, timing.end);
          done(item.from, timing.end);
        }
        hitAt.set(item.to, Math.max(hitAt.get(item.to) ?? 0, contact));
        done(item.to, contact + 260);
        const heft = swingWeight(item);
        if (!biggest || WEIGHT_RANK[heft] > WEIGHT_RANK[biggest.weight]) biggest = { weight: heft, at: contact };
        if (!to) continue;
        const isPlayer = now.players.has(item.to);
        // the hit: splash and number, the target knocked back (or shaken), sparks the striker's player earned
        // without events the number comes from the life and damage the views show, not from the guessed strike
        if (!item.estimated) addAt(contact, { kind: 'damage', ...to, value: item.amount, tone: item.infect ? 'infect' : undefined });
        if (fancy && from) recoil(bodyOf(item.to), from, to, heft, contact);
        else shake(bodyOf(item.to), isPlayer ? 10 : 8, contact);
        if (item.to === myPlayerId) flushAt(contact);
        const color = sparkOf(item.from);
        if (color) addAt(contact, { kind: 'spark', ...to, color });
        if (!fancy) continue;
        if (heft === 'heavy' || heft === 'huge') addAt(contact, { kind: 'dust', ...to, size: heft === 'huge' ? 1.5 : 1 });
        if (heft === 'huge') addAt(contact, { kind: 'crack', ...to, angle: from ? angleOf(from, to) : 0 });
        if ((item.firstStrike || item.doubleStrike) && from) addAt(contact - 60, { kind: 'slash', ...to, angle: angleOf(from, to), tone: item.doubleStrike && phase === 1 ? 'gold' : undefined });
        if (item.deathtouch && !isPlayer) {
          addAt(contact, { kind: 'void', ...to });
          killedBy.set(item.to, item);
        }
        if (item.lifelink && item.controllerId) {
          const plate = pointOf(item.controllerId);
          if (plate) {
            addAt(contact + 80, { kind: 'motes', ...to, to: plate });
            healDelay.set(item.controllerId, Math.max(healDelay.get(item.controllerId) ?? 0, contact + 80 + 520));
          }
        }
        const target = lookup(item.to);
        const fighter = target && !isPlayer ? fighterOf(target, item.to) : null;
        if (fighter?.indestructible && item.amount >= fighter.toughness - fighter.marked) addAt(contact, { kind: 'shield', ...to });
      }
      phaseStart = phaseEnd + PHASE_GAP;
    }
    for (const [id, hold] of held) hold.release((doneAt.get(id) ?? 0) + 120);
    if (fancy && biggest && TABLE_SHAKE[biggest.weight] > 0) {
      const { weight, at } = biggest;
      after(at, () => shake(stage.element, TABLE_SHAKE[weight]));
    }

    if (view.events) {
      const visuals = eventVisuals(view.events);
      // a bolt flies first: what it hits takes the number and the shake when it arrives
      for (const bolt of visuals.bolts) {
        if (!plainSpells || signed.has(bolt.from) || [...signed].some((id) => before.stackCards.get(id)?.cardId === bolt.from)) continue;
        const from = pointOf(bolt.from);
        const to = pointOf(bolt.to);
        if (!from || !to) continue;
        add({ kind: 'bolt', ...from, to });
        spellHitAt.set(bolt.to, Math.max(spellHitAt.get(bolt.to) ?? 0, BOLT_MS));
      }
      for (const [id, amount] of visuals.noncombat) {
        const at = pointOf(id);
        const moment = spellHitAt.get(id) ?? 0;
        if (at) addAt(moment, { kind: 'damage', ...at, value: amount });
        shake(elementOf(id), now.life.has(id) ? 10 : 8, moment);
        if (id === myPlayerId) flushAt(moment);
        // a creature killed by it dissolves as it lands
        if (moment > 0) hitAt.set(id, Math.max(hitAt.get(id) ?? 0, moment));
      }
      for (const [id, amount] of visuals.drained) {
        const at = pointOf(id);
        if (at) add({ kind: 'damage', ...at, value: amount });
        if (id === myPlayerId) flush = true;
      }
      for (const [id, amount] of visuals.healed) {
        const at = pointOf(id);
        // lifelink: the number lands with the motes
        if (at) addAt(healDelay.get(id) ?? 0, { kind: 'heal', ...at, value: amount });
      }
      if (fancy) {
        // counters popping onto cards: +1/+1 in sage, −1/−1 in oxblood, loyalty punched in brass
        for (const event of view.events) {
          if ((event.kind !== 'COUNTERS_ADDED' && event.kind !== 'COUNTERS_REMOVED') || !event.targetId || !event.counter) continue;
          if (!now.permanents.has(event.targetId)) continue;
          const at = pointOf(event.targetId);
          if (!at) continue;
          const added = event.kind === 'COUNTERS_ADDED';
          const amount = event.amount ?? 1;
          const counter = event.counter.toLowerCase();
          if (counter === 'loyalty') {
            add({ kind: 'counter', ...at, label: `${added ? '+' : '−'}${amount}`, tone: 'gold' });
          } else if (counter === '+1/+1' || counter === '-1/-1') {
            const grows = (counter === '+1/+1') === added;
            add({ kind: 'counter', ...at, label: `${added ? '' : '−'}${amount > 1 ? `${amount}× ` : ''}${counter.replace('-', '−')}`, tone: grows ? 'up' : 'down' });
          }
        }
      }
    } else {
      // damage marked on permanents (struck ones are knocked back or shaken by the strike)
      const struck = new Set(strikes.map((item) => item.to));
      for (const [id, permanent] of now.permanents) {
        const was = before.permanents.get(id);
        if (!was) continue;
        const dealt = (permanent.damage ?? 0) - (was.damage ?? 0);
        if (dealt > 0) {
          const at = center(id);
          if (at) add({ kind: 'damage', ...at, value: dealt });
          if (!struck.has(id)) shake(elementOf(id));
        }
      }
      // life changes
      for (const [id, life] of now.life) {
        const delta = life - (before.life.get(id) ?? life);
        if (delta === 0) continue;
        const at = center(id);
        if (at) add({ kind: delta < 0 ? 'damage' : 'heal', ...at, value: Math.abs(delta) });
        if (delta < 0) {
          if (!struck.has(id)) shake(elementOf(id), 10);
          if (id === myPlayerId) flush = true;
        }
      }
    }

    // a player brought to 0 or less: the last hit lands with a bigger moment
    if (fancy) {
      for (const [id, life] of now.life) {
        const was = before.life.get(id);
        if (was === undefined || was <= 0 || life > 0) continue;
        const at = pointOf(id);
        const moment = hitAt.get(id) ?? 0;
        if (at) addAt(moment, { kind: 'lethal', ...at });
        after(moment, () => shake(stage.element, 12));
      }
    }

    // cards landing on the battlefield: the ring goes out as each one's flight from the hand or stack puts it down
    // (flip.ts has started the flights by now), measured there
    const landed = [...now.permanents.keys()].filter((id) => !before.permanents.has(id));
    const landingOf = new Map<number, string[]>();
    let landAt = 0;
    for (const id of landed) {
      const ms = Math.round(landingIn(cardKey(now.permanents.get(id)!)));
      landAt = Math.max(landAt, ms);
      landingOf.set(ms, [...(landingOf.get(ms) ?? []), id]);
    }
    for (const [ms, group] of landingOf) {
      after(ms, () => {
        const rings: Effect[] = [];
        let heavy = false;
        for (const id of group) {
          const at = center(id);
          const element = elementOf(id);
          const box = element?.getBoundingClientRect();
          const permanent = now.permanents.get(id)!;
          const size = box ? box.width / stage.scale : 120;
          if (!at) continue;
          const creature = (permanent.cardTypes ?? []).includes('CREATURE');
          if (fancy && permanent.isToken) {
            rings.push({ kind: 'puff', ...at, size, key: nextKey.current++ });
          } else if (fancy && creature && (toInt(permanent.power) >= 6 || (permanent.manaValue ?? 0) >= 6)) {
            heavy = true;
            thump(element, 0);
            rings.push({ kind: 'enter', ...at, size, tone: 'big', key: nextKey.current++ });
            rings.push({ kind: 'dust', ...at, size: 1.3, key: nextKey.current++ });
          } else {
            rings.push({ kind: 'enter', ...at, size, key: nextKey.current++ });
          }
        }
        if (rings.length > 0) show(rings);
        if (heavy) after(170, () => shake(stage.element, 5));
      });
    }

    // creatures and other permanents leaving: ink dissolving where they stood, once the hit that killed them lands
    const zoneOf = new Map<string, string | undefined>();
    for (const event of view.events ?? []) if (event.kind === 'ZONE' && event.fromZone === 'BATTLEFIELD' && event.targetId) zoneOf.set(event.targetId, event.toZone);
    const damagedNow = new Set<string>([...strikes.map((item) => item.to), ...(view.events ?? []).filter((event) => event.kind === 'DAMAGE').map((event) => event.targetId!)]);
    for (const [id, permanent] of before.permanents) {
      if (now.permanents.has(id)) continue;
      const placement = lastPlacement(permanent.cardId ?? id);
      const stood = boardSpots.current.get(id);
      if (!placement || !(permanent.cardTypes ?? []).some((type) => type === 'CREATURE' || type === 'PLANESWALKER' || type === 'BATTLE')) continue;
      const spot = { x: stood?.x ?? placement.x + placement.width / 2, y: stood?.y ?? placement.y + placement.height / 2, size: placement.width };
      const moment = hitAt.get(id) ? hitAt.get(id)! + 90 : 0;
      const toZone = zoneOf.get(id) ?? zoneOf.get(permanent.cardId ?? '');
      if (toZone === 'HAND' || toZone === 'LIBRARY') continue;
      if (fancy && toZone === 'EXILED') addAt(moment, { kind: 'exile', ...spot });
      else if (fancy && killedBy.has(id)) addAt(moment, { kind: 'death', ...spot, tone: 'void' });
      else if (fancy && damagedNow.has(id)) addAt(moment, { kind: 'shatter', ...spot });
      else if ((permanent.cardTypes ?? []).includes('CREATURE')) addAt(moment, { kind: 'death', ...spot });
    }

    // the sounds of this view wait for its first hit and its landings (beats.ts)
    const hits = [...hitAt.values()].filter((ms) => ms > 0);
    noteBeats(view, { impact: hits.length > 0 ? Math.min(...hits) : 0, land: landAt });

    // spells and abilities resolving
    const departed = [...before.stack].filter((id) => !now.stack.has(id) && !signed.has(id));
    if (plainSpells && departed.length > 0) add({ kind: 'resolve', ...matchLayout(stage).stackPoint });
    // a planeswalker's ability resolving: a beam from the walker to what it targeted
    if (fancy) {
      for (const id of departed) {
        const ability = before.stackCards.get(id);
        if (!ability?.mageObjectType?.startsWith('ABILITY') || !(ability.targets ?? []).length) continue;
        const walker = [...before.permanents.values()].find((permanent) => permanent.name === ability.name && permanent.controllerId === ability.controllerId && (permanent.cardTypes ?? []).includes('PLANESWALKER'));
        const from = walker ? pointOf(walker.id!) : null;
        if (!from) continue;
        for (const target of ability.targets ?? []) {
          const to = before.stack.has(target) ? stackSpots.current.get(target) ?? null : pointOf(target);
          if (to) add({ kind: 'beam', ...from, to });
        }
      }
    }

    // new attackers charge. No lean toward the target here: declaring isn't striking, and a lunge on declaring read
    // as an attack on its own; the swing comes with the damage.
    for (const id of now.attackers) {
      if (before.attackers.has(id)) continue;
      const at = center(id);
      if (at) add({ kind: 'charge', ...at });
    }

    if (fancy) {
      for (const [id, permanent] of now.permanents) {
        const was = before.permanents.get(id);
        if (!was) continue;
        // power or toughness changing (counters, pump spells, shrinking): the card pulses green or red
        const grew = toInt(permanent.power) + toInt(permanent.toughness) - toInt(was.power) - toInt(was.toughness);
        if ((permanent.cardTypes ?? []).includes('CREATURE') && grew !== 0 && was.power !== undefined) {
          const box = elementOf(id)?.getBoundingClientRect();
          const at = center(id);
          if (at && box) add({ kind: 'pt', ...at, size: box.width / stage.scale, tone: grew > 0 ? 'up' : 'down' });
        }
        // an aura or equipment attaching to a new host: a click where it lands
        if (permanent.attachedTo && permanent.attachedTo !== was.attachedTo && permanent.attachedToPermanent) {
          const host = permanent.attachedTo;
          after(380, () => {
            const at = center(host);
            if (at) show([{ kind: 'attach', ...at, key: nextKey.current++ }]);
          });
        }
      }
    }

    // effects render on the next frame, after this view's layout has painted. Not cancelled when the next view
    // arrives in the same frame: that view diffs against this one, so these effects would otherwise be lost.
    requestAnimationFrame(() => {
      if (spawned.length > 0) show(spawned);
      if (flush) setHurt((value) => value + 1);
      if (spawnedSignatures.length > 0) {
        setSignatures((current) => [...current, ...spawnedSignatures]);
        for (const effect of spawnedSignatures) {
          after(lifetimeOf(effect), () => setSignatures((current) => current.filter((item) => item.key !== effect.key)));
        }
      }
      if (quake > 0) shake(stage.element, quake);
      measureStack();
    });
  }, [view, center, stage, myPlayerId, cardEffects, fancy]);

  return (
    <div className={styles.layer} aria-hidden="true">
      {effects.map((effect) => <EffectView key={effect.key} effect={effect} />)}
      <SignatureVfx effects={signatures} />
      {hurt > 0 && <div key={hurt} className={styles.hurt} />}
      <TriggerSignals view={view} />
    </div>
  );
}

const DROPLETS = Array.from({ length: 12 }, (_, i) => ({
  angle: (i / 12) * Math.PI * 2 + (i % 2) * 0.3,
  distance: 50 + (i * 37) % 60,
  size: 8 + (i * 13) % 12,
}));

/** A hit spark: a few small bits thrown a short way, unevenly. */
const SPARKS = Array.from({ length: 9 }, (_, i) => {
  const angle = (i / 9) * Math.PI * 2 + (i % 3) * 0.4;
  const distance = 34 + (i * 23) % 30;
  return { dx: Math.cos(angle) * distance, dy: Math.sin(angle) * distance };
});

/** Dust thrown low and wide by a heavy hit or a heavy landing. */
const DUST = Array.from({ length: 14 }, (_, i) => {
  const angle = (i / 14) * Math.PI * 2 + (i % 2) * 0.2;
  const distance = 70 + (i * 29) % 50;
  return { dx: Math.cos(angle) * distance, dy: Math.sin(angle) * distance * 0.55, size: 14 + (i * 7) % 14 };
});

/** The cracks of a huge hit, as rays from where it landed. */
const CRACKS = Array.from({ length: 7 }, (_, i) => ({ angle: -60 + i * 20 + ((i * 13) % 9), length: 90 + (i * 41) % 80 }));

/** Shards a creature breaks into when damage kills it. */
const SHARDS = Array.from({ length: 10 }, (_, i) => {
  const angle = (i / 10) * Math.PI * 2 + 0.3;
  const distance = 60 + (i * 31) % 50;
  return { dx: Math.cos(angle) * distance, dy: Math.sin(angle) * distance + 40, spin: ((i * 47) % 160) - 80 };
});

/** Motes of life: a short stream from a hit to the lifelinker's controller. */
const MOTES = Array.from({ length: 7 }, (_, i) => ({ delay: i * 45, wobble: ((i * 17) % 21) - 10 }));

const vars = (values: Record<string, string | number>): CSSProperties => values as CSSProperties;

function EffectView({ effect }: { effect: Effect }) {
  const position: CSSProperties = { left: effect.x, top: effect.y };
  switch (effect.kind) {
    case 'damage':
    case 'heal':
      return (
        <div className={[styles.hit, effect.kind === 'heal' ? styles.heal : styles.damage, effect.tone === 'infect' ? styles.infect : ''].join(' ')} style={position}>
          {effect.kind === 'damage' && DROPLETS.map((drop, index) => (
            <span
              key={index}
              className={styles.drop}
              style={vars({ width: drop.size, height: drop.size, '--dx': `${Math.cos(drop.angle) * drop.distance}px`, '--dy': `${Math.sin(drop.angle) * drop.distance}px` })}
            />
          ))}
          <span className={styles.ring} />
          <span className={styles.number}>{effect.kind === 'heal' ? '+' : '−'}{effect.value}</span>
        </div>
      );
    case 'enter':
      return <span className={[styles.shockwave, effect.tone === 'big' ? styles.bigLanding : ''].join(' ')} style={{ ...position, width: (effect.size ?? 120) * 1.6, height: (effect.size ?? 120) * 1.6 }} />;
    case 'resolve':
      return (
        <div className={styles.burst} style={position}>
          {DROPLETS.map((drop, index) => (
            <span key={index} className={styles.spark} style={vars({ '--dx': `${Math.cos(drop.angle) * drop.distance * 1.4}px`, '--dy': `${Math.sin(drop.angle) * drop.distance * 1.4}px` })} />
          ))}
          <span className={styles.ring} />
        </div>
      );
    case 'death':
      return <span className={[styles.dissolve, effect.tone === 'void' ? styles.voidDeath : ''].join(' ')} style={{ ...position, width: (effect.size ?? 120) * 1.3, height: (effect.size ?? 120) * 1.3 * 1.4 }} />;
    case 'shatter':
      return (
        <div className={styles.point} style={position}>
          <span className={styles.dissolve} style={{ width: (effect.size ?? 120) * 1.2, height: (effect.size ?? 120) * 1.2 * 1.4 }} />
          {SHARDS.map((shard, index) => (
            <span key={index} className={styles.shard} style={vars({ '--dx': `${shard.dx}px`, '--dy': `${shard.dy}px`, '--spin': `${shard.spin}deg` })} />
          ))}
        </div>
      );
    case 'exile':
      return <span className={styles.exile} style={{ ...position, width: (effect.size ?? 120) * 1.1, height: (effect.size ?? 120) * 1.1 * 1.4 }} />;
    case 'charge':
      return <span className={styles.charge} style={position} />;
    case 'bolt':
      return <Bolt from={effect} to={effect.to ?? effect} />;
    case 'spark':
      return (
        <div className={styles.sparks} style={vars({ ...position, '--spark': effect.color ?? '' })}>
          {SPARKS.map((spark, index) => (
            <span key={index} className={styles.sparkBit} style={vars({ '--dx': `${spark.dx}px`, '--dy': `${spark.dy}px` })} />
          ))}
        </div>
      );
    case 'dust':
      return (
        <div className={styles.point} style={position}>
          <span className={styles.dustRing} style={{ scale: `${effect.size ?? 1}` }} />
          {DUST.map((mote, index) => (
            <span key={index} className={styles.dust} style={vars({ width: mote.size, height: mote.size, '--dx': `${mote.dx * (effect.size ?? 1)}px`, '--dy': `${mote.dy * (effect.size ?? 1)}px` })} />
          ))}
        </div>
      );
    case 'crack':
      return (
        <div className={styles.point} style={vars({ ...position, rotate: `${effect.angle ?? 0}deg` })}>
          {CRACKS.map((crack, index) => <span key={index} className={styles.crack} style={vars({ rotate: `${crack.angle}deg`, '--length': `${crack.length}px` })} />)}
          <span className={styles.crackWave} />
        </div>
      );
    case 'slash':
      return <span className={[styles.slash, effect.tone === 'gold' ? styles.slashSecond : ''].join(' ')} style={vars({ ...position, rotate: `${(effect.angle ?? 0) + 90}deg` })} />;
    case 'void':
      return <span className={styles.voidMark} style={position} />;
    case 'shield':
      return <span className={styles.shield} style={position} />;
    case 'lethal':
      return (
        <div className={styles.point} style={position}>
          <span className={styles.lethal} />
          <span className={[styles.lethal, styles.lethalLate].join(' ')} />
        </div>
      );
    case 'motes':
      return <Stream from={effect} to={effect.to ?? effect} className={styles.mote} count={MOTES.length} travel={520} />;
    case 'carry':
      return <Streak from={effect} to={effect.to ?? effect} className={styles.carry} />;
    case 'beam':
      return <Streak from={effect} to={effect.to ?? effect} className={styles.beam} />;
    case 'puff':
      return (
        <div className={styles.point} style={position}>
          {DUST.slice(0, 10).map((mote, index) => (
            <span key={index} className={styles.puff} style={vars({ width: mote.size * 2, height: mote.size * 2, '--dx': `${mote.dx * 0.6}px`, '--dy': `${mote.dy * 0.9}px` })} />
          ))}
        </div>
      );
    case 'counter':
      return <span className={[styles.counterPop, effect.tone ? styles[`counter_${effect.tone}`] : ''].join(' ')} style={position}>{effect.label}</span>;
    case 'pt':
      return <span className={[styles.pt, effect.tone === 'down' ? styles.ptDown : styles.ptUp].join(' ')} style={{ ...position, width: (effect.size ?? 120) * 1.05, height: (effect.size ?? 120) * 1.05 * 1.4 }} />;
    case 'attach':
      return <span className={styles.attach} style={position} />;
  }
}

/** Noncombat damage flying from its source to what it hits. */
function Bolt({ from, to }: { from: { x: number; y: number }; to: { x: number; y: number } }) {
  const ref = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    ref.current?.animate(
      [{ translate: '0 0', scale: '0.6' }, { translate: `${to.x - from.x}px ${to.y - from.y}px`, scale: '1' }],
      { duration: LIFETIME.bolt, easing: 'cubic-bezier(0.5, 0, 0.9, 0.6)', fill: 'forwards' },
    );
  }, [from.x, from.y, to.x, to.y]);
  return <span ref={ref} className={styles.bolt} style={{ left: from.x, top: from.y }} />;
}

/** A line drawn from one point to another and fading: trample carrying through, a planeswalker's beam. */
function Streak({ from, to, className }: { from: Point; to: Point; className: string }) {
  const length = Math.hypot(to.x - from.x, to.y - from.y);
  return <span className={className} style={vars({ left: from.x, top: from.y, width: length, rotate: `${angleOf(from, to)}deg` })} />;
}

/** Motes flowing from one point to another, one after another. */
function Stream({ from, to, className, count, travel }: { from: Point; to: Point; className: string; count: number; travel: number }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const motes = ref.current?.children;
    if (!motes) return;
    const dx = to.x - from.x;
    const dy = to.y - from.y;
    // bowed a little to the side, so the stream reads as flowing rather than flying straight
    const bow = { x: -dy * 0.12, y: dx * 0.12 };
    Array.from(motes).forEach((mote, index) => {
      const wobble = MOTES[index % MOTES.length].wobble;
      mote.animate(
        [
          { translate: '0 0', scale: '0.4', opacity: 0 },
          { translate: `${dx * 0.5 + bow.x + wobble}px ${dy * 0.5 + bow.y}px`, scale: '1', opacity: 1, offset: 0.5 },
          { translate: `${dx}px ${dy}px`, scale: '0.6', opacity: 0 },
        ],
        { duration: travel, delay: MOTES[index % MOTES.length].delay, easing: 'cubic-bezier(0.4, 0, 0.6, 1)', fill: 'both' },
      );
    });
  }, [from.x, from.y, to.x, to.y, travel]);
  return (
    <div ref={ref} className={styles.point} style={{ left: from.x, top: from.y }}>
      {Array.from({ length: count }, (_, index) => <span key={index} className={className} />)}
    </div>
  );
}
