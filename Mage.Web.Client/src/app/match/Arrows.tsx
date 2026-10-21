import { useEffect, useState } from 'react';
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

/**
 * Arrows on the table: attackers to what they attack (oxblood), blockers to attackers: what the top of the stack targets (ink), and while choosing targets,
 * a live arrow from the spell to the pointer (amber).
 */
export function Arrows({ sourceId, targetIds, live, links, attacks, leads }: {
  sourceId: string | null;
  targetIds: string[];
  live: boolean;
  /** blocker -> attacker pairs in the current combat */
  links: [string, string][];
  /** attacker -> player or planeswalker it attacks */
  attacks: [string, string][];
  /** each permanent's stack lead: a stack of attacking tokens draws one arrow */
  leads: ReadonlyMap<string, string>;
}) {
  const center = useObjectCenter();
  const stage = useStage();
  const [, setTick] = useState(0);
  const [pointer, setPointer] = useState<Point | null>(null);
  const dragging = useMatchUi((state) => state.dragging);
  // a creature dragged toward an attacker to block it: a live line that snaps to the attacker under the pointer
  const blockDrag = useMatchUi((state) => state.blockDrag);

  const targetKey = targetIds.join(',') + '|' + [...links, ...attacks].map((link) => link.join('>')).join(',');
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
  const targets = from ? targetIds.map(center).filter((point): point is Point => !!point) : [];
  const combat = links
    .map(([blocker, attacker]) => [center(blocker), center(attacker)] as const)
    .filter((pair): pair is readonly [Point, Point] => !!pair[0] && !!pair[1]);
  // one arrow per attacking stack; many stacks at one defender join into a trunk with a count
  const single: (readonly [Point, Point])[] = [];
  const feeders: string[] = [];
  const trunks: { d: string; at: Point; count: number }[] = [];
  for (const bundle of bundleAttacks(attacks, leads)) {
    const to = center(bundle.defender);
    if (!to) continue;
    const sources = bundle.sources.map((source) => center(source.id)).filter((point): point is Point => !!point);
    if (sources.length === 0) continue;
    if (isTrunk(bundle)) {
      const join = trunkJoin(sources, to);
      sources.forEach((source) => feeders.push(feederPath(source, join)));
      const d = attackPath(join, to, 0, 1);
      // the count sits where the feeders meet, clear of the defender's board
      trunks.push({ d, at: join, count: bundle.count });
    } else {
      sources.forEach((source) => single.push([source, to]));
    }
  }
  const assault = single;
  const blockFrom = blockDrag ? center(blockDrag.blockerId) : null;
  const blockTo = blockDrag?.attackerId ? center(blockDrag.attackerId) : pointer;
  const blockLine = blockFrom && blockTo ? [blockFrom, blockTo] as const : null;
  const attackPaths = [...assault.map(([attacker, defender], index) => attackPath(attacker, defender, index, assault.length)), ...trunks.map((trunk) => trunk.d)];
  if (dragging || (!blockLine && combat.length === 0 && attackPaths.length === 0 && (!from || (targets.length === 0 && !(live && pointer))))) return null;

  return (
    <svg className={styles.arrows} width={stage.width} height={stage.height} aria-hidden="true">
      <defs>
        <marker id="arrow-ink" viewBox="0 0 10 10" refX="7" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
          <path d="M0 0 L10 5 L0 10 z" fill="var(--ink)" />
        </marker>
        <marker id="arrow-decision" viewBox="0 0 10 10" refX="7" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
          <path d="M0 0 L10 5 L0 10 z" fill="var(--decision)" />
        </marker>
        <marker id="arrow-attack" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="4.2" markerHeight="4.2" orient="auto-start-reverse">
          <path d="M0 0 L10 5 L0 10 L2.5 5 z" fill="var(--oxblood-lit)" />
        </marker>
      </defs>
      {/* each layer is filtered once as a whole: a filter per arrow, repainted under the marching dashes, stalls the frame when a horde attacks */}
      {attackPaths.length > 0 && (
        <>
          <g className={styles.attackGlow}>
            {feeders.map((d, index) => <path key={`f${index}`} className={[styles.attack, styles.feeder].join(' ')} d={d} />)}
            {attackPaths.map((d, index) => <path key={index} className={styles.attack} d={d} />)}
          </g>
          <g className={styles.attackBody}>
            {feeders.map((d, index) => <path key={`f${index}`} className={[styles.attack, styles.feeder].join(' ')} d={d} />)}
            {attackPaths.map((d, index) => <path key={index} className={styles.attack} d={d} markerEnd="url(#arrow-attack)" />)}
          </g>
          <g className={styles.attackFlow}>
            {attackPaths.map((d, index) => <path key={index} className={styles.attack} d={d} />)}
          </g>
          {trunks.map((trunk, index) => (
            <g key={index} className={styles.attackCount} transform={`translate(${trunk.at.x} ${trunk.at.y})`}>
              <circle r={26} />
              <text dy="0.35em">×{trunk.count}</text>
            </g>
          ))}
        </>
      )}
      {combat.length > 0 && (
        <g className={styles.block}>
          {combat.map(([blocker, attacker], index) => (
            <line key={index} x1={blocker.x} y1={blocker.y} x2={attacker.x} y2={attacker.y} />
          ))}
        </g>
      )}
      {from && targets.length > 0 && (
        <g className={styles.ink}>
          {targets.map((to, index) => <path key={index} d={curve(from, to)} markerEnd="url(#arrow-ink)" />)}
        </g>
      )}
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
  );
}
