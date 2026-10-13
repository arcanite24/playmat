import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Copy, Ghost, ListChecks, Trash2 } from 'lucide-react';
import { useState, type FormEvent } from 'react';
import { api } from '../connection';
import { toWire } from '../decks/deckModel';
import { formatDate, registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { useDecks } from '../stores/decks';
import { notify } from '../stores/toasts';
import type { GhostDeck } from '../../protocol/generated/views';
import { Button, IconButton } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { Field } from '../ui/Field';
import { ghostRecord } from './freePlay';
import styles from './Ghosts.module.css';

registerMessages(messages);

const MINE_KEY = ['ghostMine'];

/** Copy a ghost code, saying so (or showing it when the clipboard is out of reach). */
function useCopyCode() {
  const t = useT();
  return (code: string) => {
    navigator.clipboard.writeText(code)
      .then(() => notify(t('solo.ghost.copied'), t('solo.ghost.copiedDetail', { code })))
      .catch(() => notify(t('solo.ghost.codeTitle'), code));
  };
}

/**
 * Ghost decks (S047) on the deck shelf: share the selected deck under a code anyone can play against, and the list of
 * the decks you share with their records.
 */
export function GhostActions({ deckId, deckName }: { deckId: string; deckName: string }) {
  const t = useT();
  const [sharing, setSharing] = useState(false);
  const [listing, setListing] = useState(false);
  return (
    <>
      <Button variant="print" size="sm" icon={<Ghost size={16} />} onClick={() => setSharing(true)}>{t('solo.ghost.share')}</Button>
      <Button variant="quiet" size="sm" icon={<ListChecks size={16} />} onClick={() => setListing(true)}>{t('solo.ghost.mine')}</Button>
      <Dialog open={sharing} onOpenChange={setSharing} title={t('solo.ghost.shareTitle')} description={t('solo.ghost.shareLead')} width="sm">
        {sharing && <ShareForm key={deckId} deckId={deckId} deckName={deckName} onClose={() => setSharing(false)} />}
      </Dialog>
      <Dialog open={listing} onOpenChange={setListing} title={t('solo.ghost.mineTitle')} description={t('solo.ghost.mineLead')} width="md">
        {listing && <MyGhosts />}
      </Dialog>
    </>
  );
}

function ShareForm({ deckId, deckName, onClose }: { deckId: string; deckName: string; onClose(): void }) {
  const t = useT();
  const client = useQueryClient();
  const copy = useCopyCode();
  const [name, setName] = useState(deckName);
  const share = useMutation({
    mutationFn: async () => api.ghostShare(name.trim() || null, toWire(await useDecks.getState().loadList(deckId))),
    onSuccess: () => void client.invalidateQueries({ queryKey: MINE_KEY }),
  });
  const shared = share.data;

  function submit(event: FormEvent) {
    event.preventDefault();
    if (!share.isPending) share.mutate();
  }

  if (shared?.code) {
    return (
      <div className={styles.form}>
        <p className={styles.lead}>{t('solo.ghost.sharedLead', { name: shared.name ?? deckName })}</p>
        <div className={styles.codeRow}>
          <code className={styles.code}>{shared.code}</code>
          <Button variant="print" icon={<Copy size={16} />} onClick={() => copy(shared.code!)}>{t('solo.ghost.copy')}</Button>
        </div>
        <p className={styles.quiet}>{t('solo.ghost.sharedHint')}</p>
        <div className={styles.buttons}>
          <Button variant="decision" onClick={onClose}>{t('solo.ghost.done')}</Button>
        </div>
      </div>
    );
  }
  return (
    <form className={styles.form} onSubmit={submit}>
      <Field label={t('solo.ghost.name')} value={name} maxLength={60} autoFocus onChange={(event) => setName(event.target.value)}
        error={share.error ? share.error.message || t('solo.ghost.shareFailed') : null} />
      <div className={styles.buttons}>
        <Button variant="quiet" onClick={onClose}>{t('solo.ghost.cancel')}</Button>
        <Button variant="decision" type="submit" icon={<Ghost size={16} />} busy={share.isPending}>{t('solo.ghost.shareConfirm')}</Button>
      </div>
    </form>
  );
}

/** The decks you share as ghosts: code, record, and stop sharing. */
function MyGhosts() {
  const t = useT();
  const client = useQueryClient();
  const copy = useCopyCode();
  const mine = useQuery({ queryKey: MINE_KEY, queryFn: () => api.ghostMine() });
  const unshare = useMutation({
    mutationFn: (code: string) => api.ghostUnshare(code),
    onSuccess: (_, code) => {
      client.setQueryData<GhostDeck[]>(MINE_KEY, (list) => (list ?? []).filter((ghost) => ghost.code !== code));
    },
  });
  if (mine.isError) return <p className={styles.error} role="alert">{t('solo.ghost.mineFailed')}</p>;
  if (!mine.data) return <p className={styles.quiet}>{t('solo.ghost.loading')}</p>;
  if (mine.data.length === 0) return <p className={styles.quiet}>{t('solo.ghost.none')}</p>;
  return (
    <ul className={styles.list}>
      {mine.data.map((ghost) => {
        const record = ghostRecord(ghost);
        return (
          <li key={ghost.code} className={styles.ghost}>
            <div className={styles.text}>
              <b>{ghost.name}</b>
              <small>
                {t('solo.ghost.sharedOn', { date: formatDate(ghost.created ?? 0, { dateStyle: 'medium' }) })}
                {' · '}
                {record.played > 0 ? t('solo.ghost.record', { won: record.won, played: record.played }) : t('solo.ghost.unplayed')}
              </small>
            </div>
            <code className={styles.codeSmall}>{ghost.code}</code>
            <IconButton label={t('solo.ghost.copyCode', { code: ghost.code ?? '' })} icon={<Copy size={16} />} onClick={() => copy(ghost.code!)} />
            <IconButton
              label={t('solo.ghost.unshare', { name: ghost.name ?? '' })}
              icon={<Trash2 size={16} />}
              disabled={unshare.isPending}
              onClick={() => unshare.mutate(ghost.code!)}
            />
          </li>
        );
      })}
      {unshare.error && <li className={styles.error} role="alert">{t('solo.ghost.unshareFailed')}</li>}
    </ul>
  );
}
