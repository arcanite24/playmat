import { Coins, Lock, Search, Sparkles } from 'lucide-react';
import { useMemo, useState } from 'react';
import { Navigate } from 'react-router-dom';
import type { CareerShopSet } from '../../protocol/generated/views';
import { formatNumber, registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { BoosterPack } from './BoosterPack';
import { buyPacks, useCareerSetProgress, useCareerShop, useCareerState, type OpenedPacks } from './careerData';
import { batchCost, batchHighlights, PACK_BATCHES, packsAffordable } from './careerModel';
import { useSceneArt } from './careerScene';
import { ProgressBar } from './CareerScreen';
import { dealStyle } from './motion';
import motion from './motion.module.css';
import { PackReveal } from './PackReveal';
import { fraction } from './progressModel';
import styles from './Career.module.css';
import shop from './Shop.module.css';

registerMessages(messages);

/** the batch size picked last, kept on this browser */
const BATCH_KEY = 'playmat.career.packBatch';

function savedBatch(): number {
  try {
    const value = Number(localStorage.getItem(BATCH_KEY));
    return (PACK_BATCHES as readonly number[]).includes(value) ? value : 1;
  } catch {
    return 1;
  }
}

/**
 * The shop: one set's pack on the counter, big enough to want, with what it costs and how much of the set you have;
 * every other set's pack on the shelf below. Buying tears the pack open right here; several packs bought at once open
 * together, showing what's worth turning.
 */
export function CareerShopScreen() {
  const t = useT();
  const state = useCareerState();
  const profile = state.data?.profile;
  const catalog = useCareerShop(!!profile);
  const progress = useCareerSetProgress(!!profile);
  const [query, setQuery] = useState('');
  const [chosen, setChosen] = useState<string | null>(null);
  const [buying, setBuying] = useState<string | null>(null);
  const [opened, setOpened] = useState<{ result: OpenedPacks; set: CareerShopSet; serial: number } | null>(null);
  const [batch, setBatch] = useState(savedBatch);
  const [opening, setOpening] = useState<{ done: number; count: number } | null>(null);
  const highlights = useMemo(() => {
    if (!opened || opened.result.packs < 2) return null;
    const list = batchHighlights(opened.result.cards ?? [], opened.result.fresh);
    return { cards: list.map((item) => item.card), copies: new Map(list.map((item) => [item.card, item.copies])) };
  }, [opened]);
  const sets = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return (catalog.data ?? []).filter((set) => !needle || set.name?.toLowerCase().includes(needle) || set.setCode?.toLowerCase() === needle);
  }, [catalog.data, query]);
  const completion = useMemo(() => new Map((progress.data ?? []).map((set) => [set.setCode, set])), [progress.data]);
  const featured = sets.find((set) => set.setCode === chosen) ?? sets.find((set) => !set.locked) ?? sets[0];
  useSceneArt(featured?.cover);

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;
  const coins = profile.coins ?? 0;
  const price = featured?.price ?? catalog.data?.[0]?.price ?? 100;
  // free packs (level rewards, the weekly goal) are spent before coins
  const tokens = profile.packTokens ?? 0;
  const short = tokens === 0 && coins < price;
  const affordable = packsAffordable(coins, tokens, price);
  // a batch bigger than the coins allow opens as many as they do
  const count = Math.max(1, Math.min(batch, affordable));
  const cost = batchCost(count, tokens, price);
  const free = Math.min(count, tokens);

  function pick(size: number) {
    setBatch(size);
    try {
      localStorage.setItem(BATCH_KEY, String(size));
    } catch {
      // remembered only on this page then
    }
  }

  async function buy(set: CareerShopSet, packs: number) {
    setBuying(set.setCode ?? null);
    setOpening(packs > 1 ? { done: 0, count: packs } : null);
    try {
      const result = await buyPacks(set.setCode!, packs, (done) => setOpening(packs > 1 ? { done, count: packs } : null));
      setOpened((current) => ({ result, set, serial: (current?.serial ?? 0) + 1 }));
    } catch (reason) {
      notify(t('career.shop.failed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBuying(null);
      setOpening(null);
    }
  }

  const have = completion.get(featured?.setCode);
  return (
    <div className={shop.shop}>
      {catalog.isPending && <p className={styles.note}>{t('career.loading')}</p>}
      {featured && (
        <section className={[shop.counter, motion.scene].join(' ')} aria-labelledby="shop-set" key={featured.setCode}>
          <div className={shop.display}>
            <button
              type="button"
              className={shop.featuredPack}
              disabled={featured.locked || short || buying !== null}
              onClick={() => void buy(featured, count)}
              aria-label={featured.locked ? t('career.shop.locked', { chapter: featured.unlockedBy ?? '' }) : t('career.shop.openOne', { set: featured.name ?? '' })}
            >
              <BoosterPack setCode={featured.setCode ?? ''} setName={featured.name ?? ''} cover={featured.cover} size="lg" tilt={!featured.locked} locked={featured.locked} />
            </button>
            <span className={shop.pedestal} aria-hidden="true" />
          </div>
          <div className={shop.pitch}>
            <p className={shop.kicker}>{[featured.setCode, featured.releaseDate?.slice(0, 4)].filter(Boolean).join(' · ')}</p>
            <h2 id="shop-set" className={shop.setName}>{featured.name}</h2>
            {have && (have.total ?? 0) > 0 && (
              <div className={shop.completion}>
                <ProgressBar value={fraction(have.owned, have.total)} label={t('career.shop.collected', { owned: have.owned ?? 0, total: have.total ?? 0 })} done={have.owned === have.total} />
                <span>{t('career.shop.collected', { owned: formatNumber(have.owned ?? 0), total: formatNumber(have.total ?? 0) })}</span>
              </div>
            )}
            <ul className={shop.facts}>
              <li><Sparkles size={16} aria-hidden="true" /> {t('career.shop.factRare')}</li>
              <li><Coins size={16} aria-hidden="true" /> {t('career.shop.factSpares')}</li>
            </ul>
            {featured.locked ? (
              <p className={shop.lockedNote}><Lock size={16} aria-hidden="true" /> {t('career.shop.locked', { chapter: featured.unlockedBy ?? '' })}</p>
            ) : (
              <div className={shop.buy}>
                {affordable > 1 && (
                  <div className={shop.batch} role="group" aria-label={t('career.shop.howMany')}>
                    {PACK_BATCHES.map((size) => (
                      <button
                        key={size}
                        type="button"
                        className={shop.batchOption}
                        aria-pressed={size === batch}
                        disabled={(size > affordable && size !== 1) || buying !== null}
                        onClick={() => pick(size)}
                        data-nav
                      >
                        ×{size}
                      </button>
                    ))}
                  </div>
                )}
                <span className={shop.price}>
                  {count === 1
                    ? tokens > 0 ? t('career.shop.tokens', { count: tokens }) : <><Coins size={20} aria-hidden="true" /> {formatNumber(price)}</>
                    : <>
                      {free > 0 && t('career.shop.batchFree', { count: free })}
                      {free > 0 && cost > 0 && ' + '}
                      {cost > 0 && <><Coins size={20} aria-hidden="true" /> {formatNumber(cost)}</>}
                    </>}
                </span>
                <Button
                  variant="decision"
                  size="xl"
                  busy={buying === featured.setCode}
                  disabled={short || buying !== null}
                  aria-describedby={short ? 'career-short' : undefined}
                  onClick={() => void buy(featured, count)}
                  data-nav
                >
                  {opening
                    ? t('career.shop.opening', { done: Math.min(opening.done + 1, opening.count), count: opening.count })
                    : count > 1
                      ? cost === 0 ? t('career.shop.openFreeMany', { count }) : t('career.shop.buyMany', { count })
                      : tokens > 0 ? t('career.shop.openFree') : t('career.shop.buyOpen')}
                </Button>
              </div>
            )}
            {!featured.locked && short && <p id="career-short" className={shop.short}>{t('career.shop.shortBy', { coins: formatNumber(price - coins) })}</p>}
          </div>
        </section>
      )}

      <section className={shop.shelf} aria-labelledby="shop-shelf">
        <header className={shop.shelfHead}>
          <h3 id="shop-shelf">{t('career.shop.shelf')}</h3>
          <label className={styles.search}>
            <Search size={16} aria-hidden="true" />
            <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t('career.shop.search')} aria-label={t('career.shop.search')} />
          </label>
        </header>
        {catalog.data && sets.length === 0 && <p className={styles.note}>{t('career.shop.none')}</p>}
        <ul className={shop.rack}>
          {sets.map((set, index) => {
            const progressOf = completion.get(set.setCode);
            const on = set.setCode === featured?.setCode;
            return (
              <li key={set.setCode} className={motion.deal} style={dealStyle(index)}>
                <button
                  type="button"
                  className={[shop.slot, on ? shop.slotOn : ''].join(' ')}
                  aria-pressed={on}
                  onClick={() => setChosen(set.setCode ?? null)}
                  data-nav
                >
                  <BoosterPack setCode={set.setCode ?? ''} setName={set.name ?? ''} cover={set.cover} size="sm" locked={set.locked} />
                  <span className={shop.slotName}>{set.name}</span>
                  <small>
                    {set.locked
                      ? t('career.shop.lockedShort')
                      : progressOf?.total ? t('career.shop.collected', { owned: progressOf.owned ?? 0, total: progressOf.total }) : set.releaseDate?.slice(0, 4)}
                  </small>
                </button>
              </li>
            );
          })}
        </ul>
      </section>

      {opened && (
        <PackReveal
          key={opened.serial}
          setName={opened.set.name ?? opened.result.setCode ?? ''}
          setCode={opened.result.setCode ?? opened.set.setCode ?? ''}
          cover={opened.set.cover}
          fresh={opened.result.fresh}
          cards={highlights?.cards ?? opened.result.cards ?? []}
          packs={opened.result.packs}
          copies={highlights?.copies}
          all={highlights ? opened.result.cards : undefined}
          stopped={opened.result.stopped}
          canBuyAnother={packsAffordable(opened.result.profile?.coins ?? 0, opened.result.profile?.packTokens ?? 0, opened.set.price ?? price) >= opened.result.packs && buying === null}
          onAnother={() => void buy(opened.set, opened.result.packs)}
          onClose={() => setOpened(null)}
        />
      )}
    </div>
  );
}
