import { useQuery } from '@tanstack/react-query';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import type { CareerCard, CareerSetProgress } from '../../protocol/generated/views';
import { api } from '../connection';
import { useT } from '../i18n';
import { useSession } from '../stores/session';
import { IconButton } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { useCareerSetProgress } from './careerData';
import { PLAYSET } from './careerModel';
import { ProgressBar } from './CareerScreen';
import { binderCards, binderPages, collectionNames, filterMissingPlayset, newInBinder, playsetProgress, readBinderSeen, writeBinderSeen, type BinderCard } from './binderModel';
import { dealStyle } from './motion';
import { fraction } from './progressModel';
import motion from './motion.module.css';
import styles from './Binder.module.css';

/**
 * The collection as a binder: a shelf of sets with how complete each is, and the chosen set's pages two at a time.
 * Owned cards sit in their pockets, the rest are outlines to craft; cards new since the last look shine.
 */
export function CareerBinder({ collection, owned, onPick }: {
  collection: readonly CareerCard[];
  owned: ReadonlyMap<string, number>;
  onPick(card: BinderCard): void;
}) {
  const t = useT();
  const user = useSession((session) => session.userName);
  const sets = useCareerSetProgress(true);
  const [chosen, setChosen] = useState<string | null>(null);
  const shelf = useMemo(() => sets.data ?? [], [sets.data]);
  const set = shelf.find((item) => item.setCode === chosen) ?? shelf.find((item) => (item.owned ?? 0) > 0) ?? shelf[0];

  // what was in the binder before this visit; what came since shines
  const [before] = useState(() => readBinderSeen(user));
  const names = useMemo(() => collectionNames(collection), [collection]);
  const fresh = useMemo(() => newInBinder(before, names).fresh, [before, names]);
  useEffect(() => {
    if (names.length > 0) writeBinderSeen(user, newInBinder(readBinderSeen(user), names).seen);
  }, [user, names]);

  if (sets.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!set) return <p className={styles.note}>{t('career.sets.none')}</p>;
  return (
    <div className={styles.binder}>
      <nav className={styles.shelf} aria-label={t('career.sets')}>
        <ul>
          {shelf.map((item) => (
            <li key={item.setCode}>
              <ShelfSet set={item} on={item.setCode === set.setCode} onPick={() => setChosen(item.setCode ?? null)} />
            </li>
          ))}
        </ul>
      </nav>
      <SetPages key={set.setCode} set={set} owned={owned} fresh={fresh} onPick={onPick} />
    </div>
  );
}

function ShelfSet({ set, on, onPick }: { set: CareerSetProgress; on: boolean; onPick(): void }) {
  const complete = !!set.total && set.owned === set.total;
  return (
    <button type="button" className={[styles.spine, on ? styles.spineOn : '', complete ? styles.spineDone : ''].join(' ')} aria-pressed={on} onClick={onPick} data-nav>
      <span className={styles.spineCode} aria-hidden="true">{set.setCode}</span>
      <span className={styles.spineText}>
        <b>{set.name}</b>
        <small>{set.owned ?? 0} / {set.total ?? 0}</small>
      </span>
      <ProgressBar value={fraction(set.owned, set.total)} label={set.name ?? set.setCode ?? ''} done={complete} />
    </button>
  );
}

function SetPages({ set, owned, fresh, onPick }: {
  set: CareerSetProgress;
  owned: ReadonlyMap<string, number>;
  fresh: ReadonlySet<string>;
  onPick(card: BinderCard): void;
}) {
  const t = useT();
  const code = set.setCode ?? '';
  const found = useQuery({
    queryKey: ['career', 'binder', code],
    queryFn: () => api.searchCards({ setCodes: [code], start: 0, count: 1000 }),
    enabled: !!code,
    staleTime: Infinity,
  });
  const cards = useMemo(() => binderCards(found.data ?? []), [found.data]);
  const [missingPlayset, setMissingPlayset] = useState(false);
  const shown = useMemo(
    () => (missingPlayset ? filterMissingPlayset(cards, owned) : cards),
    [cards, owned, missingPlayset],
  );
  const pages = useMemo(() => binderPages(shown), [shown]);
  const [spread, setSpread] = useState(0);
  const spreads = Math.max(1, Math.ceil(pages.length / 2));
  const safeSpread = Math.min(spread, spreads - 1);
  const have = useMemo(() => cards.filter((card) => (owned.get(card.name.toLowerCase()) ?? 0) > 0).length, [cards, owned]);
  const { playsets } = useMemo(() => playsetProgress(cards, owned), [cards, owned]);

  // Page Up and Page Down turn the pages (the arrow keys move between pockets)
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'PageDown') setSpread((current) => Math.min(current + 1, spreads - 1));
      else if (event.key === 'PageUp') setSpread((current) => Math.max(current - 1, 0));
      else return;
      event.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [spreads]);

  const left = pages[safeSpread * 2] ?? [];
  const right = pages[safeSpread * 2 + 1] ?? [];
  return (
    <section className={styles.book} aria-labelledby="binder-set">
      <header className={styles.bookHead}>
        <h2 id="binder-set">{set.name}</h2>
        <div className={styles.bookMeta}>
          <span className={styles.complete}>{t('career.binder.complete', { have, total: cards.length })}</span>
          <span className={styles.playsets}>{t('career.binder.playsets', { have: playsets, total: cards.length })}</span>
          <label className={styles.toggle}>
            <input
              type="checkbox"
              checked={missingPlayset}
              onChange={(event) => {
                setMissingPlayset(event.target.checked);
                setSpread(0);
              }}
            />
            {t('career.collection.missingPlayset')}
          </label>
        </div>
        <ProgressBar value={fraction(have, cards.length)} label={set.name ?? code} done={cards.length > 0 && have === cards.length} />
      </header>
      {found.isPending ? <p className={styles.note}>{t('career.loading')}</p> : found.isError ? <p className={styles.note} role="alert">{t('career.failed')}</p> : shown.length === 0 && missingPlayset ? (
        <p className={styles.note}>{t('career.binder.playsetComplete')}</p>
      ) : (
        <div className={[styles.spread, motion.scene].join(' ')} key={`${missingPlayset ? 'm' : 'a'}-${safeSpread}`}>
          <Page cards={left} owned={owned} fresh={fresh} onPick={onPick} first={0} />
          {right.length > 0 && <Page cards={right} owned={owned} fresh={fresh} onPick={onPick} first={left.length} />}
        </div>
      )}
      <footer className={styles.turn}>
        <IconButton label={t('career.binder.previous')} icon={<ChevronLeft size={20} />} disabled={safeSpread === 0} onClick={() => setSpread(safeSpread - 1)} />
        <span>{t('career.binder.page', { page: safeSpread + 1, pages: spreads })}</span>
        <IconButton label={t('career.binder.next')} icon={<ChevronRight size={20} />} disabled={safeSpread >= spreads - 1} onClick={() => setSpread(safeSpread + 1)} />
      </footer>
    </section>
  );
}

function Page({ cards, owned, fresh, onPick, first }: {
  cards: readonly BinderCard[];
  owned: ReadonlyMap<string, number>;
  fresh: ReadonlySet<string>;
  onPick(card: BinderCard): void;
  first: number;
}) {
  const t = useT();
  return (
    <ul className={styles.page}>
      {cards.map((card, index) => {
        const count = owned.get(card.name.toLowerCase()) ?? 0;
        const isNew = count > 0 && fresh.has(card.name.toLowerCase());
        const label = [card.name, `#${card.cardNumber}`, count > 0 ? t('career.owned', { count, max: PLAYSET }) : t('career.binder.missing'), isNew ? t('career.binder.new') : null].filter(Boolean).join(', ');
        return (
          <li key={card.name} className={motion.deal} style={dealStyle(first + index)}>
            <button type="button" className={[styles.pocket, count > 0 ? '' : styles.empty, isNew ? styles.fresh : '', styles[card.rarity] ?? ''].join(' ')} onClick={() => onPick(card)} aria-label={label} data-nav>
              {count > 0 ? (
                <>
                  <CardFace card={card} size="normal" />
                  <span className={styles.pips} aria-hidden="true">
                    {Array.from({ length: PLAYSET }, (_, i) => <i key={i} className={i < count ? styles.pipOn : undefined} />)}
                  </span>
                  {isNew && <span className={styles.newTag} aria-hidden="true">{t('career.binder.new')}</span>}
                </>
              ) : (
                <span className={styles.outline} aria-hidden="true">
                  <small>#{card.cardNumber}</small>
                  <b>{card.name}</b>
                </span>
              )}
            </button>
          </li>
        );
      })}
    </ul>
  );
}
