import { ScrollText, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import type { ChatMessage } from '../../protocol/generated/views';
import { events } from '../connection';
import styles from './MatchStage.module.css';

const SCHEME_RE = /Scheme in motion — ([^:]+): (.+)$/;

/**
 * Dramatic Archenemy Scheme reveal banner (Phase 5.2): listens to game chat/status events for newly triggered Schemes
 * ("Scheme in motion — <Name>: <Text>") and briefly displays the active Scheme across the top of the table.
 */
export function SchemeBanner() {
  const [active, setActive] = useState<{ key: number; name: string; text: string } | null>(null);
  const seq = useRef(0);

  useEffect(() => {
    let timer: number | null = null;
    const stop = events.on('CHATMESSAGE', (message: ChatMessage) => {
      const raw = (message?.message ?? '').replace(/<[^>]+>/g, '').trim();
      const match = SCHEME_RE.exec(raw);
      if (!match) return;
      const key = ++seq.current;
      setActive({ key, name: match[1].trim(), text: match[2].trim() });
      if (timer) window.clearTimeout(timer);
      timer = window.setTimeout(() => {
        setActive((current) => (current?.key === key ? null : current));
      }, 5500);
    });
    return () => {
      stop();
      if (timer) window.clearTimeout(timer);
    };
  }, []);

  if (!active) return null;

  return (
    <aside className={styles.schemeBanner} role="status" aria-live="polite" aria-label={`Scheme in motion: ${active.name}`}>
      <span className={styles.schemeIcon} aria-hidden="true"><ScrollText size={18} /></span>
      <div className={styles.schemeBody}>
        <span className={styles.schemeKicker}>Scheme in Motion</span>
        <b className={styles.schemeName}>{active.name}</b>
        <p className={styles.schemeText}>{active.text}</p>
      </div>
      <button type="button" className={styles.schemeClose} aria-label="Dismiss scheme banner" onClick={() => setActive(null)}>
        <X size={15} aria-hidden="true" />
      </button>
    </aside>
  );
}
