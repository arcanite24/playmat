import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
import { memo, useEffect, useMemo, useRef, useState } from 'react';
import type { CardCriteria, CardView } from '../../protocol/generated/views';
import { api } from '../connection';
import { CardFace } from '../ui/CardFace';
import { CardFilterBar } from './CardFilterBar';
import { useCardInfoStore } from './cardInfo';
import styles from './DeckBuilder.module.css';
import { EMPTY_FILTERS, type CardFilters } from './filters';

const PAGE = 60;

/** Sets most players don't build with: joke sets, unofficial sets, and Arena-only digital cards. */
const SIDELINED_SET_TYPES = new Set(['JOKE_SET', 'CUSTOM_SET', 'MAGIC_ARENA']);

function toCriteria(filters: CardFilters, format: string, start: number, ignoreSets: string[]): CardCriteria {
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
    setCodes: filters.set ? [filters.set] : undefined,
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
export function Collection({ format, identity, counts, limitOf, onAdd, onRemove, onPreview }: {
  format: string;
  /** a commander deck's color identity (WUBRG letters): only cards it allows are shown; null shows every color */
  identity: string[] | null;
  /** copies already in the deck, by card name */
  counts: ReadonlyMap<string, number>;
  limitOf(card: CardView): number;
  onAdd(card: CardView): void;
  onRemove(card: CardView): void;
  onPreview(card: CardView | null, anchor?: DOMRect): void;
}) {
  const [filters, setFilters] = useState<CardFilters>(EMPTY_FILTERS);
  const [allSets, setAllSets] = useState(false);
  // a commander deck shows what its commander allows, unless the player asks for everything
  const [anyIdentity, setAnyIdentity] = useState(false);
  const colorIdentity = identity && !anyIdentity ? identity.join('') : null;
  const query = useDebounced(filters, 250);
  const remember = useCardInfoStore((state) => state.remember);
  const sets = useQuery({ queryKey: ['expansionSets'], queryFn: () => api.getExpansionSets(), staleTime: Infinity });
  const ignoreSets = useMemo(
    () => (allSets ? [] : (sets.data ?? []).filter((set) => SIDELINED_SET_TYPES.has(set.type ?? '')).map((set) => set.setCode!).filter(Boolean)),
    [allSets, sets.data],
  );
  const results = useInfiniteQuery({
    queryKey: ['collection', query, format, ignoreSets, colorIdentity],
    queryFn: ({ pageParam }) => api.searchCards(toCriteria(query, format, pageParam, ignoreSets), colorIdentity === null ? null : { colorIdentity }),
    // wait for the set list so the first page already leaves the sidelined sets out
    enabled: allSets || !sets.isPending,
    initialPageParam: 0,
    getNextPageParam: (last, pages) => (last.length < PAGE ? undefined : pages.length * PAGE),
    staleTime: 5 * 60_000,
  });
  const cards = useMemo(() => (results.data?.pages ?? []).flat().filter(Boolean), [results.data]);
  useEffect(() => remember(cards), [cards, remember]);

  const sentinel = useRef<HTMLDivElement>(null);
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = results;
  useEffect(() => {
    const element = sentinel.current;
    if (!element) return;
    const observer = new IntersectionObserver((items) => {
      if (items.some((item) => item.isIntersecting) && hasNextPage && !isFetchingNextPage) void fetchNextPage();
    }, { rootMargin: '600px' });
    observer.observe(element);
    return () => observer.disconnect();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  // newest sets first; the sidelined kinds only when they are shown
  const setOptions = useMemo(
    () => [...(sets.data ?? [])]
      .filter((item) => item.setCode && (allSets || !SIDELINED_SET_TYPES.has(item.type ?? '')))
      .sort((a, b) => (b.releaseDate ?? 0) - (a.releaseDate ?? 0)),
    [sets.data, allSets],
  );

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
        ) : cards.length === 0 ? (
          <p className={styles.note}>No cards match. Loosen a filter or check the spelling.</p>
        ) : (
          <div className={styles.grid}>
            {cards.map((card) => (
              <CollectionCard
                key={`${card.expansionSetCode}:${card.cardNumber}`}
                card={card}
                count={counts.get((card.name ?? '').toLowerCase()) ?? 0}
                limit={limitOf(card)}
                onAdd={onAdd}
                onRemove={onRemove}
                onPreview={onPreview}
              />
            ))}
          </div>
        )}
        <div ref={sentinel} className={styles.sentinel} aria-hidden="true" />
        {isFetchingNextPage && <p className={styles.note}>More cards…</p>}
      </div>
    </section>
  );
}

const CollectionCard = memo(function CollectionCard({ card, count, limit, onAdd, onRemove, onPreview }: {
  card: CardView;
  count: number;
  limit: number;
  onAdd(card: CardView): void;
  onRemove(card: CardView): void;
  onPreview(card: CardView | null, anchor?: DOMRect): void;
}) {
  const full = count >= limit;
  return (
    <button
      type="button"
      className={[styles.tile, full ? styles.tileFull : ''].join(' ')}
      onClick={() => onAdd(card)}
      onContextMenu={(event) => {
        event.preventDefault();
        if (count > 0) onRemove(card);
      }}
      onPointerEnter={(event) => onPreview(card, event.currentTarget.getBoundingClientRect())}
      onPointerLeave={() => onPreview(null)}
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
  );
});
