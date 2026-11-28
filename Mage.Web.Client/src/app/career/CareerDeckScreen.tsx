import { useQuery } from '@tanstack/react-query';
import { Copy, Crown, Plus, Shield, Trash2 } from 'lucide-react';
import { memo, useCallback, useEffect, useMemo, useState } from 'react';
import { Navigate, useNavigate } from 'react-router-dom';
import type { CardView, CareerSavedDeck } from '../../protocol/generated/views';
import { parseIdentity } from '../../core/decks/formats';
import type { DeckCardInfo, DeckCardLists } from '../../core/decks/types';
import { api } from '../connection';
import { CardFilterBar } from '../decks/CardFilterBar';
import { useCardInfo } from '../decks/cardInfo';
import { InspectButton } from '../decks/CardInspect';
import { entryKey } from '../decks/deckModel';
import { EMPTY_FILTERS, matchesFilters, type CardFilters } from '../decks/filters';
import { DeckBuilder, type BinderProps, type BuilderMode } from '../decks/DeckBuilderScreen';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { useSession } from '../stores/session';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { Dialog } from '../ui/Dialog';
import { ensureCareerDeck, saveCareerDeck, toCareerDeck } from './careerDeck';
import { deleteCareerDeck, saveNamedCareerDeck, selectCareerDeck, useCareerCollection, useCareerDecks, useCareerState } from './careerData';
import { careerLimit, COMMANDER_FORMAT, CONSTRUCTED_FORMAT, isCommanderFormat, mainDeckSize, ownedByName } from './careerModel';
import builder from '../decks/DeckBuilder.module.css';
import styles from './Career.module.css';

registerMessages(messages);

function savedDeckCount(saved: CareerSavedDeck): number {
  const main = (saved.deck?.cards ?? []).reduce((sum, entry) => sum + (entry.amount ?? 0), 0);
  if (isCommanderFormat(saved.format)) {
    const cmdr = (saved.deck?.sideboard ?? []).reduce((sum, entry) => sum + (entry.amount ?? 0), 0);
    return main + cmdr;
  }
  return main;
}

function canBeCommander(card: CardView): boolean {
  const supers = card.superTypes ?? [];
  const types = card.cardTypes ?? [];
  if (supers.includes('LEGENDARY') && types.includes('CREATURE')) return true;
  return (card.rules ?? []).some((rule) => /can be your commander/i.test(rule));
}

function cardFitsIdentity(card: CardView, identity: readonly string[]): boolean {
  const allowed = new Set(identity);
  const color = card.color ?? {};
  if (color.white && !allowed.has('W')) return false;
  if (color.blue && !allowed.has('U')) return false;
  if (color.black && !allowed.has('B')) return false;
  if (color.red && !allowed.has('R')) return false;
  if (color.green && !allowed.has('G')) return false;
  return parseIdentity(card.originalColorIdentity).every((letter) => allowed.has(letter));
}

/** The Career decks in the deck builder: only owned cards, as many as owned; basic lands are free. */
export function CareerDeckScreen() {
  const t = useT();
  const navigate = useNavigate();
  const user = useSession((state) => state.userName);
  const state = useCareerState();
  const profile = state.data?.profile;
  const collection = useCareerCollection(!!profile);
  const savedDecks = useCareerDecks(!!profile);
  const [fallbackDeck, setFallbackDeck] = useState<DeckCardLists | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);

  const decks = useMemo(() => savedDecks.data ?? [], [savedDecks.data]);
  const activeSaved = useMemo(() => decks.find((candidate) => candidate.active) ?? decks[0] ?? null, [decks]);
  const activeDeck = useMemo(() => (activeSaved ? toCareerDeck(activeSaved) : null), [activeSaved]);

  const opened = !!profile;
  const starter = profile?.starter;
  useEffect(() => {
    if (!opened || activeDeck || savedDecks.isPending) return;
    let cancelled = false;
    void ensureCareerDeck(user, starter).then((loaded) => !cancelled && setFallbackDeck(loaded));
    return () => {
      cancelled = true;
    };
  }, [user, opened, starter, activeDeck, savedDecks.isPending]);

  const deck = activeDeck ?? fallbackDeck;
  const activeId = activeSaved?.id ?? null;

  const owned = useMemo(() => ownedByName(collection.data ?? []), [collection.data]);
  const activeFormat = activeSaved?.format ?? deck?.format ?? CONSTRUCTED_FORMAT;
  const save = useCallback((next: DeckCardLists) => {
    if (activeId) {
      void saveNamedCareerDeck(activeId, next.name || activeSaved?.name || t('career.nav.deck'), next, true, next.format || activeFormat)
        .catch((error) => notify(t('career.decks.saveFailed'), error instanceof Error ? error.message : String(error), 'error'));
      return;
    }
    void saveCareerDeck(user, next);
  }, [activeId, activeSaved?.name, activeFormat, user, t]);

  const mode = useMemo<BuilderMode>(() => ({
    Binder: OwnedBinder,
    allowance: (name) => careerLimit(name, owned),
    save,
    doneLabel: t('career.title'),
    onDone: () => navigate('/career'),
    onPlay: () => navigate('/career'),
  }), [owned, save, navigate, t]);

  async function onSwitch(id: string) {
    if (!id || id === activeId) return;
    setBusy(true);
    try {
      await selectCareerDeck(id);
    } catch (error) {
      notify(t('career.decks.switchFailed'), error instanceof Error ? error.message : String(error), 'error');
    } finally {
      setBusy(false);
    }
  }

  async function onCreateNew(format?: string) {
    setBusy(true);
    try {
      const isCmdr = isCommanderFormat(format);
      const name = isCmdr ? t('career.decks.newCommanderName') : t('career.decks.newName');
      await saveNamedCareerDeck(
        null,
        name,
        { name, ...(isCmdr ? { format: COMMANDER_FORMAT } : {}), cards: [], sideboard: [] },
        true,
        isCmdr ? COMMANDER_FORMAT : undefined,
      );
    } catch (error) {
      notify(t('career.decks.saveFailed'), error instanceof Error ? error.message : String(error), 'error');
    } finally {
      setBusy(false);
    }
  }

  async function onDuplicate() {
    if (!deck) return;
    setBusy(true);
    try {
      const copyName = t('career.decks.copyOf', { name: deck.name || t('career.nav.deck') });
      await saveNamedCareerDeck(null, copyName, { ...deck, name: copyName }, true, activeFormat);
    } catch (error) {
      notify(t('career.decks.saveFailed'), error instanceof Error ? error.message : String(error), 'error');
    } finally {
      setBusy(false);
    }
  }

  async function onDelete() {
    if (!activeId) return;
    setBusy(true);
    try {
      await deleteCareerDeck(activeId);
      setConfirmDelete(false);
    } catch (error) {
      notify(t('career.decks.deleteFailed'), error instanceof Error ? error.message : String(error), 'error');
    } finally {
      setBusy(false);
    }
  }

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;
  if (!deck || collection.isPending || savedDecks.isPending) return <p className={styles.note}>{t('career.loading')}</p>;

  return (
    <div className={styles.deckShell}>
      <div className={styles.deckBar}>
        <label className={styles.deckPicker}>
          <span className={styles.deckPickerLabel}>{t('career.decks.choose')}</span>
          <select
            value={activeId ?? ''}
            disabled={busy || decks.length === 0}
            onChange={(event) => void onSwitch(event.target.value)}
            aria-label={t('career.decks.choose')}
          >
            {decks.map((item) => (
              <option key={item.id} value={item.id}>
                {isCommanderFormat(item.format) ? `[${t('career.track.commander')}] ` : ''}{item.name} ({savedDeckCount(item)})
              </option>
            ))}
          </select>
          {isCommanderFormat(activeFormat) && (
            <span className={styles.formatPill}>
              <Crown size={13} aria-hidden="true" />
              {t('career.track.commander')}
            </span>
          )}
        </label>
        <div className={styles.deckBarActions}>
          <Button size="sm" variant="print" icon={<Plus size={15} />} disabled={busy} onClick={() => void onCreateNew()}>
            {t('career.decks.new')}
          </Button>
          <Button size="sm" variant="print" icon={<Crown size={15} />} disabled={busy} onClick={() => void onCreateNew(COMMANDER_FORMAT)}>
            {t('career.decks.newCommander')}
          </Button>
          <Button size="sm" variant="quiet" icon={<Copy size={15} />} disabled={busy} onClick={() => void onDuplicate()}>
            {t('career.decks.duplicate')}
          </Button>
          <Button
            size="sm"
            variant="quiet"
            icon={<Trash2 size={15} />}
            disabled={busy || decks.length <= 1 || !activeId}
            onClick={() => setConfirmDelete(true)}
          >
            {t('career.decks.delete')}
          </Button>
        </div>
      </div>
      <div className={styles.deckBody}>
        <DeckBuilder key={activeId ?? 'career'} initial={deck} mode={mode} />
      </div>
      <Dialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={t('career.decks.deleteTitle', { name: activeSaved?.name ?? deck.name ?? t('career.nav.deck') })}
        description={t('career.decks.deleteDetail')}
        width="sm"
        footer={(
          <>
            <Button variant="quiet" disabled={busy} onClick={() => setConfirmDelete(false)}>{t('career.decks.cancel')}</Button>
            <Button variant="danger" busy={busy} onClick={() => void onDelete()}>{t('career.decks.delete')}</Button>
          </>
        )}
      >
        <p className={styles.small}>{t('career.deck.summary', { count: mainDeckSize(deck) })}</p>
      </Dialog>
    </div>
  );
}

/** The binder: every card the Career owns, one tile per card name. */
export function OwnedBinder({ format, identity, counts, limitOf, onAdd, onRemove, onPreview, onInspect }: BinderProps) {
  const t = useT();
  const collection = useCareerCollection(true);
  const commanderMode = isCommanderFormat(format);
  // names and types by default, as this binder always searched
  const [filters, setFilters] = useState<CardFilters>({ ...EMPTY_FILTERS, anyText: true });
  const [limitIdentity, setLimitIdentity] = useState(true);
  const [commandersOnly, setCommandersOnly] = useState(false);
  const sets = useQuery({ queryKey: ['expansionSets'], queryFn: () => api.getExpansionSets(), staleTime: Infinity });
  // one printing per name, as deck entries
  const entries = useMemo<DeckCardInfo[]>(() => {
    const seen = new Map<string, DeckCardInfo>();
    for (const card of collection.data ?? []) {
      const key = (card.name ?? '').toLowerCase();
      if (key && !seen.has(key)) seen.set(key, { cardName: card.name!, setCode: card.setCode, cardNumber: card.cardNumber, amount: 1 });
    }
    return [...seen.values()].sort((a, b) => a.cardName.localeCompare(b.cardName));
  }, [collection.data]);
  const info = useCardInfo(entries);
  const cards = useMemo<CardView[]>(() => entries.map((entry) => info.get(entryKey(entry))
    ?? { name: entry.cardName, expansionSetCode: entry.setCode ?? undefined, cardNumber: entry.cardNumber ?? undefined }), [entries, info]);
  const shown = useMemo(() => cards.filter((card) => {
    if (!matchesFilters(card, filters)) return false;
    if (commanderMode && commandersOnly && !canBeCommander(card)) return false;
    if (commanderMode && limitIdentity && identity !== null && !cardFitsIdentity(card, identity)) return false;
    return true;
  }), [cards, filters, commanderMode, commandersOnly, limitIdentity, identity]);
  // only the sets something owned comes from, newest first
  const setOptions = useMemo(() => {
    const owned = new Set(cards.map((card) => card.expansionSetCode));
    return (sets.data ?? []).filter((item) => item.setCode && owned.has(item.setCode)).sort((a, b) => (b.releaseDate ?? 0) - (a.releaseDate ?? 0));
  }, [cards, sets.data]);

  return (
    <section className={builder.collection} aria-label={t('career.nav.collection')}>
      <CardFilterBar filters={filters} onChange={setFilters} sets={setOptions} />
      {commanderMode && (
        <div className={styles.commanderTools} role="group" aria-label={t('career.deck.commanderTools')}>
          <button
            type="button"
            className={[styles.cmdrChip, commandersOnly ? styles.cmdrChipOn : ''].join(' ')}
            aria-pressed={commandersOnly}
            onClick={() => setCommandersOnly((on) => !on)}
          >
            <Crown size={14} aria-hidden="true" />
            {t('career.deck.pickCommander')}
          </button>
          {identity !== null && (
            <button
              type="button"
              className={[styles.cmdrChip, limitIdentity ? styles.cmdrChipOn : ''].join(' ')}
              aria-pressed={limitIdentity}
              onClick={() => setLimitIdentity((on) => !on)}
            >
              <Shield size={14} aria-hidden="true" />
              {t('career.deck.inIdentity')}
              {identity.length > 0 && (
                <span className={styles.cmdrPips}>
                  {identity.map((letter) => <i key={letter} className={`ms ms-cost ms-${letter.toLowerCase()}`} aria-hidden="true" />)}
                </span>
              )}
            </button>
          )}
        </div>
      )}
      <div className={builder.results}>
        {collection.isPending ? (
          <p className={builder.note}>{t('career.loading')}</p>
        ) : shown.length === 0 ? (
          <p className={builder.note}>{t('career.collection.empty')}</p>
        ) : (
          <div className={builder.grid}>
            {shown.map((card) => (
              <BinderCard
                key={(card.name ?? '').toLowerCase()}
                card={card}
                count={counts.get((card.name ?? '').toLowerCase()) ?? 0}
                limit={limitOf(card)}
                onAdd={onAdd}
                onRemove={onRemove}
                onPreview={onPreview}
                onInspect={onInspect}
              />
            ))}
          </div>
        )}
      </div>
    </section>
  );
}

const BinderCard = memo(function BinderCard({ card, count, limit, onAdd, onRemove, onPreview, onInspect }: {
  card: CardView;
  count: number;
  limit: number;
  onAdd(card: CardView): void;
  onRemove(card: CardView): void;
  onPreview(card: CardView | null, anchor?: DOMRect): void;
  onInspect(card: CardView): void;
}) {
  const t = useT();
  return (
    <div
      className={builder.tileWrap}
      onPointerEnter={(event) => onPreview(card, event.currentTarget.getBoundingClientRect())}
      onPointerLeave={() => onPreview(null)}
    >
      <button
        type="button"
        className={[builder.tile, count >= limit ? builder.tileFull : ''].join(' ')}
        onClick={() => onAdd(card)}
        onContextMenu={(event) => {
          event.preventDefault();
          if (count > 0) onRemove(card);
        }}
        aria-label={`${card.name}, ${count} / ${limit}`}
      >
        <CardFace card={card} size="normal" />
        <span className={builder.copies} aria-hidden="true">
          {Number.isFinite(limit) && limit <= 4
            ? Array.from({ length: limit }, (_, i) => <i key={i} className={i < count ? builder.copyOn : builder.copyOff} />)
            : <b>×{count}</b>}
        </span>
      </button>
      <InspectButton card={card} label={t('career.deck.inspect', { name: card.name ?? '' })} onInspect={onInspect} />
    </div>
  );
});
