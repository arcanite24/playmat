import { Bot, Lock, UserRound } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { formatRules } from '../../core/decks/formats';
import { useTables } from '../queries';
import { useDecks } from '../stores/decks';
import { useDeckRoster } from '../stores/useDeckRoster';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { Field } from '../ui/Field';
import { Zone } from '../ui/Zone';
import { joinTable } from './joinTable';
import styles from './JoinScreen.module.css';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/tables';

registerMessages(messages);

/** Where an invite link leads: the table, and a seat at it with one of your decks. */
export function JoinScreen() {
  const t = useT();
  const { tableId = '' } = useParams();
  const navigate = useNavigate();
  const { data: tables = [], dataUpdatedAt, refetch } = useTables();
  // the cached list may predate the table: only a list fetched after the link was opened can say it's gone
  const [openedAt] = useState(() => Date.now());
  useEffect(() => {
    void refetch();
  }, [refetch]);
  const fresh = dataUpdatedAt >= openedAt;
  const userName = useSession((state) => state.userName);
  const decks = useDecks();
  const roster = useDeckRoster();
  const [deckId, setDeckId] = useState<string | null>(null);
  const [password, setPassword] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!decks.loaded) void decks.refresh();
  }, [decks]);

  const table = tables.find((candidate) => candidate.tableId === tableId) ?? null;
  const chosen = roster.find((deck) => deck.id === (deckId ?? decks.selectedId)) ?? roster[0] ?? null;
  const seated = !!table?.seats?.some((seat) => seat.playerName === userName);
  const open = table?.tableState === 'WAITING' && !!table.seats?.some((seat) => !seat.playerName);

  async function join() {
    if (!table || !chosen) return;
    setBusy(true);
    setError(null);
    try {
      await joinTable(table, chosen.id, password);
      decks.select(chosen.id);
      navigate('/tables');
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }

  if (!table) {
    return (
      <div className={styles.page}>
        <Zone label={t('join.invite')} className={styles.card}>
          {!fresh ? <p className={styles.note} role="status">{t('join.looking')}</p> : (
            <>
              <p className={styles.note}>{t('join.gone')}</p>
              <Button variant="print" onClick={() => navigate('/tables')}>{t('join.seeOpen')}</Button>
            </>
          )}
        </Zone>
      </div>
    );
  }

  return (
    <div className={styles.page}>
      <Zone label={t('join.invited')} className={styles.card}>
        <h2 className={styles.name}>
          {table.passworded && <Lock size={18} aria-label={t('tables.passwordRequired')} />}
          {table.tableName}
        </h2>
        <p className={styles.meta}>
          {formatRules(table.deckType ?? '').label} · {table.gameType} · {t('join.hostedBy', { name: (table.controllerName ?? '').split(', ')[0] })}
        </p>
        <ul className={styles.seats} aria-label={t('tables.seats')}>
          {(table.seats ?? []).map((seat, index) => (
            <li key={index} className={seat.playerName ? styles.taken : styles.free}>
              {seat.playerType && seat.playerType !== 'HUMAN' ? <Bot size={14} aria-hidden="true" /> : <UserRound size={14} aria-hidden="true" />}
              {seat.playerName || t('tables.openSeat')}
            </li>
          ))}
        </ul>

        {seated ? (
          <>
            <p className={styles.note}>{t('join.seated')}</p>
            <Button variant="decision" onClick={() => navigate('/tables')}>{t('join.goToTables')}</Button>
          </>
        ) : !open ? (
          <>
            <p className={styles.note}>{t('join.full')}</p>
            <Button variant="print" onClick={() => navigate('/tables')}>{t('join.seeOpen')}</Button>
          </>
        ) : (
          <form
            className={styles.form}
            onSubmit={(event) => {
              event.preventDefault();
              void join();
            }}
          >
            <label className={styles.select}>
              <span>{t('join.deck')}</span>
              <select value={chosen?.id ?? ''} onChange={(event) => setDeckId(event.target.value)} disabled={roster.length === 0}>
                {roster.map((deck) => <option key={deck.id} value={deck.id}>{deck.name}</option>)}
              </select>
            </label>
            {table.passworded && (
              <Field label={t('tables.password')} type="password" value={password} onChange={(event) => setPassword(event.target.value)} hint={t('join.passwordHint')} />
            )}
            {error && <p className={styles.error} role="alert">{error}</p>}
            <div className={styles.actions}>
              <Button variant="quiet" onClick={() => navigate('/tables')}>{t('join.notNow')}</Button>
              <Button type="submit" variant="decision" busy={busy} disabled={!chosen || (table.passworded && !password)}>{t('join.takeSeat')}</Button>
            </div>
          </form>
        )}
      </Zone>
    </div>
  );
}
