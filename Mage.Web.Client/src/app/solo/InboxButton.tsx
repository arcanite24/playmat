import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Ghost, Inbox } from 'lucide-react';
import { useState } from 'react';
import { api } from '../connection';
import { registerMessages, useI18n, useT } from '../i18n';
import messages from '../i18n/en/career';
import { useSession } from '../stores/session';
import type { InboxNote } from '../../protocol/generated/views';
import { agoParts, unseenCount } from './freePlay';
import styles from './InboxButton.module.css';

registerMessages(messages);

const INBOX_KEY = ['inbox'];

/** "3 hours ago", in the app's language. */
function useAgo() {
  const locale = useI18n((state) => state.locale);
  const format = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });
  return (at: number, now: number) => {
    const { value, unit } = agoParts(at, now);
    return format.format(value, unit);
  };
}

/** The rail's inbox (S047): notes such as how your ghost decks did, with the unseen ones counted on the button. */
export function InboxButton() {
  const t = useT();
  const ago = useAgo();
  const client = useQueryClient();
  const signedIn = useSession((state) => state.connection === 'open' && !!state.userName);
  const inbox = useQuery({ queryKey: INBOX_KEY, queryFn: () => api.inbox(), enabled: signedIn, refetchInterval: 60_000, staleTime: 30_000 });
  const notes = inbox.data ?? [];
  const unseen = unseenCount(notes);
  // the notes that were new when the panel opened keep their mark while it's open; the badge clears at once
  const [fresh, setFresh] = useState<ReadonlySet<number | undefined>>(new Set());

  function opened(open: boolean) {
    if (!open) setFresh(new Set());
    if (!open || unseen === 0) return;
    setFresh(new Set(notes.filter((note) => !note.seen).map((note) => note.id)));
    void api.inboxSeen().then(() => {
      client.setQueryData<InboxNote[]>(INBOX_KEY, (list) => (list ?? []).map((note) => ({ ...note, seen: true })));
    }).catch(() => undefined);
  }

  if (!signedIn || inbox.isError) return null;
  return (
    <DropdownMenu.Root onOpenChange={opened}>
      <DropdownMenu.Trigger className={styles.button} aria-label={unseen > 0 ? t('solo.inbox.labelUnseen', { count: unseen }) : t('solo.inbox.label')}>
        <Inbox size={18} aria-hidden="true" />
        {unseen > 0 && <span className={styles.unseen} aria-hidden="true">{unseen > 99 ? '99+' : unseen}</span>}
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content className={styles.panel} align="end" sideOffset={8}>
          <DropdownMenu.Label className={styles.title}>{t('solo.inbox.title')}</DropdownMenu.Label>
          {notes.length === 0 && <p className={styles.empty}>{t('solo.inbox.empty')}</p>}
          {notes.map((note) => (
            <DropdownMenu.Item key={note.id} className={[styles.note, fresh.has(note.id) || !note.seen ? styles.fresh : ''].join(' ')} onSelect={(event) => event.preventDefault()}>
              {note.kind === 'ghost' ? <Ghost size={16} aria-hidden="true" /> : <Inbox size={16} aria-hidden="true" />}
              <span className={styles.text}>
                <span>{note.text}</span>
                <small>{ago(note.created ?? 0, inbox.dataUpdatedAt)}</small>
              </span>
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
