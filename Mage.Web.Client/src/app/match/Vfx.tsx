import { useEffect, useRef, useState, type CSSProperties } from 'react';
import type { GameView, PermanentView } from '../../protocol/generated/views';
import { lastPlacement, motionAllowed } from './flip';
import { eventVisuals } from './vfxEvents';
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
 */

type EffectKind = 'damage' | 'heal' | 'enter' | 'resolve' | 'death' | 'charge' | 'bolt' | 'spark';

interface Effect {
  key: number;
  kind: EffectKind;
  x: number;
  y: number;
  value?: number;
  /** shockwaves and dissolves follow the card's size */
  size?: number;
  /** where a bolt lands */
  to?: { x: number; y: number };
  /** a hit spark's color */
  color?: string;
}

interface Snapshot {
  permanents: Map<string, PermanentView>;
  life: Map<string, number>;
  stack: Set<string>;
  attackers: Set<string>;
  step: string | undefined;
}

const LIFETIME: Record<EffectKind, number> = { damage: 1300, heal: 1300, enter: 700, resolve: 800, death: 900, charge: 600, bolt: 480, spark: 760 };

function snapshot(view: GameView): Snapshot {
  const permanents = new Map<string, PermanentView>();
  const life = new Map<string, number>();
  for (const player of view.players ?? []) {
    life.set(player.playerId!, player.life ?? 0);
    for (const permanent of Object.values(player.battlefield ?? {})) permanents.set(permanent.id!, permanent);
  }
  const attackers = new Set<string>();
  for (const group of view.combat ?? []) Object.keys(group.attackers ?? {}).forEach((id) => attackers.add(id));
  return { permanents, life, stack: new Set(Object.keys(view.stack ?? {})), attackers, step: view.step };
}

/** Shake an element without disturbing its own transform (tapped, attacking). */
function shake(element: Element | null, strength = 8) {
  element?.animate(
    [0, -1, 1, -0.8, 0.6, -0.3, 0].map((factor) => ({ translate: `${factor * strength}px ${factor * strength * -0.3}px` })),
    { duration: 420, easing: 'ease-out' },
  );
}

/** Push a card toward a point and back: an attacker striking its target. */
function lunge(element: Element | null, from: { x: number; y: number }, to: { x: number; y: number }, scale: number) {
  if (!element) return;
  const dx = (to.x - from.x) * 0.45 * scale;
  const dy = (to.y - from.y) * 0.45 * scale;
  element.animate(
    [{ translate: '0 0' }, { translate: `${-dx * 0.08}px ${-dy * 0.08}px`, offset: 0.25 }, { translate: `${dx}px ${dy}px`, offset: 0.55 }, { translate: '0 0' }],
    { duration: 520, easing: 'cubic-bezier(0.3, 0, 0.2, 1)' },
  );
}

export function Vfx({ view, myPlayerId, sparks }: {
  view: GameView | null;
  myPlayerId: string | null;
  /** players who earned hit sparks: player id to the spark's color */
  sparks?: ReadonlyMap<string, string>;
}) {
  const [effects, setEffects] = useState<Effect[]>([]);
  // read when a view arrives: a look arriving later doesn't replay the last view's effects
  const sparksRef = useRef(sparks);
  useEffect(() => {
    sparksRef.current = sparks;
  }, [sparks]);
  const [hurt, setHurt] = useState(0);
  const previous = useRef<Snapshot | null>(null);
  const nextKey = useRef(0);
  const center = useObjectCenter();
  const stage = useStage();

  useEffect(() => {
    if (!view) return;
    const now = snapshot(view);
    const before = previous.current;
    previous.current = now;
    if (!before || !motionAllowed()) return;

    const spawned: Effect[] = [];
    let flush = false;
    const add = (effect: Omit<Effect, 'key'>) => spawned.push({ ...effect, key: nextKey.current++ });
    const show = (list: Effect[]) => {
      setEffects((current) => [...current, ...list]);
      for (const effect of list) {
        setTimeout(() => setEffects((current) => current.filter((item) => item.key !== effect.key)), LIFETIME[effect.kind]);
      }
    };
    const elementOf = (id: string) => stage.element?.querySelector(`[data-object-id="${CSS.escape(id)}"]`) ?? null;
    // the spark color of whoever controls a creature (now, or before it died in this update)
    const sparkOf = (id: string) => {
      const controller = (now.permanents.get(id) ?? before.permanents.get(id))?.controllerId;
      return controller ? sparksRef.current?.get(controller) ?? null : null;
    };

    // where a card or player is now, or where a card stood before it left (a creature killed by this damage)
    const pointOf = (id: string) => {
      const at = center(id);
      if (at) return at;
      const placement = lastPlacement(id);
      return placement ? { x: placement.x + placement.width / 2, y: placement.y + placement.height / 2 } : null;
    };
    if (view.events) {
      const visuals = eventVisuals(view.events);
      for (const [id, amount] of visuals.damage) {
        const at = pointOf(id);
        if (at) add({ kind: 'damage', ...at, value: amount });
        shake(elementOf(id), now.life.has(id) ? 10 : 8);
        if (id === myPlayerId) flush = true;
      }
      for (const [id, amount] of visuals.drained) {
        const at = pointOf(id);
        if (at) add({ kind: 'damage', ...at, value: amount });
        if (id === myPlayerId) flush = true;
      }
      for (const [id, amount] of visuals.healed) {
        const at = pointOf(id);
        if (at) add({ kind: 'heal', ...at, value: amount });
      }
      for (const bolt of visuals.bolts) {
        const from = pointOf(bolt.from);
        const to = pointOf(bolt.to);
        if (from && to) add({ kind: 'bolt', ...from, to });
      }
      for (const hit of visuals.hits) {
        const color = sparkOf(hit.from);
        const at = color ? pointOf(hit.to) : null;
        if (color && at) add({ kind: 'spark', ...at, color });
      }
    } else {
      // damage marked on permanents
      for (const [id, permanent] of now.permanents) {
        const was = before.permanents.get(id);
        if (!was) continue;
        const dealt = (permanent.damage ?? 0) - (was.damage ?? 0);
        if (dealt > 0) {
          const at = center(id);
          if (at) add({ kind: 'damage', ...at, value: dealt });
          shake(elementOf(id));
        }
      }
      // life changes
      for (const [id, life] of now.life) {
        const delta = life - (before.life.get(id) ?? life);
        if (delta === 0) continue;
        const at = center(id);
        if (at) add({ kind: delta < 0 ? 'damage' : 'heal', ...at, value: Math.abs(delta) });
        if (delta < 0) {
          shake(elementOf(id), 10);
          if (id === myPlayerId) flush = true;
        }
      }
    }
    // cards landing on the battlefield: measured once their flight from the hand or stack has settled
    const landed = [...now.permanents.keys()].filter((id) => !before.permanents.has(id));
    if (landed.length > 0) {
      setTimeout(() => {
        const rings: Effect[] = [];
        for (const id of landed) {
          const at = center(id);
          const box = elementOf(id)?.getBoundingClientRect();
          if (at) rings.push({ kind: 'enter', ...at, size: box ? box.width / stage.scale : 120, key: nextKey.current++ });
        }
        if (rings.length > 0) show(rings);
      }, 650);
    }
    // creatures and other permanents leaving: ink dissolving where they stood
    for (const [id, permanent] of before.permanents) {
      if (now.permanents.has(id)) continue;
      const placement = lastPlacement(permanent.cardId ?? id);
      if (placement && (permanent.cardTypes ?? []).includes('CREATURE')) {
        add({ kind: 'death', x: placement.x + placement.width / 2, y: placement.y + placement.height / 2, size: placement.width });
      }
    }
    // spells and abilities resolving
    if ([...before.stack].some((id) => !now.stack.has(id))) add({ kind: 'resolve', ...matchLayout(stage).stackPoint });
    // new attackers charge
    for (const id of now.attackers) {
      if (before.attackers.has(id)) continue;
      const at = center(id);
      if (at) add({ kind: 'charge', ...at });
    }
    // combat damage: every attacker strikes its blocker or the player it attacks
    const damageStep = (step: string | undefined) => step === 'COMBAT_DAMAGE' || step === 'FIRST_COMBAT_DAMAGE';
    if (damageStep(now.step) && now.step !== before.step) {
      for (const group of view.combat ?? []) {
        const target = Object.keys(group.blockers ?? {})[0] ?? group.defenderId;
        const to = target ? center(target) : null;
        for (const attacker of Object.keys(group.attackers ?? {})) {
          const from = center(attacker);
          if (from && to) lunge(elementOf(attacker), from, to, 1);
          // servers without events: the sparks land where the attacker strikes
          const color = view.events ? null : sparkOf(attacker);
          if (color && to) add({ kind: 'spark', ...to, color });
        }
      }
    }

    // effects render on the next frame, after this view's layout has painted. Not cancelled when the next view
    // arrives in the same frame: that view diffs against this one, so these effects would otherwise be lost.
    requestAnimationFrame(() => {
      if (spawned.length > 0) show(spawned);
      if (flush) setHurt((value) => value + 1);
    });
  }, [view, center, stage, myPlayerId]);

  return (
    <div className={styles.layer} aria-hidden="true">
      {effects.map((effect) => <EffectView key={effect.key} effect={effect} />)}
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

function EffectView({ effect }: { effect: Effect }) {
  const position: CSSProperties = { left: effect.x, top: effect.y };
  switch (effect.kind) {
    case 'damage':
    case 'heal':
      return (
        <div className={[styles.hit, effect.kind === 'heal' ? styles.heal : styles.damage].join(' ')} style={position}>
          {effect.kind === 'damage' && DROPLETS.map((drop, index) => (
            <span
              key={index}
              className={styles.drop}
              style={{
                width: drop.size,
                height: drop.size,
                ['--dx' as string]: `${Math.cos(drop.angle) * drop.distance}px`,
                ['--dy' as string]: `${Math.sin(drop.angle) * drop.distance}px`,
              }}
            />
          ))}
          <span className={styles.ring} />
          <span className={styles.number}>{effect.kind === 'heal' ? '+' : '−'}{effect.value}</span>
        </div>
      );
    case 'enter':
      return <span className={styles.shockwave} style={{ ...position, width: (effect.size ?? 120) * 1.6, height: (effect.size ?? 120) * 1.6 }} />;
    case 'resolve':
      return (
        <div className={styles.burst} style={position}>
          {DROPLETS.map((drop, index) => (
            <span
              key={index}
              className={styles.spark}
              style={{ ['--dx' as string]: `${Math.cos(drop.angle) * drop.distance * 1.4}px`, ['--dy' as string]: `${Math.sin(drop.angle) * drop.distance * 1.4}px` }}
            />
          ))}
          <span className={styles.ring} />
        </div>
      );
    case 'death':
      return <span className={styles.dissolve} style={{ ...position, width: (effect.size ?? 120) * 1.3, height: (effect.size ?? 120) * 1.3 * 1.4 }} />;
    case 'charge':
      return <span className={styles.charge} style={position} />;
    case 'bolt':
      return <Bolt from={effect} to={effect.to ?? effect} />;
    case 'spark':
      return (
        <div className={styles.sparks} style={{ ...position, ['--spark' as string]: effect.color }}>
          {SPARKS.map((spark, index) => (
            <span key={index} className={styles.sparkBit} style={{ ['--dx' as string]: `${spark.dx}px`, ['--dy' as string]: `${spark.dy}px` }} />
          ))}
        </div>
      );
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
