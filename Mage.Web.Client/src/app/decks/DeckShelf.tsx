import { Copy, Image as ImageIcon, Plus, Search, Share2, X } from 'lucide-react';
import { useMemo, useState, type FormEvent } from 'react';
import type { DeckCardLists } from '../../core/decks/types';
import { formatRules } from '../../core/decks/formats';
import { cleanTag, MAX_TAGS, tagCounts } from '../../core/decks/tags';
import type { SharedDeck } from '../../protocol/generated/views';
import { useCardImage } from '../../core/images/useCardImage';
import { useT } from '../i18n';
import { useDecks, type RosterDeck } from '../stores/decks';
import { copySharedDeck, useShareDeck, useSharedDecks, useSharingOn } from '../stores/sharedDecks';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { CoverDialog } from './CoverDialog';
import styles from './DeckShelf.module.css';

const errorText = (error: unknown) => (error instanceof Error ? error.message : String(error));

/** The shelf's filter (#200): every tag in use as a chip, and drafts shown or hidden. */
export function ShelfFilterBar({ decks }: { decks: RosterDeck[] }) {
  const t = useT();
  const filter = useDecks((state) => state.shelfFilter);
  const setFilter = useDecks((state) => state.setShelfFilter);
  const tags = useMemo(() => tagCounts(decks), [decks]);
  const drafts = decks.some((deck) => deck.draft);
  // a tag nobody uses any more doesn't keep hiding every deck
  const current = filter.tag && tags.some((entry) => entry.tag.toLowerCase() === filter.tag!.toLowerCase()) ? filter.tag : null;
  if (tags.length === 0 && !drafts && !filter.hideDrafts) return null;
  return (
    <div className={styles.filterBar} role="group" aria-label={t('decks.filter.label')}>
      <button type="button" className={current === null ? styles.chipOn : styles.chip} aria-pressed={current === null} onClick={() => setFilter({ tag: null })}>
        {t('decks.filter.all')}
      </button>
      {tags.map(({ tag, count }) => {
        const on = current?.toLowerCase() === tag.toLowerCase();
        return (
          <button key={tag} type="button" className={on ? styles.chipOn : styles.chip} aria-pressed={on} onClick={() => setFilter({ tag: on ? null : tag })}>
            {tag}<small>{count}</small>
          </button>
        );
      })}
      {(drafts || filter.hideDrafts) && (
        <label className={styles.check}>
          <input type="checkbox" checked={filter.hideDrafts} onChange={(event) => setFilter({ hideDrafts: event.target.checked })} />
          {t('decks.filter.hideDrafts')}
        </label>
      )}
    </div>
  );
}

/** A saved deck's tags, draft mark, cover and sharing, in the deck panel. */
export function DeckOrganizer({ deck }: { deck: RosterDeck }) {
  const t = useT();
  const [coverDeck, setCoverDeck] = useState<DeckCardLists | null>(null);
  const store = useDecks.getState;

  async function run(work: () => Promise<unknown>, failed: string) {
    try {
      await work();
    } catch (error) {
      notify(failed, errorText(error), 'error');
    }
  }

  return (
    <div className={styles.organizer}>
      <TagEditor deck={deck} onChange={(tags) => void run(() => store().setTags(deck.id, tags), t('decks.tags.failed'))} />
      <label className={styles.check}>
        <input type="checkbox" checked={deck.draft} onChange={(event) => void run(() => store().setDraft(deck.id, event.target.checked), t('decks.tags.failed'))} />
        {t('decks.draft')}
      </label>
      <div className={styles.row}>
        <Button
          variant="print"
          size="sm"
          icon={<ImageIcon size={16} />}
          onClick={() => void run(async () => setCoverDeck(await store().loadList(deck.id)), t('decks.cover.failed'))}
        >
          {t('decks.cover.choose')}
        </Button>
      </div>
      <ShareToggle deck={deck} />
      <CoverDialog
        open={!!coverDeck}
        onOpenChange={(open) => !open && setCoverDeck(null)}
        deck={coverDeck}
        onChange={(next) => void run(
          () => store().updateSaved(deck.id, (current) => ({ ...current, coverCard: next.coverCard, coverPinned: next.coverPinned })),
          t('decks.cover.failed'),
        )}
      />
    </div>
  );
}

function TagEditor({ deck, onChange }: { deck: RosterDeck; onChange(tags: string[]): void }) {
  const t = useT();
  const [text, setText] = useState('');
  const full = deck.tags.length >= MAX_TAGS;

  function add(event: FormEvent) {
    event.preventDefault();
    const typed = text.split(',').map(cleanTag).filter(Boolean);
    if (typed.length === 0) return;
    onChange([...deck.tags, ...typed]);
    setText('');
  }

  return (
    <div>
      <h3 className={styles.label}>{t('decks.tags')}</h3>
      {deck.tags.length > 0 && (
        <ul className={styles.tags}>
          {deck.tags.map((tag) => (
            <li key={tag} className={styles.tag}>
              {tag}
              <button type="button" aria-label={t('decks.tags.remove', { tag })} onClick={() => onChange(deck.tags.filter((other) => other !== tag))}>
                <X size={13} aria-hidden="true" />
              </button>
            </li>
          ))}
        </ul>
      )}
      <form className={styles.tagForm} onSubmit={add}>
        <input
          className={styles.tagInput}
          value={text}
          disabled={full}
          maxLength={60}
          placeholder={full ? t('decks.tags.full', { count: MAX_TAGS }) : t('decks.tags.placeholder')}
          aria-label={t('decks.tags.add')}
          onChange={(event) => setText(event.target.value)}
        />
        <button type="submit" className={styles.tagAdd} disabled={full || !cleanTag(text)} aria-label={t('decks.tags.add')}>
          <Plus size={15} aria-hidden="true" />
        </button>
      </form>
    </div>
  );
}

/** Share with this server (#137): shown where the server allows it. */
function ShareToggle({ deck }: { deck: RosterDeck }) {
  const t = useT();
  const on = useSharingOn();
  const list = useSharedDecks();
  const share = useShareDeck();
  if (!on) return null;
  const shared = (list.data ?? []).some((entry) => entry.mine && entry.deckId === deck.id);
  const pending = share.isPending ? share.variables?.shared : undefined;
  return (
    <div>
      <label className={styles.check}>
        <input
          type="checkbox"
          checked={pending ?? shared}
          disabled={share.isPending || list.isPending}
          onChange={(event) => share.mutate(
            { id: deck.id, shared: event.target.checked },
            { onError: (error) => notify(t('decks.share.failed'), errorText(error), 'error') },
          )}
        />
        <Share2 size={15} aria-hidden="true" />
        {t('decks.share')}
      </label>
      <p className={styles.hint}>{t('decks.share.detail')}</p>
    </div>
  );
}

/** The decks others share on this server, to copy into your own (#137). */
export function SharedDecksButton() {
  const t = useT();
  const on = useSharingOn();
  const [open, setOpen] = useState(false);
  if (!on) return null;
  return (
    <>
      <Button size="sm" icon={<Share2 size={16} />} onClick={() => setOpen(true)}>{t('decks.shared.open')}</Button>
      <Dialog open={open} onOpenChange={setOpen} title={t('decks.shared.title')} description={t('decks.shared.lead')} width="lg">
        {open && <SharedDeckList onCopied={() => setOpen(false)} />}
      </Dialog>
    </>
  );
}

function SharedDeckList({ onCopied }: { onCopied(): void }) {
  const t = useT();
  const list = useSharedDecks();
  const [query, setQuery] = useState('');
  const [copying, setCopying] = useState<string | null>(null);
  const needle = query.trim().toLowerCase();
  const decks = (list.data ?? []).filter((deck) => !needle
    || [deck.name, deck.owner, deck.format ? formatRules(deck.format).label : ''].some((text) => (text ?? '').toLowerCase().includes(needle)));

  async function copy(deck: SharedDeck) {
    const key = `${deck.owner}/${deck.deckId}`;
    setCopying(key);
    try {
      await copySharedDeck(deck);
      notify(t('decks.shared.copied'), deck.name ?? '');
      onCopied();
    } catch (error) {
      notify(t('decks.shared.copyFailed'), errorText(error), 'error');
    } finally {
      setCopying(null);
    }
  }

  if (list.isPending) return <p className={styles.hint}>{t('decks.shared.loading')}</p>;
  if (list.isError) return <p className={styles.hint}>{t('decks.shared.failed')} {errorText(list.error)}</p>;
  if ((list.data ?? []).length === 0) return <p className={styles.hint}>{t('decks.shared.empty')}</p>;
  return (
    <div className={styles.shared}>
      <label className={styles.search}>
        <Search size={16} aria-hidden="true" />
        <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t('decks.shared.search')} aria-label={t('decks.shared.search')} />
      </label>
      <ul className={styles.sharedList}>
        {decks.map((deck) => {
          const key = `${deck.owner}/${deck.deckId}`;
          return (
            <li key={key} className={styles.sharedRow}>
              <SharedCover deck={deck} />
              <span className={styles.sharedText}>
                <b>{deck.name}</b>
                <small>
                  {deck.mine ? t('decks.shared.yours') : t('decks.shared.by', { owner: deck.owner ?? '' })}
                  {deck.format ? ` · ${formatRules(deck.format).label}` : ''}
                  {` · ${t('decks.shared.cards', { count: deck.cardCount ?? 0 })}`}
                </small>
              </span>
              {deck.colors && (
                <span className={styles.pips} aria-label={t('ui.colors', { colors: deck.colors })}>
                  {[...deck.colors].map((color) => <i key={color} className={`ms ms-cost ms-${color.toLowerCase()}`} aria-hidden="true" />)}
                </span>
              )}
              {!deck.mine && (
                <Button variant="print" size="sm" icon={<Copy size={15} />} busy={copying === key} disabled={copying !== null} onClick={() => void copy(deck)}>
                  {t('decks.shared.copy')}
                </Button>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function SharedCover({ deck }: { deck: SharedDeck }) {
  const cover = deck.coverSetCode && deck.coverCardNumber ? { setCode: deck.coverSetCode, cardNumber: deck.coverCardNumber, name: deck.coverName } : null;
  const art = useCardImage(cover, 'front', 'art_crop');
  return <span className={styles.sharedArt} style={typeof art === 'string' ? { backgroundImage: `url("${art}")` } : undefined} aria-hidden="true" />;
}
