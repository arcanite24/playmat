import { Heart, Lock, PawPrint, Shuffle, Stamp } from 'lucide-react';
import { useEffect, useMemo, useState, type CSSProperties, type ReactNode } from 'react';
import type { CareerAchievement, CareerCatalogItem, CareerLevelTrack } from '../../protocol/generated/views';
import { registerMessages, useI18n, useT, type MessageKey } from '../i18n';
import messages from '../i18n/en/career';
import { clothOf, clothSwatch } from '../match/playmats';
import { useServerInfo } from '../queries';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import { useThankYou, type ThankYou } from '../support/supportData';
import { CardFace } from '../ui/CardFace';
import { catalogName, catalogOf, rarityOf, RARITY_COLORS, useCatalog, type CatalogKind } from '../ui/cosmeticsCatalog';
import { Art } from './Art';
import { useCareerAchievements, useCareerLevels, useCareerLook, type CareerLook } from './careerData';
import { CareerAvatar } from './CareerScreen';
import { ShowcaseBadges, TitleText } from './LookParts';
import { PlateFrame } from './PlateFrame';
import { PatronPlaymat, PatronTitle, PetShelf, SupporterMarks, SupporterSleeveItem } from './SupporterLocker';
import { howToEarn, itemKey, LOCKER_TABS, markSeen, ownsItem, readSeen, tabOf, unseen, type LockerTab } from './wardrobe';
import progress from './Progress.module.css';
import styles from './Locker.module.css';

registerMessages(messages);

/** What every tile needs to know: whose locker, what they own, how to say where a locked piece comes from. */
interface Shelf {
  user: string;
  look: CareerLook;
  owns(item: CareerCatalogItem): boolean;
  how(item: CareerCatalogItem): string;
  /** unlocked since the player last looked */
  fresh: ReadonlySet<string>;
  locale: string;
}

const EMPTY_LOOK: CareerLook = { avatar: null, title: null };

/**
 * The locker (B302): every cosmetic a player can wear, in every mode (the look goes to the server, so opponents see
 * it), one tab a kind. Everything progression gives comes from the cosmetics catalog: owned pieces are picked here
 * (sleeves are picked per deck), locked ones show dimmed with where they come from, and pieces unlocked since the
 * last visit carry a dot. The supporters' thank-you set has its own tab. Career's Cosmetics tab and Settings,
 * Cosmetics both show it; on a server without Career only the thank-you set shows.
 */
export function CosmeticsLocker({ track, achievements, career = true }: {
  track: CareerLevelTrack | null | undefined;
  achievements: readonly CareerAchievement[];
  /** whether the server offers Career: without it, only the thank-you set shows */
  career?: boolean;
}) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  const user = useSession((session) => session.userName);
  const unlocks = useMemo(() => track?.unlocks ?? [], [track?.unlocks]);
  const look = useCareerLook((state) => state.looks[user.toLowerCase()]) ?? EMPTY_LOOK;
  const catalog = useCatalog();
  const thanks = useThankYou();
  const tabs = career ? LOCKER_TABS : (['thanks'] as LockerTab[]);
  const [tab, setTab] = useState<LockerTab>(tabs[0]);
  // the dots stay for this visit; a tab loses its dot once opened
  const [seenAtOpen] = useState(() => readSeen(user));
  const [opened, setOpened] = useState<ReadonlySet<LockerTab>>(() => new Set([tabs[0]]));
  const fresh = useMemo(() => unseen(catalog, unlocks, seenAtOpen), [catalog, unlocks, seenAtOpen]);
  const freshKeys = useMemo(() => new Set(fresh.map(itemKey)), [fresh]);

  useEffect(() => {
    markSeen(user, fresh.filter((item) => tabOf(item.kind) === tab).map(itemKey));
  }, [user, tab, fresh]);
  const open = (id: LockerTab) => {
    setTab(id);
    setOpened((before) => (before.has(id) ? before : new Set([...before, id])));
  };

  const shelf: Shelf = {
    user,
    look,
    owns: (item) => ownsItem(unlocks, item),
    how: (item) => howToEarn(t, item, achievements),
    fresh: freshKeys,
    locale,
  };

  return (
    <div className={styles.locker}>
      <Mirror user={user} look={look} thanks={thanks} />
      {tabs.length > 1 && (
        <div className={styles.tabs} role="tablist" aria-label={t('career.locker.tabs')}>
          {tabs.map((id) => {
            const dot = !opened.has(id) && fresh.some((item) => tabOf(item.kind) === id);
            const name = t(`career.locker.tab.${id}` as MessageKey);
            return (
              <button
                key={id}
                type="button"
                role="tab"
                id={`locker-tab-${id}`}
                aria-selected={tab === id}
                aria-controls="locker-panel"
                className={styles.tab}
                onClick={() => open(id)}
                data-nav
              >
                {id === 'thanks' && <Heart size={13} aria-hidden="true" fill="currentColor" />}
                {name}
                {dot && <span className={styles.dot} role="img" aria-label={t('career.locker.newIn', { section: name })} />}
              </button>
            );
          })}
        </div>
      )}
      <div id="locker-panel" role="tabpanel" aria-labelledby={tabs.length > 1 ? `locker-tab-${tab}` : undefined} className={styles.panel}>
        {tab === 'avatar' && <AvatarShelf shelf={shelf} catalog={catalog} />}
        {tab === 'title' && <TitleShelf shelf={shelf} catalog={catalog} thanks={thanks} />}
        {tab === 'frame' && <FrameShelf shelf={shelf} catalog={catalog} thanks={thanks} />}
        {tab === 'sleeve' && <SleeveShelf shelf={shelf} catalog={catalog} />}
        {tab === 'playmat' && <PlaymatShelf shelf={shelf} catalog={catalog} />}
        {tab === 'pet' && <CompanionShelf shelf={shelf} catalog={catalog} thanks={thanks} />}
        {tab === 'emote' && <EmoteShelf shelf={shelf} catalog={catalog} />}
        {tab === 'battlefield' && <BattlefieldShelf shelf={shelf} catalog={catalog} />}
        {tab === 'land' && <LandShelf shelf={shelf} catalog={catalog} />}
        {tab === 'thanks' && <ThanksShelf thanks={thanks} />}
      </div>
    </div>
  );
}

/** The player as others see them: avatar in its frame, name, title and badges. */
function Mirror({ user, look, thanks }: { user: string; look: CareerLook; thanks: ThankYou }) {
  const t = useT();
  const patron = thanks.has('title') && thanks.wear.title;
  const ring = thanks.has('frame') && thanks.wear.frame;
  return (
    <div className={styles.mirror}>
      <span className={ring ? styles.ring : styles.disc}>
        <PlateFrame frame={ring ? null : look.frame}><CareerAvatar id={look.avatar} name={user} size={56} /></PlateFrame>
      </span>
      <span className={styles.mirrorName}>
        <b>{user}</b>
        {patron ? <small className={styles.patron}>{t('support.locker.patron')}</small> : look.title && <TitleText as="small" id={look.title} />}
      </span>
      <ShowcaseBadges badges={look.badges} />
    </div>
  );
}

function choose(shelf: Shelf, patch: Partial<CareerLook>) {
  useCareerLook.getState().choose(shelf.user, patch);
}

/** One piece in a shelf: its picture, name and rarity; locked ones dimmed with where they come from. */
function Tile({ shelf, item, on, onPick, children, wide = false }: {
  shelf: Shelf;
  item: CareerCatalogItem;
  on?: boolean;
  /** absent for a piece that isn't picked here (sleeves, emote sets) */
  onPick?(): void;
  children: ReactNode;
  wide?: boolean;
}) {
  const t = useT();
  const owned = shelf.owns(item);
  const rarity = rarityOf(item);
  const how = owned ? null : shelf.how(item);
  const body = (
    <>
      <span className={styles.preview}>{children}</span>
      <span className={styles.name}>{catalogName(item, shelf.locale)}</span>
      <span className={styles.rarity}>{t(`career.rarity.${rarity}` as MessageKey)}</span>
      {how && <small className={styles.how}><Lock size={11} aria-hidden="true" /> {how}</small>}
      {shelf.fresh.has(itemKey(item)) && <span className={styles.dot} role="img" aria-label={t('career.locker.new')} />}
    </>
  );
  const className = [styles.tile, owned ? '' : styles.locked, wide ? styles.wide : ''].join(' ');
  const style = { '--rarity': RARITY_COLORS[rarity] } as CSSProperties;
  if (!onPick) return <div className={className} style={style} title={how ?? undefined}>{body}</div>;
  return (
    <button type="button" role="radio" aria-checked={!!on} disabled={!owned} className={className} style={style} title={how ?? undefined} onClick={onPick} data-nav>
      {body}
    </button>
  );
}

/** "None" or "Default" at the head of a shelf. */
function PlainTile({ on, label, onPick, children }: { on: boolean; label: string; onPick(): void; children: ReactNode }) {
  return (
    <button type="button" role="radio" aria-checked={on} className={styles.tile} onClick={onPick} data-nav>
      <span className={styles.preview}>{children}</span>
      <span className={styles.name}>{label}</span>
    </button>
  );
}

function ShelfPanel({ id, title, note, children }: { id: string; title?: string; note?: string; children: ReactNode }) {
  return (
    <section className={styles.shelf} aria-labelledby={title ? `locker-${id}` : undefined}>
      {title && <h3 id={`locker-${id}`}>{title}</h3>}
      {note && <p className={progress.panelNote}>{note}</p>}
      {children}
    </section>
  );
}

/** Owned first, then the rest, each in catalog order. */
function shelfOrder(shelf: Shelf, items: readonly CareerCatalogItem[]): CareerCatalogItem[] {
  return [...items.filter((item) => shelf.owns(item)), ...items.filter((item) => !shelf.owns(item))];
}

interface ShelfProps {
  shelf: Shelf;
  catalog: readonly CareerCatalogItem[];
}

function AvatarShelf({ shelf, catalog }: ShelfProps) {
  const t = useT();
  return (
    <ShelfPanel id="avatar">
      <div className={styles.tiles} role="radiogroup" aria-label={t('career.look.avatar')}>
        <PlainTile on={!shelf.look.avatar} label={t('career.look.none')} onPick={() => choose(shelf, { avatar: null })}>
          <CareerAvatar id={null} name={shelf.user} size={60} />
        </PlainTile>
        {shelfOrder(shelf, catalogOf('avatar', catalog)).map((item) => (
          <Tile key={item.id} shelf={shelf} item={item} on={shelf.look.avatar === item.id} onPick={() => choose(shelf, { avatar: item.id! })}>
            <CareerAvatar id={item.id} name={shelf.user} size={60} />
          </Tile>
        ))}
      </div>
    </ShelfPanel>
  );
}

function TitleShelf({ shelf, catalog, thanks }: ShelfProps & { thanks: ThankYou }) {
  const t = useT();
  // a supporter's Patron title is worn instead of the Career one: picking one here takes it off
  const patron = thanks.has('title') && thanks.wear.title;
  const pick = (id: string | null) => {
    choose(shelf, { title: id });
    if (patron) thanks.choose({ title: false });
  };
  return (
    <ShelfPanel id="title">
      <div className={styles.titles} role="radiogroup" aria-label={t('career.look.title')}>
        <button type="button" role="radio" aria-checked={!patron && !shelf.look.title} className={styles.titleTile} onClick={() => pick(null)} data-nav>
          <span className={styles.titleName}>{t('career.look.none')}</span>
        </button>
        {shelfOrder(shelf, catalogOf('title', catalog)).map((item) => {
          const owned = shelf.owns(item);
          const rarity = rarityOf(item);
          const how = owned ? null : shelf.how(item);
          return (
            <button
              key={item.id}
              type="button"
              role="radio"
              aria-checked={!patron && shelf.look.title === item.id}
              disabled={!owned}
              className={[styles.titleTile, owned ? '' : styles.locked].join(' ')}
              style={{ '--rarity': RARITY_COLORS[rarity] } as CSSProperties}
              title={how ?? undefined}
              onClick={() => pick(item.id!)}
              data-nav
            >
              <span className={styles.titleName}>{catalogName(item, shelf.locale)}</span>
              <span className={styles.rarity}>{t(`career.rarity.${rarity}` as MessageKey)}</span>
              {how && <small className={styles.how}><Lock size={11} aria-hidden="true" /> {how}</small>}
              {shelf.fresh.has(itemKey(item)) && <span className={styles.dot} role="img" aria-label={t('career.locker.new')} />}
            </button>
          );
        })}
      </div>
    </ShelfPanel>
  );
}

function FrameShelf({ shelf, catalog, thanks }: ShelfProps & { thanks: ThankYou }) {
  const t = useT();
  // the supporter ring, when worn, wins over an earned frame: picking one here takes the ring off
  const ring = thanks.has('frame') && thanks.wear.frame;
  const pick = (id: string | null) => {
    choose(shelf, { frame: id });
    if (ring && id) thanks.choose({ frame: false });
  };
  const avatar = <CareerAvatar id={shelf.look.avatar} name={shelf.user} size={52} />;
  return (
    <ShelfPanel id="frame" note={t('career.locker.frameNote')}>
      <div className={styles.tiles} role="radiogroup" aria-label={t('career.locker.tab.frame')}>
        <PlainTile on={!ring && !shelf.look.frame} label={t('career.locker.noFrame')} onPick={() => pick(null)}>{avatar}</PlainTile>
        {shelfOrder(shelf, catalogOf('frame', catalog)).map((item) => (
          <Tile key={item.id} shelf={shelf} item={item} on={!ring && shelf.look.frame === item.id} onPick={() => pick(item.id!)}>
            <PlateFrame frame={item.id}>{avatar}</PlateFrame>
          </Tile>
        ))}
      </div>
    </ShelfPanel>
  );
}

function SleeveShelf({ shelf, catalog }: ShelfProps) {
  const t = useT();
  return (
    <ShelfPanel id="sleeve" note={t('career.look.sleevesNote')}>
      <p className={styles.random}><Shuffle size={14} aria-hidden="true" /> {t('career.locker.randomNote')}</p>
      <div className={styles.tiles}>
        {shelfOrder(shelf, catalogOf('sleeve', catalog)).map((item) => (
          <Tile key={item.id} shelf={shelf} item={item}>
            <span className={styles.sleeve}><CardFace card={{ name: '' }} hidden sleeve={item.color} size="normal" /></span>
          </Tile>
        ))}
      </div>
    </ShelfPanel>
  );
}

function PlaymatShelf({ shelf, catalog }: ShelfProps) {
  const t = useT();
  const cloth = useSettings((settings) => settings.settings.matCloth);
  return (
    <ShelfPanel id="playmat">
      <div className={styles.tiles} role="radiogroup" aria-label={t('career.look.playmats')}>
        {shelfOrder(shelf, catalogOf('playmat', catalog)).map((item) => (
          <Tile key={item.id} shelf={shelf} item={item} on={cloth === item.id} onPick={() => useSettings.getState().update({ matCloth: item.id! })}>
            <span className={progress.cloth} style={clothSwatch(clothOf(item.id))}><Art kind="playmat" id={item.id!} size={56} className={progress.clothPrint} /></span>
          </Tile>
        ))}
      </div>
    </ShelfPanel>
  );
}

function CompanionShelf({ shelf, catalog, thanks }: ShelfProps & { thanks: ThankYou }) {
  const t = useT();
  // the party corgi, when worn, wins over an earned companion: picking one here sends the corgi home
  const corgi = thanks.has('pet') && thanks.wear.pet;
  const pick = (id: string | null) => {
    choose(shelf, { pet: id });
    if (corgi && id) thanks.choose({ pet: false });
  };
  return (
    <ShelfPanel id="pet" note={t('career.locker.petNote')}>
      <div className={styles.tiles} role="radiogroup" aria-label={t('career.locker.tab.pet')}>
        <PlainTile on={!corgi && !shelf.look.pet} label={t('career.locker.noPet')} onPick={() => pick(null)}>
          <span className={progress.noPet} aria-hidden="true" />
        </PlainTile>
        {shelfOrder(shelf, catalogOf('pet', catalog)).map((item) => (
          <Tile key={item.id} shelf={shelf} item={item} on={!corgi && shelf.look.pet === item.id} onPick={() => pick(item.id!)}>
            <Art kind="companion" id={item.id!} size={64} fallback={<PawPrint size={36} aria-hidden="true" />} />
          </Tile>
        ))}
      </div>
    </ShelfPanel>
  );
}

function EmoteShelf({ shelf, catalog }: ShelfProps) {
  const t = useT();
  return (
    <ShelfPanel id="emote" note={t('career.locker.emoteNote')}>
      <div className={styles.sets}>
        {shelfOrder(shelf, catalogOf('emote', catalog)).map((item) => (
          <Tile key={item.id} shelf={shelf} item={item} wide>
            <span className={styles.stickers}>
              {(item.stickers ?? []).map((sticker) => {
                const phrase = (shelf.locale.startsWith('es') ? sticker.es : undefined) ?? sticker.phrase ?? sticker.id ?? '';
                return (
                  <span key={sticker.id} className={styles.sticker}>
                    {/* the free phrases are words only; the earned sets are painted */}
                    {!item.free && <Art kind="sticker" id={sticker.id!} size={44} fallback={<span className={styles.stickerBlank} aria-hidden="true" />} />}
                    <small>{phrase}</small>
                  </span>
                );
              })}
            </span>
          </Tile>
        ))}
      </div>
    </ShelfPanel>
  );
}

/** An attack arrow in its colour, as it crosses the battlefield. */
function ArrowSample({ color }: { color?: string }) {
  const paint = color ?? 'var(--ink-soft)';
  return (
    <svg className={styles.sample} viewBox="0 0 64 40" aria-hidden="true">
      <path d="M6 34C18 10 38 6 54 10" fill="none" stroke={paint} strokeWidth="4" strokeLinecap="round" />
      <path d="M46 3l12 7-11 9" fill="none" stroke={paint} strokeWidth="4" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

const SPECKS = [[12, 8], [52, 10], [14, 33], [50, 32]];

/** A hit spark in its colour: a burst of streaks and specks. */
function SparkSample({ color }: { color?: string }) {
  const paint = color ?? 'var(--ink-soft)';
  return (
    <svg className={styles.sample} viewBox="0 0 64 40" aria-hidden="true">
      {Array.from({ length: 8 }, (_, index) => {
        const angle = (index / 8) * Math.PI * 2;
        return <line key={index} x1={32 + 6 * Math.cos(angle)} y1={20 + 6 * Math.sin(angle)} x2={32 + 16 * Math.cos(angle)} y2={20 + 16 * Math.sin(angle)} stroke={paint} strokeWidth="2.5" strokeLinecap="round" />;
      })}
      {SPECKS.map(([x, y]) => <circle key={`${x}-${y}`} cx={x} cy={y} r="2" fill={paint} />)}
    </svg>
  );
}

type BattlefieldKind = CatalogKind & ('arrow' | 'spark' | 'stamp');

const BATTLEFIELD: { kind: BattlefieldKind; title: MessageKey }[] = [
  { kind: 'arrow', title: 'career.locker.arrows' },
  { kind: 'spark', title: 'career.locker.sparks' },
  { kind: 'stamp', title: 'career.locker.stamps' },
];

function BattlefieldSample({ kind, item }: { kind: BattlefieldKind; item?: CareerCatalogItem }) {
  if (kind === 'arrow') return <ArrowSample color={item?.color} />;
  if (kind === 'spark') return <SparkSample color={item?.color} />;
  const drawn = <Stamp size={32} aria-hidden="true" />;
  return item?.id ? <Art kind="stamp" id={item.id} size={52} fallback={drawn} /> : drawn;
}

function BattlefieldShelf({ shelf, catalog }: ShelfProps) {
  const t = useT();
  return (
    <>
      <p className={progress.panelNote}>{t('career.locker.battlefieldNote')}</p>
      {BATTLEFIELD.map(({ kind, title }) => (
        <ShelfPanel key={kind} id={kind} title={t(title)}>
          <div className={styles.tiles} role="radiogroup" aria-labelledby={`locker-${kind}`}>
            <PlainTile on={!shelf.look[kind]} label={t('career.locker.default')} onPick={() => choose(shelf, { [kind]: null })}>
              <BattlefieldSample kind={kind} />
            </PlainTile>
            {shelfOrder(shelf, catalogOf(kind, catalog)).map((item) => (
              <Tile key={item.id} shelf={shelf} item={item} on={shelf.look[kind] === item.id} onPick={() => choose(shelf, { [kind]: item.id })}>
                <BattlefieldSample kind={kind} item={item} />
              </Tile>
            ))}
          </div>
        </ShelfPanel>
      ))}
    </>
  );
}

/** A set's symbol, from Scryfall (as the shop's boosters show it), or its code when there is none. */
function SetMark({ set }: { set?: string }) {
  const [failed, setFailed] = useState(false);
  if (!set || failed) return <b className={styles.setCode}>{set ?? '—'}</b>;
  return <img className={styles.setSymbol} src={`https://svgs.scryfall.io/sets/${set.toLowerCase()}.svg`} alt="" decoding="async" loading="lazy" onError={() => setFailed(true)} />;
}

function LandShelf({ shelf, catalog }: ShelfProps) {
  const t = useT();
  return (
    <ShelfPanel id="land" note={t('career.locker.landNote')}>
      <div className={styles.tiles} role="radiogroup" aria-label={t('career.locker.tab.land')}>
        <PlainTile on={!shelf.look.lands} label={t('career.locker.default')} onPick={() => choose(shelf, { lands: null })}>
          <SetMark />
        </PlainTile>
        {shelfOrder(shelf, catalogOf('land', catalog)).map((item) => (
          <Tile key={item.id} shelf={shelf} item={item} on={shelf.look.lands === item.id} onPick={() => choose(shelf, { lands: item.id })}>
            <SetMark set={item.set} />
          </Tile>
        ))}
      </div>
    </ShelfPanel>
  );
}

/** The supporters' thank-you set, kept apart from what Career gives. */
function ThanksShelf({ thanks }: { thanks: ThankYou }) {
  const t = useT();
  const patron = thanks.has('title') && thanks.wear.title;
  return (
    <>
      <p className={progress.panelNote}>{t('career.locker.thanksNote')}</p>
      {thanks.shows('title') && (
        <section className={progress.shelfPanel} aria-labelledby="locker-patron">
          <h3 id="locker-patron">{t('career.look.title')}</h3>
          <div className={progress.titles}>
            <PatronTitle thanks={thanks} on={patron} />
          </div>
        </section>
      )}
      <PetShelf thanks={thanks} />
      <SupporterMarks thanks={thanks} />
      {(thanks.shows('sleeve') || thanks.shows('playmat')) && (
        <section className={progress.shelfPanel} aria-labelledby="locker-thanks-items">
          <h3 id="locker-thanks-items">{t('career.look.sleeves')} · {t('career.look.playmats')}</h3>
          <div className={progress.options}>
            <SupporterSleeveItem thanks={thanks} />
            <PatronPlaymat thanks={thanks} />
          </div>
        </section>
      )}
    </>
  );
}

/** Settings, Cosmetics: the locker outside Career, with the Career track when the player has one. */
export default function CosmeticsSettings() {
  const t = useT();
  const career = useServerInfo().data?.career === true;
  const optedIn = useSettings((settings) => settings.settings.careerOptIn);
  const levels = useCareerLevels(career && optedIn);
  const achievements = useCareerAchievements(career && optedIn);
  if (levels.isLoading) return <p className={progress.panelNote}>{t('career.look.loading')}</p>;
  return (
    <div className={progress.settingsLocker}>
      <p className={progress.panelNote}>{t(career ? 'career.look.everywhere' : 'career.look.everywhereNoCareer')}</p>
      <CosmeticsLocker track={levels.data} achievements={achievements.data ?? []} career={career} />
    </div>
  );
}
