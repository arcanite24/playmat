import { useState } from 'react';
import type { GameView } from '../../protocol/generated/views';
import { useT } from '../i18n';
import { readJson, writeJson } from '../stores/persist';
import { Button } from '../ui/Button';
import { engineOf } from './engine';
import styles from './EngineBadge.module.css';

const HIDE_NOTICE_KEY = 'playmat.rustEngineNotice.hidden';

/** the games whose notice was closed in this tab, so a reconnect doesn't show it again */
const closedGames = new Set<string>();

/**
 * A game the Rust engine plays: a chip on the board for as long as it lasts, and once per game, as it starts, a note
 * that says so (unless the player asked not to see it again).
 */
export function EngineBadge({ gameId, view, showNotice }: { gameId: string; view: GameView | null | undefined; showNotice: boolean }) {
  const t = useT();
  const rust = engineOf(view) === 'rust';
  const [closedFor, setClosedFor] = useState<string | null>(null);
  const [dontShow, setDontShow] = useState(false);
  const notice = rust && showNotice && closedFor !== gameId && !closedGames.has(gameId) && !readJson<boolean>(HIDE_NOTICE_KEY, false);

  if (!rust) return null;

  function close() {
    if (dontShow) writeJson(HIDE_NOTICE_KEY, true);
    closedGames.add(gameId);
    setClosedFor(gameId);
  }

  return (
    <>
      <div className={styles.chip} title={t('match.engine.rust.tooltip')} role="status" aria-label={t('match.engine.rust.tooltip')}>
        <span className={styles.gear} aria-hidden="true" />
        <span>{t('match.engine.rust')}</span>
        <span className={styles.beta}>{t('match.engine.beta')}</span>
      </div>
      {notice && (
        <div className={styles.notice} role="dialog" aria-modal="false" aria-labelledby="rust-engine-notice-title">
          <div className={styles.noticeHead}>
            <span className={styles.gearLarge} aria-hidden="true" />
            <div>
              <p className={styles.kicker}>{t('match.engine.beta')}</p>
              <h2 id="rust-engine-notice-title" className={styles.title}>{t('match.engine.notice.title')}</h2>
            </div>
          </div>
          <p className={styles.body}>{t('match.engine.notice.body')}</p>
          <p className={styles.hint}>{t('match.engine.notice.settings')}</p>
          <div className={styles.actions}>
            <label className={styles.dontShow}>
              <input type="checkbox" checked={dontShow} onChange={(event) => setDontShow(event.target.checked)} />
              {t('match.engine.notice.dontShow')}
            </label>
            <Button variant="decision" onClick={close} autoFocus>{t('match.engine.notice.ok')}</Button>
          </div>
        </div>
      )}
    </>
  );
}
