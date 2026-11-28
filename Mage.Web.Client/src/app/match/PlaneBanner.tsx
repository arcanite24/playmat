import { ChevronDown, ChevronUp, Dices, Sparkles } from 'lucide-react';
import { useState } from 'react';
import type { CardView, GameView } from '../../protocol/generated/views';
import { useMatchUi } from './matchUi';
import styles from './MatchStage.module.css';

function activePlane(view: GameView | null | undefined): CardView | null {
  for (const player of view?.players ?? []) {
    for (const obj of (player.commandList ?? []) as CardView[]) {
      if (obj.mageObjectType === 'PLANE') return obj;
    }
  }
  return null;
}

/**
 * Collapsible Planechase banner at the top of the battlefield: shows the active Plane's name,
 * expands to show its static and chaos rules text, and exposes a one-click Roll Planar Die button
 * whenever the player has sorcery-speed priority to roll.
 */
export function PlaneBanner({
  view,
  playable,
  canAct,
  onRoll,
}: {
  view: GameView | null | undefined;
  playable: ReadonlySet<string>;
  canAct: boolean;
  onRoll(planeId: string): void;
}) {
  const [open, setOpen] = useState(false);
  const setZoom = useMatchUi((state) => state.setZoom);
  const plane = activePlane(view);
  if (!plane || !plane.id) return null;

  const canRoll = canAct && playable.has(plane.id);
  const rules = (plane.rules ?? []).filter(Boolean);

  return (
    <aside className={styles.planeBanner} aria-label={`Active plane: ${plane.name ?? 'Plane'}`}>
      <div className={styles.planeHead}>
        <button
          type="button"
          className={styles.planeToggle}
          aria-expanded={open}
          onClick={() => setOpen((was) => !was)}
          onPointerEnter={(event) => {
            if (event.pointerType !== 'touch') setZoom({ card: plane, x: event.clientX, y: event.clientY, sleeve: '' });
          }}
          onPointerLeave={() => setZoom(null)}
        >
          <Sparkles size={14} aria-hidden="true" />
          <span className={styles.planeKicker}>Plane</span>
          <b className={styles.planeName}>{plane.name}</b>
          {open ? <ChevronUp size={14} aria-hidden="true" /> : <ChevronDown size={14} aria-hidden="true" />}
        </button>
        {canRoll && (
          <button
            type="button"
            className={styles.planeRoll}
            onClick={() => onRoll(plane.id!)}
          >
            <Dices size={14} aria-hidden="true" />
            <span>Roll Planar Die</span>
          </button>
        )}
      </div>
      {open && rules.length > 0 && (
        <ul className={styles.planeRules}>
          {rules.map((line, idx) => <li key={idx}>{line}</li>)}
        </ul>
      )}
    </aside>
  );
}
