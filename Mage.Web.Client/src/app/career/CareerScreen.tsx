import { Check, Coins, Compass, Crown, Gift, Layers, RefreshCw, Shuffle, Swords, Trophy, Upload } from 'lucide-react';
import { useEffect, useMemo, useRef, useState, type CSSProperties, type ChangeEvent, type ReactNode } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import type { CareerAchievement, CareerCampaign, CareerOpponent, CareerProfile, CareerShopSet, CareerQuest, CareerQuests, CareerStarter, CareerState, CareerWeekly } from '../../protocol/generated/views';
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
  acquirePrecon, importCareer, rerollQuest, setQuestPref, setShowProgress, startCareer, useCareerAchievements, useCareerCommanderOpponents, useCareerLevels, useCareerLook, useCareerOpponents,
  useCareerQuests, useCareerShop, useCareerStarters, useCareerState, useCareerUnlocks, useCareerWeekly,
} from './careerData';
import { useArchenemy, useCampaigns, useChallenge, useGauntlet, useHordes, useLimited, usePuzzles, useThg } from './careerModesData';
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
  if (profile?.collection) return <CareerHub profile={profile} state={state.data} />;
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
  const [track, setTrack] = useState<'constructed' | 'commander'>('constructed');
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // the pick is for good, so it's confirmed first
  const [chosen, setChosen] = useState<CareerStarter | null>(null);

  const visibleStarters = useMemo(() => (starters.data ?? []).filter((starter) => {
    const isCommander = starter.track === 'commander' || (starter.cards ?? 60) === 100;
    return track === 'commander' ? isCommander : !isCommander;
  }), [starters.data, track]);

  async function pick(starter: CareerStarter) {
    setBusy(starter.id ?? null);
    setError(null);
    try {
      await startCareer(starter.id!, starter.name ?? '', starter.format);
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
          <p className={styles.lead}>{t('career.starter.leadTracks', { coins: 100 })}</p>
        </div>
        <ImportButton />
      </header>
      <div className={styles.starterTracks} role="tablist" aria-label={t('career.starter.trackLabel')}>
        <button
          type="button"
          role="tab"
          aria-selected={track === 'constructed'}
          className={[styles.starterTrackBtn, track === 'constructed' ? styles.starterTrackOn : ''].join(' ')}
          onClick={() => setTrack('constructed')}
        >
          <Swords size={16} aria-hidden="true" />
          {t('career.starter.trackConstructed')}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={track === 'commander'}
          className={[styles.starterTrackBtn, track === 'commander' ? styles.starterTrackOn : ''].join(' ')}
          onClick={() => setTrack('commander')}
        >
          <Crown size={16} aria-hidden="true" />
          {t('career.starter.trackCommander')}
        </button>
      </div>
      {error && <p className={styles.error} role="alert">{error}</p>}
      {starters.isPending && <p className={styles.note}>{t('career.loading')}</p>}
      {!starters.isPending && visibleStarters.length === 0 && <p className={styles.note}>{t('career.starter.none')}</p>}
      <ul className={styles.starters}>
        {visibleStarters.map((starter) => (
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

type CareerTrack = 'overview' | 'constructed' | 'commander' | 'pve';

function savedTrack(user: string, fallback: CareerTrack): CareerTrack {
  try {
    const raw = localStorage.getItem(`playmat.career.track.${user.toLowerCase()}`);
    if (raw === 'overview' || raw === 'constructed' || raw === 'commander' || raw === 'pve') return raw;
  } catch {
    // ignore storage errors
  }
  return fallback;
}

/** The Career's track-aware command hub: switcher on top, adaptive hero up front, mode tiles, and quests at the side. */
function CareerHub({ profile, state }: { profile: CareerProfile; state: CareerState }) {
  const t = useT();
  const navigate = useNavigate();
  const user = useSession((session) => session.userName);
  const starters = useCareerStarters(true);
  const opponents = useCareerOpponents(true);
  const commanderOpponents = useCareerCommanderOpponents(true);
  const campaigns = useCampaigns(true);
  const gauntlet = useGauntlet(true);
  const puzzles = usePuzzles(true);
  const limited = useLimited(true);
  const challenge = useChallenge(true);
  const hordes = useHordes(true);
  const archenemy = useArchenemy(true);
  const thg = useThg(true);
  const shop = useCareerShop(true);
  const capsules = useCapsuleSummary(true);
  const achievements = useCareerAchievements(true);
  const quests = useCareerQuests(true);
  const weekly = useCareerWeekly(true);
  const deck = useCareerDeck(profile);
  const playing = usePlay((play) => play.phase !== 'idle');
  const [versus, setVersus] = useState<CareerOpponent | null>(null);
  const [giftOpen, setGiftOpen] = useState(false);
  const [prefBusy, setPrefBusy] = useState(false);

  const commanderStarters = useMemo(
    () => (starters.data ?? []).filter((starter) => starter.track === 'commander' || (starter.cards ?? 60) === 100),
    [starters.data],
  );
  const startedCommander = useMemo(
    () => commanderStarters.some((starter) => starter.id === profile.starter),
    [commanderStarters, profile.starter],
  );
  const [track, setTrack] = useState<CareerTrack>(() => savedTrack(user, startedCommander ? 'commander' : 'overview'));

  function pickTrack(nextTrack: CareerTrack) {
    setTrack(nextTrack);
    try {
      localStorage.setItem(`playmat.career.track.${user.toLowerCase()}`, nextTrack);
    } catch {
      // ignore storage errors
    }
  }

  async function onQuestPref(pref: 'any' | 'constructed' | 'commander') {
    if (prefBusy || (state.questPref ?? 'any') === pref) return;
    setPrefBusy(true);
    try {
      await setQuestPref(pref);
      notify(t('career.quests.prefSaved'), t(`career.quests.pref.${pref}` as MessageKey));
    } catch (reason) {
      notify(t('career.failed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setPrefBusy(false);
    }
  }

  const roster = useMemo(() => opponents.data ?? [], [opponents.data]);
  const next = useMemo(() => nextOpponent(roster), [roster]);
  const commanderRoster = useMemo(() => commanderOpponents.data ?? [], [commanderOpponents.data]);
  const nextCommander = useMemo(() => nextOpponent(commanderRoster), [commanderRoster]);
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

  const activeCommanderSaved = useMemo(
    () => (deck.isCommander ? deck.activeSaved : deck.commanderDecks[0] ?? null),
    [deck.isCommander, deck.activeSaved, deck.commanderDecks],
  );
  const commanderCard = activeCommanderSaved?.deck?.sideboard?.[0];
  const commanderCover = commanderCard?.cardName
    ? { name: commanderCard.cardName, setCode: commanderCard.setCode, cardNumber: commanderCard.cardNumber }
    : commanderStarters[0]?.cover ?? HUB_ART.league;

  const allTiles: (Tile & { tracks: CareerTrack[] })[] = [
    {
      to: '/career/opponents', title: t('career.nav.home'), art: next?.cover ?? null, artName: next?.name ?? '',
      status: t('career.hub.opponents', { beaten: beatenCount(roster), total: roster.length }),
      tracks: ['overview', 'constructed'],
    },
    {
      to: '/career/ranked', title: t('career.ranked.title'), art: HUB_ART.ranked, artName: '',
      status: t('career.hub.ranked'),
      tracks: ['overview', 'constructed'],
    },
    {
      to: '/career/league', title: t('career.hub.leagueTitle'), art: HUB_ART.league, artName: '',
      status: t('career.hub.league'),
      tracks: ['overview', 'constructed'],
    },
    {
      to: '/career/opponents?track=commander', title: t('career.ladder.track.commander'), art: nextCommander?.cover ?? commanderCover, artName: nextCommander?.name ?? '',
      status: t('career.hub.opponents', { beaten: beatenCount(commanderRoster), total: commanderRoster.length }),
      tracks: ['commander'],
    },
    {
      to: '/career/ranked?format=commander&queue=pod', title: t('career.hub.commanderRankedTitle'), art: HUB_ART.ranked, artName: '',
      status: t('career.hub.commanderRanked'),
      tracks: ['commander'],
    },
    {
      to: '/career/league?format=commander', title: t('career.hub.commanderLeagueTitle'), art: HUB_ART.league, artName: '',
      status: t('career.hub.commanderLeague'),
      tracks: ['commander'],
    },
    {
      to: '/career/deck', title: t('career.hub.commanderDeckTitle'), art: commanderCover, artName: commanderCard?.cardName ?? '',
      status: deck.commanderDecks.length > 0
        ? t('career.hub.commanderDecks', { count: deck.commanderDecks.length })
        : t('career.hub.commanderDeckStart'),
      tracks: ['commander'],
    },
    {
      to: '/career/shop?cat=precons', title: t('career.shop.preconsTitle'), art: commanderStarters[1]?.cover ?? HUB_ART.shop, artName: '',
      status: state.commanderPreconGift
        ? t('career.hub.preconGiftShort')
        : t('career.hub.preconShop', { owned: state.ownedPrecons?.length ?? 0, total: commanderStarters.length || 6 }),
      badge: state.commanderPreconGift ? t('career.hub.freeBadge') : undefined,
      tracks: ['commander'],
    },
    {
      to: '/career/campaign', title: t('career.modes.campaign'), art: chapter.node?.cover ?? null, artName: chapter.node?.name ?? '',
      status: chapter.current ? t('career.hub.campaign', { chapter: chapter.current.name ?? '', done: chapter.campaign?.done ?? 0, total: chapter.campaign?.total ?? 0 }) : t('career.hub.campaignDone'),
      tracks: ['overview', 'constructed', 'pve'],
    },
    ...(schools.length > 0 ? [{
      to: school.current ? `/career/academy/${school.current.id}` : '/career/academy', title: t('career.academy.title'),
      art: school.node?.cover ?? HUB_ART.academy, artName: school.node?.opponentName ?? '',
      status: school.current
        ? t('career.hub.academy', { school: school.current.name ?? '', done: school.current.bosses ?? 0, total: school.current.chapters?.length ?? 4 })
        : t('career.hub.academyOpen', { open: schools.filter((item) => item.open).length, total: schools.length }),
      tracks: ['overview', 'constructed', 'pve'] as CareerTrack[],
    }] : []),
    {
      to: '/career/gauntlet', title: t('career.modes.gauntlet'), art: run?.next?.cover ?? HUB_ART.gauntlet, artName: run?.next?.name ?? '',
      status: runOn ? t('career.hub.gauntletRun', { wins: run?.wins ?? 0 }) : t('career.hub.gauntletEntry', { coins: gauntlet.data?.entryCoins ?? 0 }),
      badge: runOn ? t('career.hub.inProgress') : undefined,
      tracks: ['overview', 'commander', 'pve'],
    },
    {
      to: '/career/puzzles', title: t('career.modes.puzzles'), art: HUB_ART.puzzles, artName: '',
      status: t('career.hub.puzzles', { solved, total: puzzles.data?.length ?? 0 }),
      tracks: ['overview', 'pve'],
    },
    {
      to: '/career/limited', title: t('career.modes.limited'), art: HUB_ART.limited, artName: '',
      status: limitedRun && limitedRun.state === 'active'
        ? t('career.hub.limitedRun', { wins: limitedRun.wins ?? 0, losses: limitedRun.losses ?? 0 })
        : t('career.hub.limitedEntry', { sealed: limited.data?.sealedEntry ?? 0, draft: limited.data?.draftEntry ?? 0 }),
      badge: limitedRun && limitedRun.state === 'active' ? t('career.hub.inProgress') : undefined,
      tracks: ['overview', 'pve'],
    },
    {
      to: '/career/challenge', title: t('career.modes.challenge'), art: challenge.data?.cover ?? null, artName: challenge.data?.opponentName ?? '',
      status: challenge.data?.won ? t('career.hub.challengeWon') : challenge.data?.name ?? '',
      tracks: ['overview', 'commander', 'pve'],
    },
    ...((hordes.data ?? []).length > 0 ? [{
      to: '/career/hordes', title: t('career.modes.hordes'), art: (hordes.data ?? []).find((item) => !item.beaten)?.cover ?? hordes.data?.[0]?.cover ?? null, artName: '',
      status: t('career.horde.beatenCount', { beaten: (hordes.data ?? []).filter((item) => item.beaten).length, total: hordes.data?.length ?? 0 }),
      tracks: ['overview', 'commander', 'pve'] as CareerTrack[],
    }] : []),
    ...((archenemy.data?.encounters ?? []).length > 0 ? [{
      to: '/career/archenemy', title: t('career.archenemy.title'), art: (archenemy.data?.encounters ?? []).find((item) => !item.beaten)?.cover ?? archenemy.data?.encounters?.[0]?.cover ?? null, artName: '',
      status: t('career.horde.beatenCount', { beaten: (archenemy.data?.encounters ?? []).filter((item) => item.beaten).length, total: archenemy.data?.encounters?.length ?? 0 }),
      tracks: ['overview', 'commander', 'pve'] as CareerTrack[],
    }] : []),
    ...((thg.data?.pairs ?? []).length > 0 ? [{
      to: '/career/thg', title: t('career.thg.title'), art: (thg.data?.pairs ?? []).find((item) => !item.beaten)?.cover ?? thg.data?.pairs?.[0]?.cover ?? null, artName: '',
      status: t('career.horde.beatenCount', { beaten: (thg.data?.pairs ?? []).filter((item) => item.beaten).length, total: thg.data?.pairs?.length ?? 0 }),
      tracks: ['overview', 'commander', 'pve'] as CareerTrack[],
    }] : []),
    ...(capsules ? [{
      to: '/career/capsules', title: t('capsules.title'), art: null, artName: '', image: capsules.art,
      status: t('capsules.hub', { have: capsules.have, total: capsules.total }),
      badge: capsules.tokens > 0 ? t('capsules.hubTokens', { count: capsules.tokens }) : undefined,
      tracks: ['overview', 'commander'] as CareerTrack[],
    }] : []),
    {
      to: track === 'commander' ? '/career/shop?cat=commander' : '/career/shop', title: t('career.nav.shop'), art: HUB_ART.shop, artName: '',
      status: packs > 0 ? t('career.hub.shopFree', { count: packs }) : t('career.hub.shop', { price }),
      badge: packs > 0 ? t('career.hub.packs', { count: packs }) : undefined,
      tracks: ['overview', 'constructed', 'commander'],
    },
    {
      to: '/career/collection', title: t('career.nav.collection'), art: HUB_ART.collection, artName: '',
      status: t('career.hub.collection', { count: deck.cardCount }),
      tracks: ['overview', 'constructed', 'commander'],
    },
    {
      to: '/career/progress', title: t('career.nav.progress'), art: HUB_ART.progress, artName: '',
      status: t('career.hub.progress', { level: profile.level ?? 1, earned }),
      tracks: ['overview', 'constructed', 'commander', 'pve'],
    },
  ];
  const tiles = allTiles.filter((tile) => tile.tracks.includes(track));
  // the mat behind the hub turns through the next duel and the modes' art
  useRotatingSceneArt([track === 'commander' ? commanderCover : next?.cover ?? deck.cover, ...tiles.map((tile) => tile.art)], still);

  const trackTabs: { id: CareerTrack; label: MessageKey; icon: ReactNode }[] = [
    { id: 'overview', label: 'career.track.overview', icon: <Compass size={15} aria-hidden="true" /> },
    { id: 'constructed', label: 'career.track.constructed', icon: <Swords size={15} aria-hidden="true" /> },
    { id: 'commander', label: 'career.track.commander', icon: <Crown size={15} aria-hidden="true" /> },
    { id: 'pve', label: 'career.track.pve', icon: <Layers size={15} aria-hidden="true" /> },
  ];

  const questPref = state.questPref ?? 'any';

  return (
    <div className={hub.hub}>
      <div className={hub.tracksBar}>
        <div className={hub.tracks} role="tablist" aria-label={t('career.track.switcher')}>
          {trackTabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={track === tab.id}
              className={[hub.trackTab, track === tab.id ? hub.trackTabOn : ''].join(' ')}
              onClick={() => pickTrack(tab.id)}
              data-nav
            >
              {tab.icon}
              {t(tab.label)}
            </button>
          ))}
        </div>
        {state.commanderPreconGift && (
          <button type="button" className={hub.giftBanner} onClick={() => setGiftOpen(true)} data-nav>
            <Gift size={15} aria-hidden="true" />
            {t('career.hub.claimFreePrecon')}
          </button>
        )}
      </div>

      <section className={hub.hero} aria-labelledby="hub-next">
        {track === 'commander' ? (
          state.commanderPreconGift ? (
            <>
              <PortraitCard
                name={commanderStarters[0]?.name ?? t('career.track.commander')}
                colors={commanderStarters[0]?.colors}
                cover={commanderStarters[0]?.cover ?? HUB_ART.league}
                size="lg"
                className={[hub.heroCard, motion.deal].join(' ')}
                onPick={() => setGiftOpen(true)}
                label={t('career.hub.claimFreePrecon')}
              />
              <div className={hub.heroText}>
                <p className={hub.kicker}>{t('career.track.commander')} · {t('career.hub.freeBadge')}</p>
                <h2 id="hub-next" className={hub.heroName}>{t('career.hub.commanderGiftTitle')}</h2>
                <p className={hub.heroLine}>{t('career.hub.commanderGiftLead')}</p>
                <div className={hub.heroActions}>
                  <Button variant="decision" size="xl" icon={<Gift size={22} />} onClick={() => setGiftOpen(true)} data-nav>
                    {t('career.hub.claimFreePrecon')}
                  </Button>
                  <Button variant="print" size="lg" onClick={() => navigate('/career/shop?cat=commander')} data-nav>
                    {t('career.shop.commanderPacks')}
                  </Button>
                </div>
              </div>
            </>
          ) : (
            <>
              <PortraitCard
                name={commanderCard?.cardName ?? activeCommanderSaved?.name ?? t('career.track.commander')}
                cover={commanderCover}
                size="lg"
                className={[hub.heroCard, motion.deal].join(' ')}
                onPick={() => navigate('/career/deck')}
                label={t('career.deck.edit')}
              />
              <div className={hub.heroText}>
                <p className={hub.kicker}>{t('career.track.commander')} · {t('career.hub.commanderHeroKicker')}</p>
                <h2 id="hub-next" className={hub.heroName}>{activeCommanderSaved?.name ?? t('career.hub.commanderDeckTitle')}</h2>
                <p className={hub.heroLine}>
                  {commanderCard?.cardName ? t('career.hub.commanderLead', { commander: commanderCard.cardName }) : t('career.hub.commanderDeckStart')}
                </p>
                <div className={hub.heroActions}>
                  <Button variant="decision" size="xl" icon={<Crown size={22} />} onClick={() => navigate('/career/opponents?track=commander')} data-nav>
                    {t('career.ladder.track.commander')}
                  </Button>
                  <Button variant="print" size="lg" onClick={() => navigate('/career/ranked?format=commander&queue=pod')} data-nav>
                    {t('career.ranked.queue.pod')}
                  </Button>
                  <Button variant="quiet" size="lg" onClick={() => navigate('/career/deck')} data-nav>
                    {t('career.deck.edit')}
                  </Button>
                </div>
                {deck.commanderDecks.length > 0 && (
                  <div className={hub.deckPlate}>
                    <span>
                      {deck.commanderDecks.length > 1 ? (
                        <select
                          className={hub.deckSelect}
                          aria-label={t('career.decks.choose')}
                          value={activeCommanderSaved?.id ?? ''}
                          onChange={(event) => void deck.selectDeck(event.target.value).catch((error) => notify(t('career.decks.switchFailed'), error instanceof Error ? error.message : String(error), 'error'))}
                          data-nav
                        >
                          {deck.commanderDecks.map((item) => (
                            <option key={item.id} value={item.id}>{item.name}</option>
                          ))}
                        </select>
                      ) : (
                        <b>{activeCommanderSaved?.name ?? t('career.nav.deck')}</b>
                      )}
                      <small>{deck.isCommander && deck.problem ? deck.problem : t('career.hub.commanderCards', { count: ((activeCommanderSaved?.deck?.cards ?? []).reduce((s, c) => s + (c.amount ?? 0), 0)) + ((activeCommanderSaved?.deck?.sideboard ?? []).reduce((s, c) => s + (c.amount ?? 0), 0)) })}</small>
                    </span>
                    <Button variant="quiet" size="sm" onClick={() => navigate('/career/deck')} data-nav>{t('career.deck.edit')}</Button>
                  </div>
                )}
              </div>
            </>
          )
        ) : track === 'pve' ? (
          <>
            <PortraitCard
              name={runOn ? (run?.next?.name ?? t('career.modes.gauntlet')) : t('career.modes.gauntlet')}
              cover={runOn ? (run?.next?.cover ?? HUB_ART.gauntlet) : (challenge.data?.cover ?? HUB_ART.gauntlet)}
              size="lg"
              className={[hub.heroCard, motion.deal].join(' ')}
              onPick={() => navigate(runOn ? '/career/gauntlet' : '/career/hordes')}
              label={t('career.modes.gauntlet')}
            />
            <div className={hub.heroText}>
              <p className={hub.kicker}>{t('career.track.pve')}</p>
              <h2 id="hub-next" className={hub.heroName}>{runOn ? t('career.modes.gauntlet') : t('career.hub.pveHeroTitle')}</h2>
              <p className={hub.heroLine}>
                {runOn
                  ? t('career.hub.gauntletRun', { wins: run?.wins ?? 0 })
                  : t('career.hub.pveHeroLead')}
              </p>
              <div className={hub.heroActions}>
                <Button variant="decision" size="xl" icon={<Swords size={22} />} onClick={() => navigate('/career/gauntlet')} data-nav>
                  {runOn ? t('career.hub.continueRun') : t('career.modes.gauntlet')}
                </Button>
                <Button variant="print" size="lg" onClick={() => navigate('/career/limited')} data-nav>
                  {t('career.modes.limited')}
                </Button>
                <Button variant="quiet" size="lg" onClick={() => navigate('/career/hordes')} data-nav>
                  {t('career.modes.hordes')}
                </Button>
              </div>
            </div>
          </>
        ) : next ? (
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
                  {deck.decks.length > 1 ? (
                    <select
                      className={hub.deckSelect}
                      aria-label={t('career.decks.choose')}
                      value={deck.activeId ?? ''}
                      onChange={(event) => void deck.selectDeck(event.target.value).catch((error) => notify(t('career.decks.switchFailed'), error instanceof Error ? error.message : String(error), 'error'))}
                      data-nav
                    >
                      {deck.decks.map((item) => (
                        <option key={item.id} value={item.id}>{item.name}</option>
                      ))}
                    </select>
                  ) : (
                    <b>{deck.deck?.name ?? t('career.nav.deck')}</b>
                  )}
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
          <div className={hub.questHead}>
            <h2 className={styles.cardLabel}>{t('career.quests')}</h2>
            <div className={hub.questPref} role="group" aria-label={t('career.quests.prefLabel')}>
              {(['any', 'constructed', 'commander'] as const).map((pref) => (
                <button
                  key={pref}
                  type="button"
                  disabled={prefBusy}
                  aria-pressed={questPref === pref}
                  className={[hub.questPrefBtn, questPref === pref ? hub.questPrefOn : ''].join(' ')}
                  onClick={() => void onQuestPref(pref)}
                >
                  {t(`career.quests.pref.${pref}` as MessageKey)}
                </button>
              ))}
            </div>
          </div>
          {quests.isPending ? <p className={styles.small}>{t('career.loading')}</p> : <QuestList quests={quests.data ?? undefined} />}
        </section>
        <section className={[hub.panel, motion.deal].join(' ')} style={dealStyle(4)}>
          <h2 className={styles.cardLabel}>{t('career.weekly')}</h2>
          <WeeklyGoal weekly={weekly.data} />
        </section>
      </aside>

      {versus && <RosterVersus opponent={versus} deck={deck} onClose={() => setVersus(null)} />}
      <PreconGiftDialog
        open={giftOpen}
        onOpenChange={setGiftOpen}
        precons={commanderStarters}
        onClaimed={() => pickTrack('commander')}
      />
    </div>
  );
}

function PreconGiftDialog({ open, onOpenChange, precons, onClaimed }: {
  open: boolean;
  onOpenChange(open: boolean): void;
  precons: readonly CareerStarter[];
  onClaimed(): void;
}) {
  const t = useT();
  const [busy, setBusy] = useState<string | null>(null);

  async function claim(starter: CareerStarter) {
    if (!starter.id || busy) return;
    setBusy(starter.id);
    try {
      await acquirePrecon(starter.id);
      notify(t('career.precon.claimedTitle'), t('career.precon.claimedBody', { name: starter.name ?? '' }));
      onOpenChange(false);
      onClaimed();
    } catch (reason) {
      notify(t('career.failed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBusy(null);
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => !busy && onOpenChange(nextOpen)}
      title={t('career.hub.commanderGiftTitle')}
      description={t('career.hub.commanderGiftLead')}
      width="lg"
    >
      <div className={hub.preconGrid}>
        {precons.map((starter) => (
          <button
            key={starter.id}
            type="button"
            className={hub.preconCard}
            disabled={busy !== null}
            onClick={() => void claim(starter)}
          >
            <CardFace card={{ name: starter.cover?.name ?? starter.name, setCode: starter.cover?.setCode, cardNumber: starter.cover?.cardNumber }} size="normal" />
            <b>{starter.name}</b>
            <small>{busy === starter.id ? t('career.loading') : t('career.starter.cards', { count: starter.cards ?? 100 })}</small>
          </button>
        ))}
      </div>
    </Dialog>
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
  ranked: { name: 'Anax and Cymede', setCode: 'THS', cardNumber: '186' },
  league: { name: 'Heliod, God of the Sun', setCode: 'THS', cardNumber: '17' },
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
