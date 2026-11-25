import { useRef, type KeyboardEvent, type PointerEvent } from 'react';
import styles from './DeckBuilder.module.css';

/**
 * The seam between the binder and the deck panel: drag it (or focus it and use the arrow keys) to give the deck more
 * room; double-click or Enter puts it back. `width` is the deck panel's, which grows as the seam moves left.
 */
export function Splitter({ width, min, max, defaultWidth, onChange, onCommit }: {
  width: number;
  min: number;
  max: number;
  defaultWidth: number;
  /** while dragging */
  onChange(width: number): void;
  /** once the drag ends or a key is pressed: the width to remember */
  onCommit(width: number): void;
}) {
  const drag = useRef<{ x: number; width: number; latest: number } | null>(null);
  const clamp = (value: number) => Math.round(Math.min(max, Math.max(min, value)));

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { x: event.clientX, width, latest: width };
  };
  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (!drag.current) return;
    const next = clamp(drag.current.width + (drag.current.x - event.clientX));
    drag.current.latest = next;
    onChange(next);
  };
  const onPointerUp = () => {
    if (!drag.current) return;
    onCommit(drag.current.latest);
    drag.current = null;
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.shiftKey ? 120 : 24;
    const next = {
      ArrowLeft: width + step,
      ArrowRight: width - step,
      Home: max,
      End: min,
      Enter: defaultWidth,
    }[event.key];
    if (next === undefined) return;
    event.preventDefault();
    onCommit(clamp(next));
  };

  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label="Deck panel width"
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={Math.round(width)}
      aria-valuetext={`${Math.round(width)} pixels`}
      title="Drag to resize the deck. Double-click to reset."
      tabIndex={0}
      className={styles.splitter}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onDoubleClick={() => onCommit(clamp(defaultWidth))}
      onKeyDown={onKeyDown}
    >
      <span className={styles.splitterGrip} aria-hidden="true" />
    </div>
  );
}
