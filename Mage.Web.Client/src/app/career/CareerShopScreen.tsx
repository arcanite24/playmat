import { Check, Coins, Crown, Gift, Lock, Palette, Package, Search, Sparkles } from 'lucide-react';
import { useMemo, useState } from 'react';
import { Link, Navigate, useSearchParams } from 'react-router-dom';
import type { CareerCatalogItem, CareerShopSet, CareerStarter } from '../../protocol/generated/views';
import { formatNumber, registerMessages, useI18n, useT, type MessageKey } from '../i18n';
import messages from '../i18n/en/career';
import { clothOf, clothSwatch } from '../match/playmats';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { catalogName, RARITY_COLORS, useCatalog } from '../ui/cosmeticsCatalog';
import { Art } from './Art';
import { BoosterPack } from './BoosterPack';
import { acquirePrecon, buyCosmetic, buyPacks, useCareerLevels, useCareerSetProgress, useCareerShop, useCareerStarters, useCareerState, type OpenedPacks } from './careerData';
import { batchCost, batchHighlights, PACK_BATCHES, packsAffordable, PRECON_PRICE } from './careerModel';
import { useSceneArt } from './careerScene';
import { CareerAvatar, ProgressBar } from './CareerScreen';
import { TitleText } from './LookParts';
import { dealStyle } from './motion';
import motion from './motion.module.css';
import { PackReveal } from './PackReveal';
import { PlateFrame } from './PlateFrame';
import { fraction } from './progressModel';
import { ownsItem } from './wardrobe';
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

type ShopTab = 'packs' | 'precons' | 'cosmetics';
type PackCategory = 'all' | 'standard' | 'commander';
type BoutiqueFilter = 'all' | 'playmat' | 'sleeve' | 'frame' | 'avatar' | 'pet' | 'emote' | 'battlefield' | 'title';

const BOUTIQUE_FILTERS: { id: BoutiqueFilter; label: MessageKey }[] = [
  { id: 'all', label: 'career.shop.cosmeticsAll' },
  { id: 'playmat', label: 'career.locker.tab.playmat' },
  { id: 'sleeve', label: 'career.locker.tab.sleeve' },
  { id: 'frame', label: 'career.locker.tab.frame' },
  { id: 'avatar', label: 'career.locker.tab.avatar' },
  { id: 'pet', label: 'career.locker.tab.pet' },
  { id: 'emote', label: 'career.locker.tab.emote' },
  { id: 'battlefield', label: 'career.locker.tab.battlefield' },
  { id: 'title', label: 'career.locker.tab.title' },
];

function matchesBoutiqueFilter(item: CareerCatalogItem, filter: BoutiqueFilter): boolean {
  if (filter === 'all') return true;
  if (filter === 'frame') return item.kind === 'frame' || item.kind === 'cmdframe';
  if (filter === 'battlefield') return item.kind === 'arrow' || item.kind === 'spark' || item.kind === 'stamp';
  return item.kind === filter;
}

/**
 * The shop: booster packs on one tab, and the Coin Cosmetics Boutique on the other.
 */
export function CareerShopScreen() {
  const t = useT();
  const state = useCareerState();
  const profile = state.data?.profile;
  const [params, setParams] = useSearchParams();
  const rawTab = params.get('tab');
  const rawCat = params.get('cat');
  const tab: ShopTab = rawTab === 'cosmetics' ? 'cosmetics'
    : rawTab === 'precons' || rawCat === 'precons' ? 'precons' : 'packs';
  const initialCat: PackCategory = rawCat === 'commander' ? 'commander' : rawCat === 'standard' ? 'standard' : 'all';

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;

  return (
    <div className={shop.shell}>
      <div className={shop.tabs} role="tablist" aria-label={t('career.nav.shop')}>
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'packs'}
          className={[shop.tab, tab === 'packs' ? shop.tabOn : ''].join(' ')}
          onClick={() => setParams({}, { replace: true })}
          data-nav
        >
          <Package size={16} aria-hidden="true" />
          {t('career.shop.tabPacks')}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'precons'}
          className={[shop.tab, tab === 'precons' ? shop.tabOn : ''].join(' ')}
          onClick={() => setParams({ tab: 'precons' }, { replace: true })}
          data-nav
        >
          <Crown size={16} aria-hidden="true" />
          {t('career.shop.preconsTitle')}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'cosmetics'}
          className={[shop.tab, tab === 'cosmetics' ? shop.tabOn : ''].join(' ')}
          onClick={() => setParams({ tab: 'cosmetics' }, { replace: true })}
          data-nav
        >
          <Palette size={16} aria-hidden="true" />
          {t('career.shop.tabCosmetics')}
        </button>
      </div>
      {tab === 'packs'
        ? <PackShop coins={profile.coins ?? 0} tokens={profile.packTokens ?? 0} initialCat={initialCat} />
        : tab === 'precons'
          ? <PreconShop coins={profile.coins ?? 0} freeGift={!!state.data?.commanderPreconGift} ownedIds={state.data?.ownedPrecons ?? []} />
          : <CosmeticsBoutique coins={profile.coins ?? 0} />}
    </div>
  );
}

function PackShop({ coins, tokens, initialCat }: { coins: number; tokens: number; initialCat: PackCategory }) {
  const t = useT();
  const catalog = useCareerShop(true);
  const progress = useCareerSetProgress(true);
  const [query, setQuery] = useState('');
  const [cat, setCat] = useState<PackCategory>(initialCat);
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
    return (catalog.data ?? []).filter((set) => {
      if (cat === 'commander' && set.category !== 'commander' && set.category !== 'masters') return false;
      if (cat === 'standard' && (set.category === 'commander' || set.category === 'masters')) return false;
      return !needle || set.name?.toLowerCase().includes(needle) || set.setCode?.toLowerCase() === needle;
    });
  }, [catalog.data, query, cat]);
  const completion = useMemo(() => new Map((progress.data ?? []).map((set) => [set.setCode, set])), [progress.data]);
  const featured = sets.find((set) => set.setCode === chosen) ?? sets.find((set) => !set.locked) ?? sets[0];
  useSceneArt(featured?.cover);

  const price = featured?.price ?? catalog.data?.[0]?.price ?? 100;
  const short = tokens === 0 && coins < price;
  const affordable = packsAffordable(coins, tokens, price);
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
          <div className={shop.filters} role="group" aria-label={t('career.shop.packCategories')}>
            {([
              ['all', 'career.shop.catAll'],
              ['standard', 'career.shop.catExpansions'],
              ['commander', 'career.shop.commanderPacks'],
            ] as [PackCategory, MessageKey][]).map(([id, label]) => (
              <button
                key={id}
                type="button"
                aria-pressed={cat === id}
                className={[shop.filter, cat === id ? shop.filterOn : ''].join(' ')}
                onClick={() => setCat(id)}
              >
                {t(label)}
              </button>
            ))}
          </div>
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

function PreconShop({ coins, freeGift, ownedIds }: { coins: number; freeGift: boolean; ownedIds: readonly string[] }) {
  const t = useT();
  const starters = useCareerStarters(true);
  const [busy, setBusy] = useState<string | null>(null);
  const owned = useMemo(() => new Set(ownedIds), [ownedIds]);
  const precons = useMemo(
    () => (starters.data ?? []).filter((starter) => starter.track === 'commander' || (starter.cards ?? 60) === 100),
    [starters.data],
  );
  useSceneArt(precons[0]?.cover);

  async function acquire(starter: CareerStarter) {
    if (!starter.id || busy) return;
    setBusy(starter.id);
    try {
      const result = await acquirePrecon(starter.id);
      notify(
        result.free ? t('career.precon.claimedTitle') : t('career.precon.boughtTitle'),
        t('career.precon.claimedBody', { name: starter.name ?? '' }),
      );
    } catch (reason) {
      notify(t('career.shop.failed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className={shop.boutique}>
      <header className={shop.boutiqueHead}>
        <div>
          <h2 className={shop.setName}>{t('career.shop.preconsTitle')}</h2>
          <p className={styles.small}>
            {freeGift ? t('career.shop.preconsGiftLead') : t('career.shop.preconsLead', { price: formatNumber(PRECON_PRICE) })}
          </p>
        </div>
        <Link to="/career/deck" className={shop.lockerLink}>{t('career.deck.edit')}</Link>
      </header>
      {starters.isPending && <p className={styles.note}>{t('career.loading')}</p>}
      <ul className={shop.boutiqueGrid}>
        {precons.map((starter, index) => {
          const have = !!starter.id && owned.has(starter.id);
          const canBuy = freeGift || coins >= PRECON_PRICE;
          const colors = (starter.colors ?? '').match(/[WUBRG]/g) ?? [];
          return (
            <li key={starter.id} className={[shop.boutiqueCard, have ? shop.boutiqueOwned : '', motion.deal].join(' ')} style={dealStyle(index)}>
              <div className={shop.boutiqueArt}>
                <CardFace
                  card={{ name: starter.cover?.name ?? starter.name, setCode: starter.cover?.setCode, cardNumber: starter.cover?.cardNumber }}
                  size="normal"
                />
              </div>
              <div className={shop.boutiqueBody}>
                <p className={shop.boutiqueKind}>
                  {colors.map((letter) => <i key={letter} className={`ms ms-cost ms-${letter.toLowerCase()}`} aria-hidden="true" />)}
                  {' '}
                  {t('career.starter.cards', { count: starter.cards ?? 100 })}
                </p>
                <b className={shop.boutiqueName}>{starter.name}</b>
                <p className={shop.boutiqueBlurb}>{starter.cover?.name}</p>
              </div>
              <div className={shop.boutiqueFoot}>
                {have ? (
                  <span className={shop.ownedTag}><Check size={14} aria-hidden="true" /> {t('career.shop.cosmeticsOwned')}</span>
                ) : (
                  <Button
                    variant={freeGift ? 'decision' : canBuy ? 'decision' : 'quiet'}
                    size="sm"
                    icon={freeGift ? <Gift size={14} /> : undefined}
                    busy={busy === starter.id}
                    disabled={!canBuy || busy !== null}
                    onClick={() => void acquire(starter)}
                  >
                    {freeGift ? t('career.shop.claimFree') : <><Coins size={14} aria-hidden="true" /> {formatNumber(PRECON_PRICE)}</>}
                  </Button>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/** UTC ISO week-based year and week number, matching Java CareerCosmetics.featuredKeys and Rust. */
function isoWeekNumber(now = new Date()): number {
  const date = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate()));
  const day = date.getUTCDay() || 7;
  date.setUTCDate(date.getUTCDate() + 4 - day);
  const yearStart = new Date(Date.UTC(date.getUTCFullYear(), 0, 1));
  const week = Math.ceil(((date.getTime() - yearStart.getTime()) / 86_400_000 + 1) / 7);
  return date.getUTCFullYear() * 53 + week;
}

/** Deterministic weekly Curator's Picks keys ("kind:id") discounted 20% this UTC ISO week. */
function featuredBoutiqueKeys(priced: readonly CareerCatalogItem[], now = new Date()): Set<string> {
  const picked = new Set<string>();
  const n = priced.length;
  if (n === 0) return picked;
  const weekNum = isoWeekNumber(now);
  const limit = Math.min(6, n);
  for (let i = 0; i < limit; i++) {
    let idx = (((weekNum * 7 + i * 5) % n) + n) % n;
    let guard = 0;
    while (guard < n) {
      const key = `${priced[idx].kind}:${priced[idx].id}`;
      if (!picked.has(key)) {
        picked.add(key);
        break;
      }
      idx = (idx + 1) % n;
      guard++;
    }
  }
  return picked;
}

function effectiveBoutiquePrice(item: CareerCatalogItem, featured: ReadonlySet<string>): number {
  const base = item.price ?? 0;
  if (base <= 0) return 0;
  return featured.has(`${item.kind}:${item.id}`) ? Math.floor((base * 4) / 5) : base;
}

function CosmeticsBoutique({ coins }: { coins: number }) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  const catalog = useCatalog();
  const levels = useCareerLevels(true);
  const unlocks = useMemo(() => levels.data?.unlocks ?? [], [levels.data?.unlocks]);
  const [filter, setFilter] = useState<BoutiqueFilter>('all');
  const [buying, setBuying] = useState<string | null>(null);

  const items = useMemo(
    () => catalog.filter((item) => (item.price ?? 0) > 0),
    [catalog],
  );
  const featured = useMemo(() => featuredBoutiqueKeys(items), [items]);
  const featuredItems = useMemo(
    () => items.filter((item) => featured.has(`${item.kind}:${item.id}`)),
    [items, featured],
  );
  const shown = useMemo(
    () => items.filter((item) => matchesBoutiqueFilter(item, filter)),
    [items, filter],
  );

  async function purchase(item: CareerCatalogItem) {
    if (!item.kind || !item.id) return;
    const key = `${item.kind}:${item.id}`;
    setBuying(key);
    try {
      await buyCosmetic(item.kind, item.id);
      notify(t('career.shop.cosmeticsBought', { name: catalogName(item, locale) }), '');
    } catch (reason) {
      notify(t('career.shop.cosmeticsFailed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBuying(null);
    }
  }

  const renderCard = (item: CareerCatalogItem, highlight = false) => {
    const key = `${item.kind}:${item.id}`;
    const owned = ownsItem(unlocks, item);
    const basePrice = item.price ?? 0;
    const price = effectiveBoutiquePrice(item, featured);
    const onSale = price < basePrice;
    const short = coins < price;
    const name = catalogName(item, locale);
    const rarity = (item.rarity ?? 'common') as keyof typeof RARITY_COLORS;
    return (
      <li key={highlight ? `feat-${key}` : key} className={[shop.boutiqueCard, owned ? shop.boutiqueOwned : '', highlight ? shop.boutiqueFeatured : ''].join(' ')}>
        {onSale && <span className={shop.saleTag}>{t('career.shop.saleBadge')}</span>}
        <div className={shop.boutiqueArt}>
          <BoutiquePreview item={item} locale={locale} />
        </div>
        <div className={shop.boutiqueInfo}>
          <span className={shop.boutiqueRarity} style={{ color: RARITY_COLORS[rarity] ?? undefined }}>
            {t(`career.rarity.${rarity}` as MessageKey)}
          </span>
          <b className={shop.boutiqueName}>{name}</b>
          {item.kind === 'emote' && item.stickers && (
            <small className={shop.boutiqueSub}>
              {item.stickers.slice(0, 3).map((s) => (locale.startsWith('es') && s.es ? s.es : s.phrase)).join(' · ')}
            </small>
          )}
        </div>
        <div className={shop.boutiqueFoot}>
          {owned ? (
            <Link to="/career/progress?tab=look" className={shop.ownedLink} data-nav>
              <Check size={14} aria-hidden="true" /> {t('career.shop.cosmeticsOwned')} · {t('career.shop.cosmeticsEquip')}
            </Link>
          ) : (
            <Button
              variant="decision"
              size="sm"
              icon={<Coins size={14} />}
              busy={buying === key}
              disabled={short || buying !== null}
              onClick={() => void purchase(item)}
              data-nav
            >
              {onSale ? (
                <>
                  <s className={shop.oldPrice}>{formatNumber(basePrice)}</s>{' '}
                  {t('career.shop.cosmeticsBuy', { price: formatNumber(price) })}
                </>
              ) : (
                t('career.shop.cosmeticsBuy', { price: formatNumber(price) })
              )}
            </Button>
          )}
        </div>
      </li>
    );
  };

  return (
    <section className={shop.boutique} aria-label={t('career.shop.tabCosmetics')}>
      {featuredItems.length > 0 && (
        <div className={shop.featuredWrap}>
          <header className={shop.featuredHead}>
            <h3><Sparkles size={15} aria-hidden="true" /> {t('career.shop.featuredTitle')}</h3>
            <p className={styles.note}>{t('career.shop.featuredNote')}</p>
          </header>
          <ul className={shop.boutiqueGrid}>
            {featuredItems.map((item) => renderCard(item, true))}
          </ul>
        </div>
      )}
      <header className={shop.boutiqueHead}>
        <p className={styles.note}>{t('career.shop.cosmeticsLead')}</p>
        <div className={shop.filters} role="tablist" aria-label={t('career.shop.tabCosmetics')}>
          {BOUTIQUE_FILTERS.map((entry) => (
            <button
              key={entry.id}
              type="button"
              role="tab"
              aria-selected={filter === entry.id}
              className={[shop.filter, filter === entry.id ? shop.filterOn : ''].join(' ')}
              onClick={() => setFilter(entry.id)}
              data-nav
            >
              {t(entry.label)}
            </button>
          ))}
        </div>
      </header>
      {shown.length === 0 ? (
        <p className={styles.note}>{t('career.shop.cosmeticsNone')}</p>
      ) : (
        <ul className={shop.boutiqueGrid}>
          {shown.map((item) => renderCard(item))}
        </ul>
      )}
    </section>
  );
}

function BoutiquePreview({ item, locale }: { item: CareerCatalogItem; locale: string }) {
  switch (item.kind) {
    case 'playmat': {
      const cloth = clothOf(item.id);
      return (
        <span
          className={shop.previewPlaymat}
          style={cloth ? clothSwatch(cloth) : { background: item.dye ?? '#2c4a3e' }}
          aria-hidden="true"
        />
      );
    }
    case 'sleeve':
      return <CardFace card={{ name: '' }} hidden sleeve={item.color ?? '#3b6ea5'} size="small" />;
    case 'frame':
      return (
        <PlateFrame frame={item.id}>
          <CareerAvatar id={null} name="P" size={52} />
        </PlateFrame>
      );
    case 'cmdframe':
      return (
        <span
          style={{
            display: 'grid',
            placeItems: 'center',
            width: 52,
            height: 70,
            borderRadius: 7,
            background: 'rgb(var(--void-rgb) / 0.6)',
            color: item.ring ?? '#e2c27a',
            boxShadow: `0 0 0 2px ${item.ring ?? '#e2c27a'}, 0 0 0 4px ${item.accent ?? '#9e7328'}`,
          }}
          aria-hidden="true"
        >
          <Crown size={22} />
        </span>
      );
    case 'avatar':
      return <CareerAvatar id={item.id} name={catalogName(item, locale)} size={64} />;
    case 'pet':
      return <Art kind="companion" id={item.id ?? ''} size={72} />;
    case 'title':
      return <TitleText as="span" id={item.id ?? ''} />;
    case 'arrow':
    case 'spark':
      return <span className={shop.previewSwatch} style={{ background: item.color ?? '#d9a441' }} aria-hidden="true" />;
    case 'emote':
      return (
        <span className={shop.previewQuote}>
          {item.stickers?.[0] ? (locale.startsWith('es') && item.stickers[0].es ? item.stickers[0].es : item.stickers[0].phrase) : item.name}
        </span>
      );
    default:
      return <Sparkles size={28} aria-hidden="true" />;
  }
}
