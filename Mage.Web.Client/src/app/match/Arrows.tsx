import { useEffect, useState, type CSSProperties } from 'react';
import { bundleAttacks, isTrunk, trunkJoin } from './attackArrows';
import { useMatchUi } from './matchUi';
import { toStagePoint, useObjectCenter, useStage } from './stageContext';
import styles from './Arrows.module.css';

interface Point {
  x: number;
  y: number;
}

function curve(from: Point, to: Point): string {
  const mx = (from.x + to.x) / 2;
  const my = Math.min(from.y, to.y) - Math.abs(to.x - from.x) * 0.18 - 40;
  return `M ${from.x} ${from.y} Q ${mx} ${my} ${to.x} ${to.y}`;
}

/** Midpoint of the quadratic Bezier curve from `from` to `to` (at t = 0.5), for placing numbered target badges. */
function curveMidpoint(from: Point, to: Point): Point {
  const mx = (from.x + to.x) / 2;
  const my = Math.min(from.y, to.y) - Math.abs(to.x - from.x) * 0.18 - 40;
  return {
    x: 0.25 * from.x + 0.5 * mx + 0.25 * to.x,
    y: 0.25 * from.y + 0.5 * my + 0.25 * to.y,
  };
}

/** An attack: leaves the attacker toward its target and stops short of it, bowing sideways so parallel attacks fan out. */
function attackPath(from: Point, to: Point, index: number, count: number): string {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const length = Math.hypot(dx, dy) || 1;
  const ux = dx / length;
  const uy = dy / length;
  const start = { x: from.x + ux * 46, y: from.y + uy * 46 };
  const end = { x: to.x - ux * 64, y: to.y - uy * 64 };
  const bow = (index - (count - 1) / 2) * 40 + length * 0.12;
  const control = { x: (start.x + end.x) / 2 - uy * bow, y: (start.y + end.y) / 2 + ux * bow };
  return `M ${start.x} ${start.y} Q ${control.x} ${control.y} ${end.x} ${end.y}`;
}

/** A feeder: from an attacking stack to where a trunk starts, bowing a little so neighbours don't overlap. */
function feederPath(from: Point, to: Point): string {
  const start = { x: from.x, y: from.y + (to.y < from.y ? -40 : 40) };
  const control = { x: (start.x + to.x) / 2, y: start.y + (to.y - start.y) * 0.15 };
  return `M ${start.x} ${start.y} Q ${control.x} ${control.y} ${to.x} ${to.y}`;
}

/** Attack arrows of one color and focus state: the usual oxblood (no color), a player's earned arrow color (B300), or per-defender colors in multi-defender combat. */
interface AttackLayer {
  color: string | null;
  dim: boolean;
  paths: string[];
  feeders: string[];
}

function layerFor(layers: AttackLayer[], color: string | null, dim: boolean): AttackLayer {
  let layer = layers.find((candidate) => candidate.color === color && candidate.dim === dim);
  if (!layer) {
    layer = { color, dim, paths: [], feeders: [] };
    layers.push(layer);
  }
  return layer;
}

/** An earned color re-inks the arrow's layers (Arrows.module.css reads these). */
function tintStyle(color: string | null): CSSProperties | undefined {
  return color ? ({ '--attack': color, '--attack-flow': `color-mix(in srgb, ${color} 45%, white)` } as CSSProperties) : undefined;
}

/**
 * Arrows on the table: attackers to what they will swing at (oxblood, or the attacking player's earned color): what
 * they attack, or their blockers once blocked; what the top of the stack targets (ink); and while choosing targets,
 * a live arrow from the spell to the pointer (amber).
 */
export function Arrows({ sourceId, targetIds, targetLabels, live, attacks, leads, tints, defenderColors }: {
  sourceId: string | null;
  targetIds: string[];
  /** optional short labels for multi-target spells/abilities (shown as numbered badges along each target arrow) */
  targetLabels?: readonly (string | null)[];
  live: boolean;
  /** attacker -> what it swings at: the player, planeswalker or battle it attacks, or each blocker blocking it */
  attacks: [string, string][];
  /** each permanent's stack lead: a stack of attacking tokens draws one arrow */
  leads: ReadonlyMap<string, string>;
  /** permanents whose controller earned an arrow color: their attacks are drawn in it */
  tints?: ReadonlyMap<string, string>;
  /** defender id -> distinct color when attacking multiple defenders at once */
  defenderColors?: ReadonlyMap<string, string>;
}) {
  const center = useObjectCenter();
  const stage = useStage();
  const [, setTick] = useState(0);
  const [pointer, setPointer] = useState<Point | null>(null);
  const dragging = useMatchUi((state) => state.dragging);
  const hoveredDefender = useMatchUi((state) => state.hoveredDefender);
  // a creature dragged toward an attacker to block it: a live line that snaps to the attacker under the pointer
  const blockDrag = useMatchUi((state) => state.blockDrag);

  const targetKey = targetIds.join(',') + '|' + attacks.map((link) => link.join('>')).join(',');
  // positions change as cards fly in; re-measure for a moment after changes
  useEffect(() => {
    let frames = 0;
    let handle = 0;
    const loop = () => {
      setTick((value) => value + 1);
      if (++frames < 40) handle = requestAnimationFrame(loop);
    };
    handle = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(handle);
    // a resized window moves everything: measure again
  }, [sourceId, targetKey, stage.scale, stage.width, stage.height]);

  // the backstop for a drag whose card never heard the release: any release, or leaving the window, ends the line
  const lineHeld = !!blockDrag;
  useEffect(() => {
    if (!lineHeld) return;
    const end = () => {
      // after the card's own pointerup, which reads the drop from the event, not from here
      window.setTimeout(() => useMatchUi.getState().setBlockDrag(null), 0);
    };
    window.addEventListener('pointerup', end);
    window.addEventListener('pointercancel', end);
    window.addEventListener('blur', end);
    return () => {
      window.removeEventListener('pointerup', end);
      window.removeEventListener('pointercancel', end);
      window.removeEventListener('blur', end);
    };
  }, [lineHeld]);

  const tracking = live || !!blockDrag;
  useEffect(() => {
    if (!tracking) {
      setPointer(null);
      return;
    }
    const onMove = (event: PointerEvent) => setPointer(toStagePoint(stage, event.clientX, event.clientY));
    window.addEventListener('pointermove', onMove);
    return () => window.removeEventListener('pointermove', onMove);
  }, [tracking, stage]);

  const from = sourceId ? center(sourceId) : null;
  const targets = from
    ? targetIds.map((id, index) => ({ point: center(id), index, label: targetLabels?.[index] ?? null }))
      .filter((entry): entry is { point: Point; index: number; label: string | null } => !!entry.point)
    : [];
  // one arrow per attacking stack; many stacks at one defender join into a trunk with a count
  const bundles = bundleAttacks(attacks, leads);
  const multiDefender = bundles.length > 1;
  const focusDefender = hoveredDefender && bundles.some((bundle) => bundle.defender === hoveredDefender) ? hoveredDefender : null;
  const single: { from: Point; to: Point; color: string | null; dim: boolean }[] = [];
  const layers: AttackLayer[] = [];
  const trunks: { at: Point; count: number; color: string | null; dim: boolean }[] = [];
  for (const bundle of bundles) {
    const to = center(bundle.defender);
    if (!to) continue;
    const bundleColor = multiDefender ? (defenderColors?.get(bundle.defender) ?? null) : null;
    const dim = focusDefender !== null && bundle.defender !== focusDefender;
    const sources = bundle.sources
      .map((source) => ({ at: center(source.id), color: bundleColor ?? tints?.get(source.id) ?? null }))
      .filter((source): source is { at: Point; color: string | null } => !!source.at);
    if (sources.length === 0) continue;
    if (isTrunk(bundle)) {
      const join = trunkJoin(sources.map((source) => source.at), to);
      // a trunk is one player's horde: it takes the defender's color in multi-defender combat, else its first stack's
      const layer = layerFor(layers, sources[0].color, dim);
      sources.forEach((source) => layer.feeders.push(feederPath(source.at, join)));
      layer.paths.push(attackPath(join, to, 0, 1));
      // the count sits where the feeders meet, clear of the defender's board
      trunks.push({ at: join, count: bundle.count, color: sources[0].color, dim });
    } else {
      sources.forEach((source) => single.push({ from: source.at, to, color: source.color, dim }));
    }
  }
  single.forEach((arrow, index) => layerFor(layers, arrow.color, arrow.dim).paths.push(attackPath(arrow.from, arrow.to, index, single.length)));
  const blockFrom = blockDrag ? center(blockDrag.blockerId) : null;
  const blockTo = blockDrag?.attackerId ? center(blockDrag.attackerId) : pointer;
  const blockLine = blockFrom && blockTo ? [blockFrom, blockTo] as const : null;
  const attackCount = layers.reduce((sum, layer) => sum + layer.paths.length, 0);
  if (dragging || (!blockLine && attackCount === 0 && (!from || (targets.length === 0 && !(live && pointer))))) return null;

  const marching = layers.some((layer) => layer.paths.length > 0) || (!!from && targets.length > 0);
  return (
    <>
    <svg className={styles.arrows} width={stage.width} height={stage.height} aria-hidden="true">
      <defs>
        <marker id="arrow-decision" viewBox="0 0 10 10" refX="7" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
          <path d="M0 0 L10 5 L0 10 z" fill="var(--decision)" />
        </marker>
        {/* a marker takes its colors where it is defined, so each arrow color has its own head */}
        {layers.map((layer, index) => (
          <marker key={index} id={`arrow-attack-${index}`} viewBox="0 0 10 10" refX="5" refY="5" markerWidth="4.2" markerHeight="4.2" orient="auto-start-reverse">
            <path d="M0 0 L10 5 L0 10 L2.5 5 z" fill={layer.color ?? 'var(--oxblood-lit)'} />
          </marker>
        ))}
      </defs>
      {/* each layer is filtered once as a whole: a filter per arrow, repainted under the marching dashes, stalls the frame when a horde attacks */}
      {layers.map((layer, layerIndex) => (
        <g key={layerIndex} className={layer.dim ? styles.dim : undefined} style={tintStyle(layer.color)}>
          <g className={styles.attackGlow}>
            {layer.feeders.map((d, index) => <path key={`f${index}`} className={[styles.attack, styles.feeder].join(' ')} d={d} />)}
            {layer.paths.map((d, index) => <path key={index} className={styles.attack} d={d} />)}
          </g>
          <g className={styles.attackBody}>
            {layer.feeders.map((d, index) => <path key={`f${index}`} className={[styles.attack, styles.feeder].join(' ')} d={d} />)}
            {layer.paths.map((d, index) => <path key={index} className={styles.attack} d={d} markerEnd={`url(#arrow-attack-${layerIndex})`} />)}
          </g>
        </g>
      ))}
      {trunks.map((trunk, index) => (
        <g
          key={index}
          className={[styles.attackCount, trunk.dim ? styles.dim : ''].join(' ')}
          style={tintStyle(trunk.color)}
          transform={`translate(${trunk.at.x} ${trunk.at.y})`}
        >
          <circle r={26} />
          <text dy="0.35em">×{trunk.count}</text>
        </g>
      ))}
      {blockLine && (
        <line
          className={blockDrag?.attackerId ? styles.blockLive : styles.live}
          x1={blockLine[0].x}
          y1={blockLine[0].y}
          x2={blockLine[1].x}
          y2={blockLine[1].y}
        />
      )}
      {from && live && pointer && <path className={styles.live} d={curve(from, pointer)} markerEnd="url(#arrow-decision)" />}
    </svg>
    {/*
      what marches, on a layer of its own with no filters: a dash moving repaints only this layer, never the glows and
      shadows under it (repainting those every frame cost a few ms of each 8 ms frame while arrows were up)
    */}
    {marching && (
      <svg className={[styles.arrows, styles.marching].join(' ')} width={stage.width} height={stage.height} aria-hidden="true">
        <defs>
          <marker id="arrow-ink" viewBox="0 0 10 10" refX="7" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
            <path d="M0 0 L10 5 L0 10 z" fill="var(--ink)" />
          </marker>
          <marker id="arrow-ink-shade" viewBox="0 0 10 10" refX="7" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
            <path d="M0 0 L10 5 L0 10 z" fill="rgb(var(--shade-rgb) / 0.5)" />
          </marker>
        </defs>
        {layers.map((layer, layerIndex) => (
          <g key={layerIndex} className={[styles.attackFlow, layer.dim ? styles.dim : ''].join(' ')} style={tintStyle(layer.color)}>
            {layer.paths.map((d, index) => <path key={index} className={styles.attack} d={d} />)}
          </g>
        ))}
        {from && targets.length > 0 && (
          <g className={styles.ink}>
            {/* the shadow, drawn as a dashed path of its own rather than a filter over the marching one */}
            <g className={styles.inkShade}>
              {targets.map((entry) => <path key={entry.index} d={curve(from, entry.point)} markerEnd="url(#arrow-ink-shade)" />)}
            </g>
            {targets.map((entry) => <path key={entry.index} d={curve(from, entry.point)} markerEnd="url(#arrow-ink)" />)}
            {targets.length > 1 && targets.map((entry) => {
              const mid = curveMidpoint(from, entry.point);
              return (
                <g key={`badge-${entry.index}`} className={styles.targetBadge} transform={`translate(${mid.x} ${mid.y})`}>
                  <circle r={16} />
                  <text dy="0.35em">{entry.index + 1}</text>
                </g>
              );
            })}
          </g>
        )}
      </svg>
    )}
    </>
  );
}
