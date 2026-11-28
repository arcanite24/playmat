import { useQuery } from '@tanstack/react-query';
import { Library, Search } from 'lucide-react';
import { memo, useEffect, useMemo, useState } from 'react';
import { Navigate, useSearchParams } from 'react-router-dom';
import type { CareerProfile } from '../../protocol/generated/views';
import { api } from '../connection';
import { registerMessages, useT, type MessageKey } from '../i18n';
import messages from '../i18n/en/career';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { Dialog } from '../ui/Dialog';
import { CareerBinder } from './CareerBinder';
import { collectionPlaysetCount, isMissingPlayset, type BinderCard } from './binderModel';
import { craftCard, useCareerCollection, useCareerSetProgress, useCareerState } from './careerData';
import { ProgressBar } from './CareerScreen';
import { fraction } from './progressModel';
import { PLAYSET, canAfford, careerRarity, craftCost, isBasic, ownedByName, type CareerRarity } from './careerModel';
import styles from './Career.module.css';

registerMessages(messages);

type Printing = BinderCard;

const RARITIES: { value: CareerRarity | ''; label: MessageKey }[] = [
  { value: '', label: 'career.rarity.any' },
  { value: 'common', label: 'career.rarity.common' },
  { value: 'uncommon', label: 'career.rarity.uncommon' },
  { value: 'rare', label: 'career.rarity.rare' },
  { value: 'mythic', label: 'career.rarity.mythic' },
];

function useDebounced<T>(value: T, ms: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), ms);
    return () => clearTimeout(timer);
  }, [value, ms]);
  return debounced;
}

/** The cards the Career owns, and crafting: any card, for coins or a wildcard. */
export function CareerCollectionScreen() {
  const t = useT();
  const state = useCareerState();
  const profile = state.data?.profile;
  const collection = useCareerCollection(!!profile);
  const [text, setText] = useState('');
  const [rarity, setRarity] = useState<CareerRarity | ''>('');
  const [everything, setEverything] = useState(false);
  const [missingPlayset, setMissingPlayset] = useState(false);
  const [crafting, setCrafting] = useState<Printing | null>(null);
  const [showSets, setShowSets] = useState(false);
  const [params, setParams] = useSearchParams();
  const view = params.get('view') === 'search' ? 'search' : 'binder';
  const owned = useMemo(() => ownedByName(collection.data ?? []), [collection.data]);
  const playsetCount = useMemo(() => collectionPlaysetCount(owned), [owned]);
  const needle = text.trim().toLowerCase();
  const query = useDebounced(needle, 250);

  const mine = useMemo<Printing[]>(() => (collection.data ?? [])
    .map((card) => ({ name: card.name ?? '', setCode: card.setCode ?? '', cardNumber: card.cardNumber ?? '', rarity: (card.rarity ?? 'common') as CareerRarity }))
    .filter((card) => (!needle || card.name.toLowerCase().includes(needle))
      && (!rarity || card.rarity === rarity)
      && (!missingPlayset || isMissingPlayset(card.name, owned)))
    .sort((a, b) => a.name.localeCompare(b.name)), [collection.data, needle, rarity, missingPlayset, owned]);

  // every card, to craft one not owned yet: searched on the server by name
  const search = useQuery({
    queryKey: ['career', 'craftSearch', query],
    queryFn: () => api.searchCards({ nameContains: query, searchNames: true, uniqueNames: true, sortBy: 'name', start: 0, count: 48 }),
    enabled: everything && query.length >= 2,
    staleTime: 5 * 60_000,
  });
  const found = useMemo<Printing[]>(() => (search.data ?? [])
    .map((card) => ({ name: card.name ?? '', setCode: card.expansionSetCode ?? '', cardNumber: card.cardNumber ?? '', rarity: careerRarity(card.rarity) }))
    .filter((card) => card.setCode
      && !isBasic(card.name)
      && (!rarity || card.rarity === rarity)
      && (!missingPlayset || isMissingPlayset(card.name, owned))), [search.data, rarity, missingPlayset, owned]);

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;
  const cards = everything ? found : mine;
  const total = (collection.data ?? []).reduce((sum, card) => sum + (card.count ?? 0), 0);

  return (
    <div className={styles.page}>
      <div className={styles.tabs} role="group" aria-label={t('career.nav.collection')}>
        <button type="button" className={styles.tab} aria-pressed={view === 'binder'} onClick={() => setParams({}, { replace: true })} data-nav>{t('career.binder')}</button>
        <button type="button" className={styles.tab} aria-pressed={view === 'search'} onClick={() => setParams({ view: 'search' }, { replace: true })} data-nav>{t('career.binder.search')}</button>
        <p className={styles.tabsNote}>
          {t('career.collection.count', { count: total })} · {t('career.collection.playsets', { count: playsetCount })}
        </p>
      </div>
      {view === 'binder' ? (
        <CareerBinder collection={collection.data ?? []} owned={owned} onPick={setCrafting} />
      ) : (
        <>
          <header className={styles.pageHead}>
            <div className={styles.filters}>
              <label className={styles.search}>
                <Search size={16} aria-hidden="true" />
                <input value={text} onChange={(event) => setText(event.target.value)} placeholder={t('career.collection.search')} aria-label={t('career.collection.search')} />
              </label>
              <select className={styles.select} value={rarity} onChange={(event) => setRarity(event.target.value as CareerRarity | '')} aria-label={t('career.rarity.any')}>
                {RARITIES.map((item) => <option key={item.value} value={item.value}>{t(item.label)}</option>)}
              </select>
              <label className={styles.toggle}>
                <input type="checkbox" checked={missingPlayset} onChange={(event) => setMissingPlayset(event.target.checked)} />
                {t('career.collection.missingPlayset')}
              </label>
              <label className={styles.toggle}>
                <input type="checkbox" checked={everything} onChange={(event) => setEverything(event.target.checked)} />
                {t('career.collection.all')}
              </label>
              <Button variant="quiet" size="sm" icon={<Library size={16} />} onClick={() => setShowSets(true)}>{t('career.sets')}</Button>
            </div>
          </header>
          <div className={styles.binder}>
            {everything && query.length < 2 ? (
              <p className={styles.note}>{t('career.collection.searchHint')}</p>
            ) : (everything ? search.isPending : collection.isPending) ? (
              <p className={styles.note}>{t('career.loading')}</p>
            ) : cards.length === 0 ? (
              <p className={styles.note}>{t('career.collection.empty')}</p>
            ) : (
              <ul className={styles.grid}>
                {cards.map((card) => (
                  <OwnedCard key={`${card.setCode}:${card.cardNumber}`} card={card} count={owned.get(card.name.toLowerCase()) ?? 0} onPick={setCrafting} />
                ))}
              </ul>
            )}
          </div>
        </>
      )}
      <SetProgressDialog open={showSets} onClose={() => setShowSets(false)} />
      <CraftDialog card={crafting} profile={profile} owned={crafting ? owned.get(crafting.name.toLowerCase()) ?? 0 : 0} onClose={() => setCrafting(null)} />
    </div>
  );
}

const OwnedCard = memo(function OwnedCard({ card, count, onPick }: { card: Printing; count: number; onPick(card: Printing): void }) {
  const t = useT();
  return (
    <li>
      <button type="button" className={[styles.tile, count === 0 ? styles.tileUnowned : ''].join(' ')} onClick={() => onPick(card)} aria-label={`${card.name}, ${t('career.owned', { count, max: PLAYSET })}`}>
        <CardFace card={{ name: card.name, setCode: card.setCode, cardNumber: card.cardNumber }} size="normal" />
        <span className={styles.pips} aria-hidden="true">
          {Array.from({ length: PLAYSET }, (_, i) => <i key={i} className={i < count ? styles.pipOn : styles.pipOff} />)}
        </span>
      </button>
    </li>
  );
});

function CraftDialog({ card, profile, owned, onClose }: { card: Printing | null; profile: CareerProfile; owned: number; onClose(): void }) {
  const t = useT();
  const [busy, setBusy] = useState(false);
  const cost = card ? craftCost(card.rarity, profile) : null;
  const full = owned >= PLAYSET;
  const affordable = canAfford(profile, cost);

  async function craft() {
    if (!card) return;
    setBusy(true);
    try {
      await craftCard(card.setCode, card.cardNumber);
      notify(t('career.craft.done', { name: card.name }), t('career.owned', { count: owned + 1, max: PLAYSET }));
    } catch (reason) {
      notify(t('career.craft.failed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog
      open={!!card}
      onOpenChange={(open) => !open && onClose()}
      title={card ? t('career.craft.title', { name: card.name }) : ''}
      width="sm"
      footer={cost && !full ? (
        <Button variant="decision" busy={busy} disabled={!affordable} onClick={() => void craft()}>{t('career.craft.confirm')}</Button>
      ) : undefined}
    >
      {card && (
        <div className={styles.craft}>
          <span className={styles.craftArt}><CardFace card={{ name: card.name, setCode: card.setCode, cardNumber: card.cardNumber }} size="normal" /></span>
          <div className={styles.craftText}>
            <p>{t('career.owned', { count: owned, max: PLAYSET })}</p>
            {!cost ? <p>{t('career.craft.basic')}</p>
              : full ? <p>{t('career.craft.full')}</p>
                : 'coins' in cost
                  ? <p>{t('career.craft.costCoins', { coins: cost.coins, have: profile.coins ?? 0 })}</p>
                  : <p>{t('career.craft.costWildcard', { rarity: t(`career.rarity.${cost.wildcard}`).toLowerCase(), have: profile.wildcards?.[cost.wildcard] ?? 0 })}</p>}
          </div>
        </div>
      )}
    </Dialog>
  );
}

/** How much of each set the collection has. */
function SetProgressDialog({ open, onClose }: { open: boolean; onClose(): void }) {
  const t = useT();
  const sets = useCareerSetProgress(open);
  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()} title={t('career.sets')} description={t('career.sets.lead')} width="md">
      {sets.isPending ? <p className={styles.note}>{t('career.loading')}</p>
        : !sets.data?.length ? <p className={styles.note}>{t('career.sets.none')}</p>
          : (
            <ul className={styles.sets}>
              {sets.data.map((set) => (
                <li key={set.setCode} className={styles.setRow}>
                  <span>
                    <b>{set.name}</b>
                    <small>{set.setCode}</small>
                  </span>
                  <ProgressBar value={fraction(set.owned, set.total)} label={set.name ?? set.setCode ?? ''} done={!!set.total && set.owned === set.total} />
                  <small>{set.owned ?? 0} / {set.total ?? 0}</small>
                </li>
              ))}
            </ul>
          )}
    </Dialog>
  );
}
