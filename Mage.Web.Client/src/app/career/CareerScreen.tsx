import { Check, Coins, Gift, RefreshCw, Shuffle, Swords, Trophy, Upload } from 'lucide-react';
import { useEffect, useMemo, useRef, useState, type CSSProperties, type ChangeEvent } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import type { CareerAchievement, CareerCampaign, CareerOpponent, CareerProfile, CareerShopSet, CareerQuest, CareerQuests, CareerStarter, CareerWeekly } from '../../protocol/generated/views';
import { registerMessages, useT, type MessageKey } from '../i18n';
import messages from '../i18n/en/career';
import { usePlay } from '../stores/play';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { Dialog } from '../ui/Dialog';
import { achievementMoment, useCeremonies, type CeremonyMoment } from './ceremonies';
import {
  importCareer, rerollQuest, setShowProgress, startCareer, useCareerAchievements, useCareerLevels, useCareerLook, useCareerOpponents,
  useCareerQuests, useCareerShop, useCareerStarters, useCareerState, useCareerUnlocks, useCareerWeekly,
} from './careerData';
import { useCampaigns, useChallenge, useGauntlet, useHordes, useLimited, usePuzzles } from './careerModesData';
import { focusNode, graduated, schoolsOf, storiesOf } from './careerModesModel';
import { useRotatingSceneArt } from './careerScene';
import { playCareerCue } from './careerSound';
import { achievementFact, beatenCount, freshUnlocks, nextOpponent, readSeen, unlockFacts, writeSeen } from './hubModel';
import { dealStyle, useStill } from './motion';
import { CardArt, PortraitCard, type ArtCard } from './Portraits';
import { useCapsuleSummary } from './capsules/capsuleData';
import { CAREER_SLEEVES, cosmeticName, describePay, fraction, unlockedIds, weeklyMarks } from './progressModel';
import { TitleText } from './LookParts';
import { RANDOM_SLEEVE, sleeveBackOf } from '../ui/sleeveArt';
import { useCatalog, useCatalogItem } from '../ui/cosmeticsCatalog';
import { RosterVersus } from './rosterPlay';
import { useCareerDeck } from './useCareerDeck';
import hub from './CareerHub.module.css';
import motion from './motion.module.css';
import { Art } from './Art';
import { usePaintedArt } from './careerArt';
import styles from './Career.module.css';

registerMessages(messages);

/** Career: the opt-in, the starter pick, then the Career's home with its opponents. */
export function CareerScreen() {
  const t = useT();
  const navigate = useNavigate();
  const state = useCareerState();
  const optedIn = useSettings((settings) => settings.settings.careerOptIn);
  // a Career opened elsewhere counts as opted in on this browser too (its cosmetics show in Decks and Settings);
  // every account has progression, so it's the collection that says a Career was opened
  const hasProfile = !!state.data?.profile?.collection;
  useEffect(() => {
    if (hasProfile && !optedIn) useSettings.getState().update({ careerOptIn: true });
  }, [hasProfile, optedIn]);

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (state.isError) {
    return (
      <div className={styles.center}>
        <p>{t('career.failed')}</p>
        <Button onClick={() => void state.refetch()}>{t('career.retry')}</Button>
      </div>
    );
  }
  if (!state.data.enabled) {
    return (
      <div className={styles.center}>
        <p>{t('career.off')}</p>
        <Button onClick={() => navigate('/')}>{t('career.backToPlay')}</Button>
      </div>
    );
  }
  const profile = state.data.profile;
  if (profile?.collection) return <CareerHub profile={profile} />;
  if (!optedIn) return <Intro />;
  return <StarterPicker />;
}

function Intro() {
  const t = useT();
  const navigate = useNavigate();
  return (
    <div className={styles.intro}>
      <h1 className={styles.introTitle}>{t('career.intro.title')}</h1>
      <p className={styles.introLead}>{t('career.intro.lead')}</p>
      <ul className={styles.introList}>
        <li>{t('career.intro.own')}</li>
        <li>{t('career.intro.earn')}</li>
        <li>{t('career.intro.money')}</li>
        <li>{t('career.intro.keep')}</li>
      </ul>
      <div className={styles.introActions}>
        <Button variant="quiet" onClick={() => navigate('/')}>{t('career.intro.notNow')}</Button>
        <Button variant="decision" size="lg" onClick={() => useSettings.getState().update({ careerOptIn: true })}>{t('career.intro.start')}</Button>
      </div>
    </div>
  );
}


function StarterPicker() {
  const t = useT();
  const starters = useCareerStarters(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // the pick is for good, so it's confirmed first
  const [chosen, setChosen] = useState<CareerStarter | null>(null);

  async function pick(starter: CareerStarter) {
    setBusy(starter.id ?? null);
    setError(null);
    try {
      await startCareer(starter.id!, starter.name ?? '');
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      setBusy(null);
    }
    setChosen(null);
  }
  const colorsOf = (starter: CareerStarter) => (starter.colors ?? '').match(/[WUBRG]/g) ?? [];
  // what the colors play like, from the deck's colors: "Kill and bring back. Big creatures, ramp."
  const styleOf = (starter: CareerStarter) => colorsOf(starter).map((letter) => t(`career.starter.style.${letter}` as MessageKey)).join(' ');

  return (
    <div className={styles.page}>
      <header className={styles.pageHead}>
        <div>
          <h1 className={styles.pageTitle}>{t('career.starter.title')}</h1>
          <p className={styles.lead}>{t('career.starter.lead', { coins: 100 })}</p>
        </div>
        <ImportButton />
      </header>
      {error && <p className={styles.error} role="alert">{error}</p>}
      {starters.isPending && <p className={styles.note}>{t('career.loading')}</p>}
      {starters.data?.length === 0 && <p className={styles.note}>{t('career.starter.none')}</p>}
      <ul className={styles.starters}>
        {starters.data?.map((starter) => (
          <li key={starter.id}>
            <button type="button" className={styles.starter} onClick={() => setChosen(starter)} disabled={busy !== null} aria-busy={busy === starter.id || undefined}>
              <span className={styles.starterArt}>
                <CardFace card={{ name: starter.cover?.name ?? starter.name, setCode: starter.cover?.setCode, cardNumber: starter.cover?.cardNumber }} size="normal" />
              </span>
              <b>{starter.name}</b>
              <span className={styles.starterColors}>
                {colorsOf(starter).map((letter) => <i key={letter} className={`ms ms-cost ms-${letter.toLowerCase()}`} role="img" aria-label={t(`career.color.${letter}` as MessageKey)} />)}
                {styleOf(starter)}
              </span>
              <small>{busy === starter.id ? t('career.loading') : t('career.starter.cards', { count: starter.cards ?? 60 })}</small>
            </button>
          </li>
        ))}
      </ul>
      <Dialog
        open={!!chosen}
        onOpenChange={(open) => !open && busy === null && setChosen(null)}
        title={t('career.starter.confirm', { name: chosen?.name ?? '' })}
        description={t('career.starter.confirm.detail')}
        width="sm"
        footer={(
          <>
            <Button variant="quiet" disabled={busy !== null} onClick={() => setChosen(null)}>{t('career.starter.back')}</Button>
            <Button variant="decision" busy={busy !== null} onClick={() => chosen && void pick(chosen)}>{t('career.starter.pick')}</Button>
          </>
        )}
      >
        {chosen && <p className={styles.note}>{styleOf(chosen)}</p>}
      </Dialog>
    </div>
  );
}

/** Bring back an exported Career (only while the account has none). */
function ImportButton() {
  const t = useT();
  const input = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);

  async function chosen(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = '';
    if (!file) return;
    setBusy(true);
    try {
      await importCareer(await file.text());
      notify(t('career.import.done'), file.name);
    } catch (reason) {
      notify(t('career.import.failed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBusy(false);
    }
  }

  return (
    <span className={styles.importBox} title={t('career.import.hint')}>
      <Button variant="print" size="sm" icon={<Upload size={16} />} busy={busy} onClick={() => input.current?.click()}>{t('career.import')}</Button>
      <input ref={input} type="file" accept=".txt,text/plain" hidden onChange={(event) => void chosen(event)} />
    </span>
  );
}

/** The Career's hub: the next duel up front, every mode as a tile, the day's quests at the side. */
function CareerHub({ profile }: { profile: CareerProfile }) {
  const t = useT();
  const navigate = useNavigate();
  const opponents = useCareerOpponents(true);
  const campaigns = useCampaigns(true);
  const gauntlet = useGauntlet(true);
  const puzzles = usePuzzles(true);
  const limited = useLimited(true);
  const challenge = useChallenge(true);
  const hordes = useHordes(true);
  const shop = useCareerShop(true);
  const capsules = useCapsuleSummary(true);
  const achievements = useCareerAchievements(true);
  const quests = useCareerQuests(true);
  const weekly = useCareerWeekly(true);
  const deck = useCareerDeck(profile);
  const playing = usePlay((play) => play.phase !== 'idle');
  const [versus, setVersus] = useState<CareerOpponent | null>(null);
  const roster = useMemo(() => opponents.data ?? [], [opponents.data]);
  const next = useMemo(() => nextOpponent(roster), [roster]);
  const still = useStill();
  useUnlockMoments(roster, campaigns.data, achievements.data, shop.data, opponents.isSuccess && campaigns.isSuccess && achievements.isSuccess && shop.isSuccess);

  const schools = useMemo(() => schoolsOf(campaigns.data), [campaigns.data]);
  const school = useMemo(() => {
    // the school you're furthest into and haven't finished, else the first open one
    const open = schools.filter((item) => item.open && !graduated(item));
    const current = [...open].sort((a, b) => (b.done ?? 0) - (a.done ?? 0))[0];
    const chapters = current?.chapters ?? [];
    const act = chapters.find((item) => !item.done);
    return { current, node: act ? focusNode(act) : undefined };
  }, [schools]);
  const chapter = useMemo(() => {
    const campaign = storiesOf(campaigns.data)[0];
    const chapters = campaign?.chapters ?? [];
    const current = chapters.find((item) => !item.done && (item.nodes ?? []).some((node) => node.state === 'open')) ?? chapters.find((item) => !item.done);
    return { campaign, current, node: current ? focusNode(current) : undefined };
  }, [campaigns.data]);
  const solved = (puzzles.data ?? []).filter((puzzle) => puzzle.solved).length;
  const run = gauntlet.data?.run;
  const runOn = run && run.state === 'active';
  const limitedRun = limited.data?.run;
  const earned = (achievements.data ?? []).filter((achievement) => achievement.achieved).length;
  const packs = profile.packTokens ?? 0;
  const price = shop.data?.[0]?.price ?? 100;

  const tiles: Tile[] = [
    {
      to: '/career/opponents', title: t('career.nav.home'), art: next?.cover ?? null, artName: next?.name ?? '',
      status: t('career.hub.opponents', { beaten: beatenCount(roster), total: roster.length }),
    },
    {
      to: '/career/campaign', title: t('career.modes.campaign'), art: chapter.node?.cover ?? null, artName: chapter.node?.name ?? '',
      status: chapter.current ? t('career.hub.campaign', { chapter: chapter.current.name ?? '', done: chapter.campaign?.done ?? 0, total: chapter.campaign?.total ?? 0 }) : t('career.hub.campaignDone'),
    },
    ...(schools.length > 0 ? [{
      to: school.current ? `/career/academy/${school.current.id}` : '/career/academy', title: t('career.academy.title'),
      art: school.node?.cover ?? HUB_ART.academy, artName: school.node?.opponentName ?? '',
      status: school.current
        ? t('career.hub.academy', { school: school.current.name ?? '', done: school.current.bosses ?? 0, total: school.current.chapters?.length ?? 4 })
        : t('career.hub.academyOpen', { open: schools.filter((item) => item.open).length, total: schools.length }),
    }] : []),
    {
      to: '/career/gauntlet', title: t('career.modes.gauntlet'), art: run?.next?.cover ?? HUB_ART.gauntlet, artName: run?.next?.name ?? '',
      status: runOn ? t('career.hub.gauntletRun', { wins: run?.wins ?? 0 }) : t('career.hub.gauntletEntry', { coins: gauntlet.data?.entryCoins ?? 0 }),
      badge: runOn ? t('career.hub.inProgress') : undefined,
    },
    {
      to: '/career/puzzles', title: t('career.modes.puzzles'), art: HUB_ART.puzzles, artName: '',
      status: t('career.hub.puzzles', { solved, total: puzzles.data?.length ?? 0 }),
    },
    {
      to: '/career/limited', title: t('career.modes.limited'), art: HUB_ART.limited, artName: '',
      status: limitedRun && limitedRun.state === 'active'
        ? t('career.hub.limitedRun', { wins: limitedRun.wins ?? 0, losses: limitedRun.losses ?? 0 })
        : t('career.hub.limitedEntry', { sealed: limited.data?.sealedEntry ?? 0, draft: limited.data?.draftEntry ?? 0 }),
      badge: limitedRun && limitedRun.state === 'active' ? t('career.hub.inProgress') : undefined,
    },
    {
      to: '/career/challenge', title: t('career.modes.challenge'), art: challenge.data?.cover ?? null, artName: challenge.data?.opponentName ?? '',
      status: challenge.data?.won ? t('career.hub.challengeWon') : challenge.data?.name ?? '',
    },
    ...((hordes.data ?? []).length > 0 ? [{
      to: '/career/hordes', title: t('career.modes.hordes'), art: (hordes.data ?? []).find((item) => !item.beaten)?.cover ?? hordes.data?.[0]?.cover ?? null, artName: '',
      status: t('career.horde.beatenCount', { beaten: (hordes.data ?? []).filter((item) => item.beaten).length, total: hordes.data?.length ?? 0 }),
    }] : []),
    ...(capsules ? [{
      to: '/career/capsules', title: t('capsules.title'), art: null, artName: '', image: capsules.art,
      status: t('capsules.hub', { have: capsules.have, total: capsules.total }),
      badge: capsules.tokens > 0 ? t('capsules.hubTokens', { count: capsules.tokens }) : undefined,
    }] : []),
    {
      to: '/career/shop', title: t('career.nav.shop'), art: HUB_ART.shop, artName: '',
      status: packs > 0 ? t('career.hub.shopFree', { count: packs }) : t('career.hub.shop', { price }),
      badge: packs > 0 ? t('career.hub.packs', { count: packs }) : undefined,
    },
    {
      to: '/career/collection', title: t('career.nav.collection'), art: HUB_ART.collection, artName: '',
      status: t('career.hub.collection', { count: deck.cardCount }),
    },
    {
      to: '/career/progress', title: t('career.nav.progress'), art: HUB_ART.progress, artName: '',
      status: t('career.hub.progress', { level: profile.level ?? 1, earned }),
    },
  ];
  // the mat behind the hub turns through the next duel and the modes' art
  useRotatingSceneArt([next?.cover ?? deck.cover, ...tiles.map((tile) => tile.art)], still);

  return (
    <div className={hub.hub}>
      <section className={hub.hero} aria-labelledby="hub-next">
        {next ? (
          <>
            <PortraitCard
              name={next.name ?? ''}
              colors={next.colors}
              cover={next.cover}
              size="lg"
              className={[hub.heroCard, motion.deal].join(' ')}
              onPick={() => setVersus(next)}
              label={t('career.hub.face', { opponent: next.name ?? '' })}
            />
            <div className={hub.heroText}>
              <p className={hub.kicker}>{t('career.hub.next')} · {t('career.tier', { tier: next.tier ?? 1, name: next.tierName ?? '' })}</p>
              <h2 id="hub-next" className={hub.heroName}>{next.name}</h2>
              <p className={hub.heroLine}>{next.tagline}</p>
              <p className={hub.heroPays}><Coins size={18} aria-hidden="true" /> {t('career.opponent.pays', { coins: next.winCoins ?? 0 })}</p>
              <div className={hub.heroActions}>
                {deck.problem ? (
                  <Button variant="decision" size="xl" onClick={() => navigate('/career/deck')} data-nav>{t('career.deck.fix')}</Button>
                ) : (
                  <Button variant="decision" size="xl" icon={<Swords size={24} />} disabled={playing || !deck.deck} onClick={() => setVersus(next)} data-nav>
                    {t('career.opponent.play')}
                  </Button>
                )}
                <Button variant="print" size="lg" onClick={() => navigate('/career/opponents')} data-nav>{t('career.hub.allOpponents')}</Button>
              </div>
              <div className={hub.deckPlate}>
                <span>
                  <b>{deck.deck?.name ?? t('career.nav.deck')}</b>
                  <small>{deck.problem ?? t('career.deck.summary', { count: deck.size })}</small>
                </span>
                <Button variant="quiet" size="sm" onClick={() => navigate('/career/deck')} data-nav>{t('career.deck.edit')}</Button>
              </div>
            </div>
          </>
        ) : (
          <p className={styles.note}>{opponents.isError ? t('career.failed') : t('career.loading')}</p>
        )}
      </section>

      <nav className={hub.tiles} aria-label={t('career.hub.modes')}>
        {tiles.map((tile, index) => <ModeTile key={tile.to} tile={tile} index={index} />)}
      </nav>

      <aside className={hub.side}>
        <section className={[hub.panel, motion.deal].join(' ')} style={dealStyle(3)}>
          <h2 className={styles.cardLabel}>{t('career.quests')}</h2>
          {quests.isPending ? <p className={styles.small}>{t('career.loading')}</p> : <QuestList quests={quests.data ?? undefined} />}
        </section>
        <section className={[hub.panel, motion.deal].join(' ')} style={dealStyle(4)}>
          <h2 className={styles.cardLabel}>{t('career.weekly')}</h2>
          <WeeklyGoal weekly={weekly.data} />
        </section>
      </aside>

      {versus && <RosterVersus opponent={versus} deck={deck} onClose={() => setVersus(null)} />}
    </div>
  );
}

/** Cards whose art stands for a mode that has no opponent of its own to show. */
const HUB_ART = {
  academy: { name: 'Daxos of Meletis', setCode: 'THS', cardNumber: '191' },
  gauntlet: { name: 'Stoneshock Giant', setCode: 'THS', cardNumber: '142' },
  puzzles: { name: 'Omenspeaker', setCode: 'THS', cardNumber: '57' },
  limited: { name: 'Chronicler of Heroes', setCode: 'THS', cardNumber: '190' },
  shop: { name: 'Sylvan Caryatid', setCode: 'THS', cardNumber: '180' },
  collection: { name: 'Horizon Scholar', setCode: 'THS', cardNumber: '51' },
  progress: { name: "Elspeth, Sun's Champion", setCode: 'THS', cardNumber: '9' },
} as const;

interface Tile {
  to: string;
  title: string;
  status: string;
  art: ArtCard | null;
  artName: string;
  badge?: string;
  /** a painted picture in place of a card's art (the capsule machines) */
  image?: string | null;
}

function ModeTile({ tile, index }: { tile: Tile; index: number }) {
  return (
    <Link to={tile.to} className={[hub.tile, motion.deal].join(' ')} style={dealStyle(index)} onClick={() => playCareerCue('whoosh')} data-nav>
      {tile.image
        ? <img src={tile.image} alt="" className={[hub.tileArt, hub.tileImage].join(' ')} draggable={false} />
        : <CardArt card={tile.art} name={tile.artName || tile.title} className={hub.tileArt} />}
      <span className={hub.tileText}>
        <b>{tile.title}</b>
        <small>{tile.status}</small>
      </span>
      {tile.badge && <span className={hub.badge}>{tile.badge}</span>}
    </Link>
  );
}

/** New tiers, chapters, shop sets and achievements get their moment the first time the hub sees them. */
function useUnlockMoments(
  roster: readonly CareerOpponent[],
  campaigns: readonly CareerCampaign[] | undefined,
  achievements: readonly CareerAchievement[] | undefined,
  shop: readonly CareerShopSet[] | undefined,
  ready: boolean,
) {
  const t = useT();
  const user = useSession((state) => state.userName);
  useEffect(() => {
    if (!ready) return;
    const { fresh, seen } = freshUnlocks(readSeen(user), unlockFacts(roster, campaigns ?? [], achievements ?? [], shop ?? []));
    writeSeen(user, seen);
    const moments: CeremonyMoment[] = fresh.flatMap((fact): CeremonyMoment[] => {
      const [kind, ...rest] = fact.split(':');
      if (kind === 'tier') {
        const tier = Number(rest[0]);
        const members = roster.filter((opponent) => opponent.tier === tier);
        const first = members[0];
        if (!first) return [];
        return [{
          id: fact,
          kicker: t('career.ceremony.tier'),
          title: t('career.tier', { tier, name: first.tierName ?? '' }),
          text: t('career.ceremony.tierText', { names: members.map((opponent) => opponent.name).join(', ') }),
          card: first.cover ?? undefined,
        }];
      }
      if (kind === 'school') {
        const school = campaigns?.find((campaign) => campaign.id === rest[0]);
        if (!school) return [];
        return [{ id: fact, kicker: t('career.ceremony.school'), title: school.name ?? '', text: school.summary, crest: { name: school.name ?? '', colors: school.colors } }];
      }
      if (kind === 'chapter') {
        const [campaignId, chapterId] = rest;
        const chapter = campaigns?.find((campaign) => campaign.id === campaignId)?.chapters?.find((item) => item.id === chapterId);
        if (!chapter) return [];
        return [{ id: fact, kicker: t('career.ceremony.chapter'), title: chapter.name ?? '', text: chapter.text, crest: { name: chapter.name ?? '', colors: chapter.color } }];
      }
      if (kind === 'set') {
        const set = shop?.find((item) => item.setCode === rest[0]);
        if (!set) return [];
        return [{ id: fact, kicker: t('career.ceremony.set'), title: set.name ?? set.setCode ?? '', text: t('career.ceremony.setText', { chapter: set.unlockedBy ?? '' }), crest: { name: set.setCode ?? '' }, cue: 'coins' }];
      }
      const achievement = achievements?.find((item) => achievementFact(item) === fact);
      if (!achievement) return [];
      return [achievementMoment(t, achievement)];
    });
    if (moments.length > 0) useCeremonies.getState().show(moments);
  }, [ready, roster, campaigns, achievements, shop, user, t]);
}

// ---- Career outside its screens (loaded with this one): your profile's Career line, and unlocked sleeves in Decks

/** Your Career in a line, for your profile: level, title and achievements, with the way to the full list. */
export function CareerSummary({ user, onOpen }: { user: string; onOpen(): void }) {
  const t = useT();
  const navigate = useNavigate();
  const levels = useCareerLevels(true);
  const achievements = useCareerAchievements(true);
  const avatar = useCareerLook((look) => look.looks[user.toLowerCase()]?.avatar ?? null);
  const title = useCareerLook((look) => look.looks[user.toLowerCase()]?.title ?? null);
  if (!levels.data || !achievements.data) return null;
  const earned = achievements.data.filter((achievement) => achievement.achieved).length;
  return (
    <div className={styles.summary}>
      <CareerAvatar id={avatar} name={user} size={32} />
      <span className={styles.identityText}>
        {title && <TitleText as="b" id={title} />}
        <small>{t('career.profile.line', { level: levels.data.level ?? 1, earned, total: achievements.data.length })}</small>
      </span>
      <Button size="sm" variant="quiet" icon={<Trophy size={16} />} onClick={() => { onOpen(); navigate('/career/progress?tab=achievements'); }}>
        {t('career.progress.achievements')}
      </Button>
    </div>
  );
}

/**
 * The sleeves a Career unlocked, beside the free ones wherever a deck's sleeves are chosen, and "random" (once there
 * are two to pick from): the deck wears one of them, a different one each game (ui/sleeveArt.ts).
 */
/** A sleeve's swatch in a picker: its painted back when painted art is on, its dye otherwise. */
function sleeveSwatchBackground(color: string, paintedArt: boolean): string {
  const back = sleeveBackOf(color);
  return back && paintedArt ? `center / cover no-repeat url("${back}"), ${color}` : color;
}

export function CareerSleeves({ current, swatchClass, onPick, onPreview }: {
  current: string;
  swatchClass: string;
  onPick(color: string): void;
  /** right-click: a sleeve up close */
  onPreview?(color: string, name: string): void;
}) {
  const t = useT();
  const paintedArt = useSettings((state) => state.settings.paintedArt);
  const unlocks = useCareerUnlocks();
  // the catalog says which sleeves exist and their dyes
  useCatalog();
  const owned = unlockedIds(unlocks, 'sleeve').filter((id) => id in CAREER_SLEEVES);
  const random = t('career.locker.randomSleeve');
  return (
    <>
      {owned.map((id) => {
        const color = CAREER_SLEEVES[id];
        const name = t('career.reward.sleeve', { name: cosmeticName(t, { kind: 'sleeve', id }) });
        return (
          <button
            key={id}
            type="button"
            role="radio"
            aria-checked={current === color}
            aria-label={name}
            title={name}
            className={swatchClass}
            style={{ background: sleeveSwatchBackground(color, paintedArt) }}
            onClick={() => onPick(color)}
            onContextMenu={onPreview ? (event) => {
              event.preventDefault();
              onPreview(color, name);
            } : undefined}
          />
        );
      })}
      {owned.length >= 2 && (
        <button
          type="button"
          role="radio"
          aria-checked={current === RANDOM_SLEEVE}
          aria-label={random}
          title={random}
          className={swatchClass}
          style={{ display: 'grid', placeItems: 'center', color: 'var(--ink-strong)', background: `conic-gradient(${owned.slice(0, 6).map((id) => CAREER_SLEEVES[id]).join(', ')}, ${CAREER_SLEEVES[owned[0]]})` }}
          onClick={() => onPick(RANDOM_SLEEVE)}
        >
          <Shuffle size={14} aria-hidden="true" />
        </button>
      )}
    </>
  );
}

// ---- progress pieces the Career screens share

/** A Career avatar: its painted bust on a medallion in its colours, or the player's initial before one is chosen. */
export function CareerAvatar({ id, name, size = 48 }: { id: string | null | undefined; name: string; size?: number }) {
  const painted = usePaintedArt();
  const item = useCatalogItem('avatar', id);
  const art = item ? { light: item.light ?? '#7a7466', dark: item.dark ?? '#22201b', path: item.path ?? '' } : undefined;
  const style = { '--crest-size': `${size}px`, ...(art ? { '--crest-light': art.light, '--crest-dark': art.dark } : {}) } as CSSProperties;
  const drawn = art && (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <path d={art.path} />
    </svg>
  );
  return (
    <span className={[styles.crest, art && painted ? styles.crestArt : ''].join(' ')} style={style} aria-hidden="true">
      {art && id ? <Art kind="avatar" id={id} size={size} fallback={drawn} /> : <b>{name.slice(0, 1).toUpperCase()}</b>}
    </span>
  );
}

/** A thin bar for progress toward a target. */
export function ProgressBar({ value, label, done }: { value: number; label: string; done?: boolean }) {
  return (
    <div className={[styles.bar2, done ? styles.bar2Done : ''].join(' ')} role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(value * 100)} aria-label={label}>
      <span style={{ width: `${Math.round(value * 100)}%` }} />
    </div>
  );
}

/** The weekly goal: Career wins this week on one bar, with each goal that gives a free pack marked along it. */
export function WeeklyGoal({ weekly }: { weekly: CareerWeekly | undefined }) {
  const t = useT();
  const marks = weeklyMarks(weekly);
  const wins = weekly?.wins ?? 0;
  return (
    <div className={styles.weekly}>
      <p>
        <b>{t('career.weekly.wins', { count: wins })}</b>
        <small>{weekly?.nextGoal ? t('career.weekly.next', { goal: weekly.nextGoal }) : t('career.weekly.done')}</small>
      </p>
      <div className={styles.weeklyTrack} role="progressbar" aria-valuemin={0} aria-valuemax={marks.top} aria-valuenow={Math.min(wins, marks.top)} aria-label={t('career.weekly')}>
        <span className={styles.weeklyFill} style={{ width: `${marks.fill * 100}%` }} />
        {marks.goals.map((goal) => (
          <span
            key={goal.goal}
            className={[styles.weeklyMark, goal.reached ? styles.weeklyMarkOn : ''].join(' ')}
            style={{ left: `${goal.at * 100}%` }}
            title={t('career.weekly.goal', { goal: goal.goal })}
          >
            <Gift size={12} aria-hidden="true" />
            <small>{goal.goal}</small>
          </span>
        ))}
      </div>
    </div>
  );
}

/** The quests waiting, each with its bar and pay, and the day's swap. */
export function QuestList({ quests }: { quests: CareerQuests | undefined }) {
  const t = useT();
  const [swapping, setSwapping] = useState<number | null>(null);
  const list = quests?.quests ?? [];
  const canReroll = !!quests?.canReroll;

  async function swap(quest: CareerQuest) {
    setSwapping(quest.slot ?? null);
    try {
      await rerollQuest(quest.slot ?? 0);
    } catch (reason) {
      notify(t('career.quests.swapFailed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setSwapping(null);
    }
  }

  return (
    <>
      {list.length === 0 && <p className={styles.small}>{t('career.quests.none')}</p>}
      <ul className={styles.quests}>
        {list.map((quest) => (
          <li key={quest.slot} className={styles.quest}>
            <span>
              <b>{quest.weekly && <em className={styles.weeklyTag}>{t('career.quests.weekly')}</em>}{quest.text}</b>
              <small>{[quest.counts, describePay(t, quest.coins, quest.xp)].filter(Boolean).join(' · ')}</small>
            </span>
            {quest.weekly ? <span /> : <Button
              variant="quiet"
              size="sm"
              icon={<RefreshCw size={14} />}
              busy={swapping === quest.slot}
              disabled={!canReroll || swapping !== null}
              aria-label={t('career.quests.swapLabel', { quest: quest.text ?? '' })}
              title={canReroll ? t('career.quests.swapLabel', { quest: quest.text ?? '' }) : t('career.quests.swapUsed')}
              onClick={() => void swap(quest)}
            >
              {t('career.quests.swap')}
            </Button>}
            <ProgressBar value={fraction(quest.progress, quest.target)} label={quest.text ?? ''} />
            <small>{quest.progress ?? 0} / {quest.target ?? 0}</small>
          </li>
        ))}
      </ul>
      <p className={styles.small}>{t('career.quests.note')}{canReroll ? '' : ` ${t('career.quests.swapUsed')}`}</p>
    </>
  );
}

/** Whether progression shows: the header, Today on Home and rewards after games. Games count either way. */
export function ShowProgress({ profile }: { profile: CareerProfile }) {
  const t = useT();
  const [busy, setBusy] = useState(false);

  async function change(show: boolean) {
    setBusy(true);
    try {
      await setShowProgress(show);
    } catch (reason) {
      notify(t('career.settings.failed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBusy(false);
    }
  }

  return (
    <label className={styles.setting}>
      <input type="checkbox" checked={profile.showProgress !== false} disabled={busy} onChange={(event) => void change(event.target.checked)} />
      <span>
        {t('career.settings.showProgress')}
        <small>{t('career.settings.showProgressNote')}</small>
      </span>
    </label>
  );
}

/** A done mark for finished quests and earned achievements. */
export function DoneMark({ label }: { label: string }) {
  return <span className={styles.doneMark} title={label}><Check size={14} aria-hidden="true" /><span className={styles.srOnly}>{label}</span></span>;
}
