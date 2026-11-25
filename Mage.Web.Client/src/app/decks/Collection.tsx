import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
import { Images } from 'lucide-react';
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { CardCriteria, CardView } from '../../protocol/generated/views';
import { api } from '../connection';
import { CardFace } from '../ui/CardFace';
import { CardFilterBar } from './CardFilterBar';
import { InspectButton } from './CardInspect';
import { useCardInfoStore } from './cardInfo';
import { entryKey, printingOf } from './deckModel';
import { CARD_MIME } from './deckDrag';
import styles from './DeckBuilder.module.css';
import {
  effectiveSetCodes, EMPTY_FILTERS, intersectIdentity, onlyColorsIdentity, onlyInSidelinedSets, type CardFilters,
} from './filters';
import { PrintingPicker } from './PrintingPicker';

const PAGE = 60;

/** Sets most players don't build with: joke sets, unofficial sets, and Arena-only digital cards. */
const SIDELINED_SET_TYPES = new Set(['JOKE_SET', 'CUSTOM_SET', 'MAGIC_ARENA']);
const EMPTY: string[] = [];
const EMPTY_CARDS: CardView[] = [];

function toCriteria(filters: CardFilters, format: string, start: number, ignoreSets: string[], setCodes: string[] | null): CardCriteria {
  const text = filters.text.trim();
  return {
    ignoreSetCodes: ignoreSets,
    nameContains: text && !filters.anyText ? text : undefined,
    searchText: text && filters.anyText ? text : undefined,
    searchNames: true,
    searchTypes: true,
    searchRules: true,
    webColors: filters.colors,
    colorMatch: 'any',
    types: filters.type ? [filters.type] : [],
    setCodes: setCodes ?? undefined,
    rarities: filters.rarity ? [filters.rarity] : [],
    manaValue: filters.manaValue ?? undefined,
    manaValueOperator: filters.manaValue === 7 ? 'gte' : filters.manaValue !== null ? 'eq' : undefined,
    format: format || undefined,
    uniqueNames: true,
    sortBy: 'name',
    start,
    count: PAGE,
  };
}

function useDebounced<T>(value: T, ms: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), ms);
    return () => clearTimeout(timer);
  }, [value, ms]);
  return debounced;
}

/** Every card the format allows, filterable; click a card to add it. */
export function Collection({ format, identity, counts, limitOf, onAdd, onRemove, onPreview, onInspect }: {
  format: string;
  /** a commander deck's color identity (WUBRG letters): only cards it allows are shown; null shows every color */
  identity: string[] | null;
  /** copies already in the deck, by card name */
  counts: ReadonlyMap<string, number>;
  limitOf(card: CardView): number;
  onAdd(card: CardView): void;
  onRemove(card: CardView): void;
  onPreview(card: CardView | null, anchor?: DOMRect): void;
  onInspect(card: CardView): void;
}) {
  const [filters, setFilters] = useState<CardFilters>(EMPTY_FILTERS);
  const [allSets, setAllSets] = useState(false);
  // a commander deck shows what its commander allows, unless the player asks for everything
  const [anyIdentity, setAnyIdentity] = useState(false);
  // printings chosen in the binder before adding to the deck (#478), by lowercase card name
  const [chosenPrintings, setChosenPrintings] = useState<ReadonlyMap<string, CardView>>(() => new Map());
  const [printingFor, setPrintingFor] = useState<CardView | null>(null);
  const onChoosePrinting = useCallback((card: CardView) => {
    onPreview(null);
    setPrintingFor(card);
  }, [onPreview]);
  const query = useDebounced(filters, 250);
  // "only these colors" is an identity limit too, on top of the commander's (both run on the server, Java and Rust alike)
  const allowed = intersectIdentity(identity && !anyIdentity ? identity : null, onlyColorsIdentity(query.colors, query.onlyColors));
  const colorIdentity = allowed === null ? null : allowed.join('');
  const remember = useCardInfoStore((state) => state.remember);
  const sets = useQuery({ queryKey: ['expansionSets'], queryFn: () => api.getExpansionSets(), staleTime: Infinity });
  const sidelinedSets = useMemo(
    () => (sets.data ?? []).filter((set) => SIDELINED_SET_TYPES.has(set.type ?? '')).map((set) => set.setCode!).filter(Boolean),
    [sets.data],
  );
  // newest sets first; the sidelined kinds only when they are shown
  const setOptions = useMemo(
    () => [...(sets.data ?? [])]
      .filter((item) => item.setCode && (allSets || !SIDELINED_SET_TYPES.has(item.type ?? '')))
      .sort((a, b) => (b.releaseDate ?? 0) - (a.releaseDate ?? 0)),
    [sets.data, allSets],
  );
  const ignoreSets = allSets ? EMPTY : sidelinedSets;
  const matchedSetCodes = useMemo(() => effectiveSetCodes(query, setOptions), [query, setOptions]);
  const noSetsMatch = matchedSetCodes !== null && matchedSetCodes.length === 0;
  const results = useInfiniteQuery({
    queryKey: ['collection', query, format, ignoreSets, matchedSetCodes, colorIdentity],
    queryFn: ({ pageParam }) => api.searchCards(toCriteria(query, format, pageParam, ignoreSets, matchedSetCodes), colorIdentity === null ? null : { colorIdentity }),
    // wait for the set list so the first page already leaves the sidelined sets out (and era filters resolve)
    enabled: !noSetsMatch && ((allSets && matchedSetCodes === null) || !sets.isPending),
    initialPageParam: 0,
    getNextPageParam: (last, pages) => (last.length < PAGE ? undefined : pages.length * PAGE),
    staleTime: 5 * 60_000,
  });
  const cards = useMemo(() => (noSetsMatch ? EMPTY_CARDS : (results.data?.pages ?? []).flat().filter(Boolean)), [noSetsMatch, results.data]);
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = results;

  // a name typed still finds the cards printed only in the sets left out (Arena-only Alchemy cards, #392), after the rest
  const searchSidelined = !allSets && query.text.trim() !== '' && matchedSetCodes === null && sidelinedSets.length > 0;
  const sidelined = useQuery({
    queryKey: ['collection-sidelined', query, format, sidelinedSets, colorIdentity],
    queryFn: () => api.searchCards({ ...toCriteria(query, format, 0, [], sidelinedSets) }, colorIdentity === null ? null : { colorIdentity }),
    enabled: searchSidelined,
    staleTime: 5 * 60_000,
  });
  const extras = useMemo(
    () => (searchSidelined && !hasNextPage ? onlyInSidelinedSets(cards, (sidelined.data ?? []).filter(Boolean)) : EMPTY_CARDS),
    [searchSidelined, hasNextPage, cards, sidelined.data],
  );
  useEffect(() => remember(cards), [cards, remember]);
  useEffect(() => remember(extras), [extras, remember]);

  const tile = (card: CardView) => {
    const key = (card.name ?? '').toLowerCase();
    const effective = chosenPrintings.get(key) ?? card;
    return (
      <CollectionCard
        key={`${card.expansionSetCode}:${card.cardNumber}`}
        card={effective}
        count={counts.get(key) ?? 0}
        limit={limitOf(effective)}
        onAdd={onAdd}
        onRemove={onRemove}
        onPreview={onPreview}
        onInspect={onInspect}
        onChoosePrinting={onChoosePrinting}
      />
    );
  };

  const sentinel = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const element = sentinel.current;
    if (!element) return;
    const observer = new IntersectionObserver((items) => {
      if (items.some((item) => item.isIntersecting) && hasNextPage && !isFetchingNextPage) void fetchNextPage();
    }, { rootMargin: '600px' });
    observer.observe(element);
    return () => observer.disconnect();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  return (
    <section className={styles.collection} aria-label="Collection">
      <CardFilterBar
        filters={filters}
        onChange={setFilters}
        sets={setOptions}
        toggles={(
          <>
            {identity && (
              <label className={styles.toggle} title="Only cards your commander's color identity allows">
                <input type="checkbox" checked={!anyIdentity} onChange={(event) => setAnyIdentity(!event.target.checked)} />
                Commander&rsquo;s colors
                {identity.length === 0
                  ? <i className="ms ms-cost ms-c" aria-label="colorless" />
                  : identity.map((letter) => <i key={letter} className={`ms ms-cost ms-${letter.toLowerCase()}`} aria-hidden="true" />)}
              </label>
            )}
            <label className={styles.toggle} title="Joke sets, unofficial sets and Arena-only cards">
              <input type="checkbox" checked={allSets} onChange={(event) => setAllSets(event.target.checked)} />
              Joke &amp; digital sets
            </label>
          </>
        )}
      />

      <div className={styles.results}>
        {results.isPending ? (
          <p className={styles.note}>Opening the binder…</p>
        ) : results.isError ? (
          <p className={styles.note}>The card search failed. {results.error instanceof Error ? results.error.message : ''}</p>
        ) : cards.length === 0 && extras.length === 0 ? (
          <p className={styles.note}>
            {searchSidelined && sidelined.isPending ? 'Opening the binder…' : 'No cards match. Loosen a filter or check the spelling.'}
          </p>
        ) : (
          <>
            {cards.length > 0 && <div className={styles.grid}>{cards.map(tile)}</div>}
            {extras.length > 0 && (
              <>
                <p className={styles.note}>Only printed in joke &amp; digital sets:</p>
                <div className={styles.grid}>{extras.map(tile)}</div>
              </>
            )}
          </>
        )}
        <div ref={sentinel} className={styles.sentinel} aria-hidden="true" />
        {isFetchingNextPage && <p className={styles.note}>More cards…</p>}
      </div>
      <PrintingPicker
        name={printingFor?.name ?? null}
        current={printingFor ? entryKey(printingOf(printingFor)) : null}
        description="The card uses this printing in the binder and when you add it to your deck."
        onClose={() => setPrintingFor(null)}
        onPick={() => undefined}
        onPickCard={(picked) => {
          if (!picked.name) return;
          setChosenPrintings((prev) => {
            const next = new Map(prev);
            next.set(picked.name!.toLowerCase(), picked);
            return next;
          });
        }}
      />
    </section>
  );
}

const CollectionCard = memo(function CollectionCard({ card, count, limit, onAdd, onRemove, onPreview, onInspect, onChoosePrinting }: {
  card: CardView;
  count: number;
  limit: number;
  onAdd(card: CardView): void;
  onRemove(card: CardView): void;
  onPreview(card: CardView | null, anchor?: DOMRect): void;
  onInspect(card: CardView): void;
  onChoosePrinting(card: CardView): void;
}) {
  const full = count >= limit;
  return (
    <div
      className={styles.tileWrap}
      onPointerEnter={(event) => onPreview(card, event.currentTarget.getBoundingClientRect())}
      onPointerLeave={() => onPreview(null)}
    >
      <button
        type="button"
        className={[styles.tile, full ? styles.tileFull : ''].join(' ')}
        draggable
        onDragStart={(event) => {
          // dropped on the deck, a zone tab or a category, the card is added there
          const printing = printingOf(card);
          event.dataTransfer.setData(CARD_MIME, JSON.stringify({ zone: 'binder', key: entryKey(printing), name: card.name ?? '', printing }));
          event.dataTransfer.setData('text/plain', card.name ?? '');
          event.dataTransfer.effectAllowed = 'copy';
          onPreview(null);
        }}
        onClick={() => onAdd(card)}
        onContextMenu={(event) => {
          event.preventDefault();
          if (count > 0) onRemove(card);
        }}
        aria-label={`${card.name}${count > 0 ? `, ${count} in deck` : ''}. Click to add, right-click to remove.`}
      >
        <CardFace card={card} size="normal" />
        {count > 0 && (
          <span className={styles.copies} aria-hidden="true">
            {Number.isFinite(limit) && limit <= 4
              ? Array.from({ length: limit }, (_, i) => <i key={i} className={i < count ? styles.copyOn : styles.copyOff} />)
              : <b>×{count}</b>}
          </span>
        )}
      </button>
      <div className={styles.tileActions}>
        <InspectButton card={card} label={`Look at ${card.name ?? 'the card'} up close`} onInspect={onInspect} />
        <button
          type="button"
          className={styles.tileAction}
          aria-label={`Choose the printing of ${card.name ?? 'the card'}`}
          title="Choose printing"
          onClick={() => onChoosePrinting(card)}
        >
          <Images size={16} aria-hidden="true" />
        </button>
      </div>
    </div>
  );
});
