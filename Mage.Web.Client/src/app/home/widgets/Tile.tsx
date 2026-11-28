import { ChevronRight } from 'lucide-react';
import type { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import styles from './Widgets.module.css';

/**
 * A widget's frame: its name, an optional link to where the progress is made, and the body. In Customize the link is
 * inert, so a drag or a tap edits the board instead of leaving Home.
 */
export function Tile({ title, to, linkLabel, editing, children, className }: {
  title: string;
  to?: string;
  linkLabel?: string;
  editing: boolean;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={[styles.tile, className ?? ''].join(' ')} aria-label={title}>
      <header className={styles.head}>
        <h3 className={styles.title}>{title}</h3>
        {to && !editing && (
          <Link to={to} className={styles.link} aria-label={linkLabel ?? title}>
            <ChevronRight size={16} aria-hidden="true" />
          </Link>
        )}
      </header>
      <div className={styles.body}>{children}</div>
    </section>
  );
}

/** a thin progress bar, 0 to 1 */
export function Bar({ value, label }: { value: number; label?: string }) {
  const clamped = Math.max(0, Math.min(1, Number.isFinite(value) ? value : 0));
  return (
    <span className={styles.bar} role={label ? 'img' : undefined} aria-label={label} aria-hidden={label ? undefined : true}>
      <i style={{ width: `${clamped * 100}%` }} />
    </span>
  );
}

export function Big({ children, tone }: { children: ReactNode; tone?: string }) {
  return <p className={styles.big} style={tone ? { color: tone } : undefined}>{children}</p>;
}

export function Note({ children }: { children: ReactNode }) {
  return <p className={styles.note}>{children}</p>;
}
