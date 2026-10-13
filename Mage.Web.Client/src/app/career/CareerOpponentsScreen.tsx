import { Award, Crosshair, Gauge, Lock, ScrollText, Skull } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { Navigate, useNavigate } from 'react-router-dom';
import type { AdaptiveView, CareerOpponent, CareerRank } from '../../protocol/generated/views';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { usePlay } from '../stores/play';
import { Button } from '../ui/Button';
import { useCareerOpponents, useCareerState } from './careerData';
import { WINS_TO_UNLOCK, tiersOf } from './careerModel';
import { useAdaptive } from './careerModesData';
import { phaseLines, pipSlots, styleKeys } from './ladderModel';
import { useSceneArt } from './careerScene';
import { nextOpponent } from './hubModel';
import { dealStyle } from './motion';
import { PortraitCard } from './Portraits';
import { RosterVersus } from './rosterPlay';
import { useCareerDeck } from './useCareerDeck';
import motion from './motion.module.css';
import { adaptiveSentence } from '../solo/freePlay';
import styles from './CareerLadder.module.css';

registerMessages(messages);

/** The opponent ladder: each tier a row of portraits, climbed left to right and bottom to top. */
export function CareerOpponentsScreen() {
  const t = useT();
  const navigate = useNavigate();
  const state = useCareerState();
  const profile = state.data?.profile;
  const opponents = useCareerOpponents(!!profile);
  const deck = useCareerDeck(profile);
  const playing = usePlay((play) => play.phase !== 'idle');
  const [versus, setVersus] = useState<CareerOpponent | null>(null);
  const roster = useMemo(() => opponents.data ?? [], [opponents.data]);
  const tiers = useMemo(() => tiersOf(roster), [roster]);
  const next = useMemo(() => nextOpponent(roster), [roster]);
  useSceneArt(next?.cover);
  // the champions are on top; the climb starts at the next opponent's rung, so that's where the ladder opens
  const ladder = useRef<HTMLDivElement>(null);
  const nextId = next?.id;
  useEffect(() => {
    if (nextId) ladder.current?.querySelector('[aria-pressed="true"]')?.scrollIntoView({ block: 'center' });
  }, [nextId]);

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;

  let dealt = 0;
  return (
    <div ref={ladder} className={styles.ladder}>
      <LadderHead rank={state.data?.rank} adaptive={state.data?.adaptive} />
      {deck.problem && (
        <div className={styles.deckWarning} role="status">
          <span>{deck.problem}</span>
          <Button variant="decision" size="sm" onClick={() => navigate('/career/deck')} data-nav>{t('career.deck.fix')}</Button>
        </div>
      )}
      {opponents.isPending && <p className={styles.note}>{t('career.loading')}</p>}
      {opponents.isError && <p className={styles.note} role="alert">{t('career.failed')}</p>}
      {/* the top tier first: the ladder reads upward, toward the champions */}
      {[...tiers].reverse().map((tier, index, all) => {
        const below = all[index + 1];
        const progress = !tier.unlocked
          ? t('career.tier.locked', { needed: WINS_TO_UNLOCK })
          : index > 0 || tiers.length === 1 ? (tier.wins >= WINS_TO_UNLOCK ? t('career.tier.done') : t('career.tier.progress', { wins: tier.wins, needed: WINS_TO_UNLOCK }))
            : null;
        return (
          <section key={tier.tier} className={[styles.tier, tier.unlocked ? '' : styles.tierLocked].join(' ')} aria-labelledby={`tier-${tier.tier}`}>
            <header className={styles.tierHead}>
              <span className={styles.rung} aria-hidden="true">{tier.tier}</span>
              <h2 id={`tier-${tier.tier}`}>{tier.name}</h2>
              {progress && <small>{!tier.unlocked && <Lock size={14} aria-hidden="true" />} {progress}</small>}
              {!tier.unlocked && below && (
                <span className={styles.meter} role="progressbar" aria-valuemin={0} aria-valuemax={WINS_TO_UNLOCK} aria-valuenow={Math.min(below.wins, WINS_TO_UNLOCK)} aria-label={t('career.tier.progress', { wins: Math.min(below.wins, WINS_TO_UNLOCK), needed: WINS_TO_UNLOCK })}>
                  {Array.from({ length: WINS_TO_UNLOCK }, (_, i) => <i key={i} className={i < below.wins ? styles.meterOn : undefined} />)}
                </span>
              )}
            </header>
            <ul className={styles.row}>
              {tier.opponents.map((opponent) => {
                const order = dealt++;
                const locked = !opponent.unlocked;
                return (
                  <li key={opponent.id}>
                    <PortraitCard
                      name={opponent.name ?? ''}
                      tagline={opponent.tagline}
                      colors={opponent.colors}
                      cover={opponent.cover}
                      locked={locked}
                      selected={opponent.id === next?.id}
                      className={motion.deal}
                      style={dealStyle(order)}
                      label={locked
                        ? `${opponent.name}, ${t('career.opponent.locked')}`
                        : `${t('career.hub.face', { opponent: opponent.name ?? '' })}, ${t('career.ladder.pips', { pips: opponent.pips ?? 1 })}`}
                      onPick={locked || playing || !!deck.problem || !deck.deck ? undefined : () => setVersus(opponent)}
                    >
                      <span className={styles.wins}>
                        {t('career.opponent.wins', { count: opponent.wins ?? 0 })}
                      </span>
                      <span className={styles.pays}>+{opponent.winCoins ?? 0}</span>
                      <DifficultyPips pips={opponent.pips} />
                    </PortraitCard>
                    <OpponentFacts opponent={opponent} />
                  </li>
                );
              })}
            </ul>
          </section>
        );
      })}
      {versus && <RosterVersus opponent={versus} deck={deck} onClose={() => setVersus(null)} />}
    </div>
  );
}

/** The ladder's head: the rank badge earned so far, and the adaptive-difficulty switch with how it reads you. */
function LadderHead({ rank, adaptive }: { rank?: CareerRank; adaptive?: AdaptiveView }) {
  const t = useT();
  const on = useAdaptive((state) => state.on);
  const set = useAdaptive((state) => state.set);
  return (
    <header className={styles.ladderHead}>
      <p className={styles.rank}>
        <Award size={20} aria-hidden="true" />
        {rank?.badge
          ? <span><small>{t('career.rank.label')}</small> <b>{rank.badge}</b></span>
          : <span>{t('career.rank.none')}</span>}
      </p>
      <div className={styles.adaptive}>
        <button type="button" role="switch" aria-checked={on} className={styles.switch} onClick={() => set(!on)} data-nav>
          <Gauge size={16} aria-hidden="true" />
          <span>{t('career.adaptive.toggle')}</span>
          <i aria-hidden="true" />
        </button>
        <small>{on ? (adaptiveSentence(t, adaptive) ?? t('career.adaptive.learning')) : t('career.adaptive.off')}</small>
      </div>
    </header>
  );
}

/** 1 to 5 pips: how hard the opponent measured against a starter deck */
function DifficultyPips({ pips }: { pips: number | undefined }) {
  return (
    <span className={styles.difficulty} aria-hidden="true">
      {pipSlots(pips).map((on, index) => <i key={index} className={on ? styles.difficultyOn : undefined} />)}
    </span>
  );
}

/** What sets an opponent apart, as small tags under their portrait: nemesis, rival, raid, phases, style, lists. */
function OpponentFacts({ opponent }: { opponent: CareerOpponent }) {
  const t = useT();
  const phases = phaseLines(opponent.twists);
  const styleTags = styleKeys(opponent.style);
  const lists = opponent.lists ?? 1;
  if (!opponent.nemesis && !opponent.rivalChapter && !opponent.raid && phases.length === 0 && styleTags.length === 0 && lists < 2) return null;
  return (
    <div className={styles.facts}>
      {opponent.nemesis && (
        <p className={styles.nemesis}>
          <Skull size={14} aria-hidden="true" /> <b>{t('career.nemesis.badge')}</b>
          {opponent.bounty ? <span className={styles.pays}> {t('career.nemesis.bounty', { coins: opponent.bounty })}</span> : null}
          {opponent.lines?.nemesis && <q>{opponent.lines.nemesis}</q>}
        </p>
      )}
      {opponent.rivalChapter ? (
        <p className={styles.rival}>
          <ScrollText size={14} aria-hidden="true" /> <b>{t('career.rival.chapter', { chapter: opponent.rivalChapter })}</b>
          {opponent.rivalBefore && <q>{opponent.rivalBefore}</q>}
        </p>
      ) : null}
      {opponent.raid && <p className={styles.raid}><Crosshair size={14} aria-hidden="true" /> {t('career.ladder.raid')}</p>}
      {phases.length > 0 && (
        <details className={styles.phases}>
          <summary>{t('career.ladder.phases', { count: phases.length })}</summary>
          <ul>{phases.map((phase, index) => <li key={index}>{t(phase.key, phase.vars)}</li>)}</ul>
        </details>
      )}
      {(styleTags.length > 0 || lists > 1) && (
        <ul className={styles.tags}>
          {styleTags.map((key) => <li key={key}>{t(key)}</li>)}
          {lists > 1 && <li>{t('career.ladder.lists', { count: lists })}</li>}
        </ul>
      )}
    </div>
  );
}
