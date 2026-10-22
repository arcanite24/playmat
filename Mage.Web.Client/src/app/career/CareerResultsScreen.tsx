import { useQuery } from '@tanstack/react-query';
import { Coins, Gift, GraduationCap, Package, Sparkles, Trophy, Unlock } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { Navigate, useNavigate, useSearchParams } from 'react-router-dom';
import type { CareerGameResult } from '../../protocol/generated/views';
import { api } from '../connection';
import { formatNumber, registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { refreshCareer, useCareerOpponents, useCareerState } from './careerData';
import { useCampaigns } from './careerModesData';
import { achievementMoment, useCeremonies } from './ceremonies';
import { markSeen } from './hubModel';
import { useSceneArt } from './careerScene';
import { playCareerCue } from './careerSound';
import { DoneMark, ProgressBar, WeeklyGoal } from './CareerScreen';
import { Art } from './Art';
import { dealStyle, useCountUp, useSequence, useStill } from './motion';
import { PortraitCard, type ArtCard } from './Portraits';
import { describePay, describeReward, fetchResult, fraction, momentLine, resultIsEmpty, resultTier, rewardParts, safeBack, xpSweep } from './progressModel';
import motion from './motion.module.css';
import modeStyles from './CareerModes.module.css';
import styles from './Results.module.css';

registerMessages(messages);

/**
 * After a Career match (or a regular game against the AI that counts): the result told as a short scene. The verdict
 * stamps down, coins count up, the XP bar sweeps, then quests and rewards deal in; anything can be skipped.
 * Nothing shows during the match; the server applies the result as the game ends, and this screen asks for it.
 */
export function CareerRewardsScreen() {
  const t = useT();
  const [params] = useSearchParams();
  const key = params.get('game');
  const career = params.get('career') === '1';
  const back = safeBack(params.get('back'));
  const state = useCareerState();
  const profile = state.data?.profile;
  // every game counts now; a player who hid progression skips the screen (B312)
  const counts = career || profile?.showProgress !== false;
  const result = useQuery({
    queryKey: ['careerResult', key ?? 'latest'],
    queryFn: () => fetchResult(() => api.careerGameResult(key)),
    enabled: !!profile && counts,
    staleTime: Infinity,
    retry: false,
  });
  const ready = result.isSuccess && !!result.data;
  useEffect(() => {
    // the payout, quests, level and weekly goal have all moved
    if (ready) refreshCareer();
  }, [ready]);

  // nothing for this screen: straight on
  const skip = state.isError || (state.isSuccess && (!profile || !counts)) || (result.isSuccess && !career && (!result.data || resultIsEmpty(result.data)));
  if (skip) return <Navigate to={back} replace />;

  return (
    <div className={styles.page}>
      {ready ? <Results result={result.data!} back={back} /> : (
        <div className={styles.waiting}>
          <p>{result.isSuccess ? t('career.rewards.none') : t('career.rewards.waiting')}</p>
          {result.isSuccess && <ContinueButton back={back} />}
        </div>
      )}
    </div>
  );
}

function ContinueButton({ back }: { back: string }) {
  const t = useT();
  const navigate = useNavigate();
  return <Button variant="decision" size="lg" autoFocus onClick={() => navigate(back, { replace: true })} data-nav>{t('career.rewards.continue')}</Button>;
}

/** Who the game was against, as a face and their last word: a roster opponent, or a campaign duel's closing line. */
function useFoe(result: CareerGameResult): { name: string; colors?: string; cover?: ArtCard | null; line?: string } | null {
  const roster = useCareerOpponents(!!result.career && !!result.opponent);
  const campaign = result.mode?.kind === 'campaign';
  const campaigns = useCampaigns(campaign);
  return useMemo(() => {
    if (campaign && result.mode?.ref) {
      const [campaignId, nodeId] = result.mode.ref.split('/');
      const owner = campaigns.data?.find((item) => item.id === campaignId);
      const chapter = owner?.chapters?.find((item) => item.nodes?.some((node) => node.id === nodeId));
      const node = chapter?.nodes?.find((item) => item.id === nodeId);
      if (node) {
        // a Five Paths chapter is its opponents' colour; a school's act colour says nothing about the deck across the table
        const colors = owner?.school ? undefined : chapter?.color ?? owner?.colors;
        return { name: node.opponentName ?? node.name ?? '', colors, cover: node.cover, line: result.won ? node.after : undefined };
      }
    }
    const opponent = roster.data?.find((candidate) => candidate.id === result.opponent);
    if (opponent) return { name: opponent.name ?? '', colors: opponent.colors, cover: opponent.cover, line: result.won ? opponent.lines?.lose : opponent.lines?.win };
    return null;
  }, [campaign, campaigns.data, roster.data, result]);
}

// how long each beat holds before the next one deals in: verdict, coins, XP, quests, rewards
const BEATS = [900, 900, 1200, 700, 700] as const;

function Results({ result, back }: { result: CareerGameResult; back: string }) {
  const t = useT();
  const navigate = useNavigate();
  const reduced = useStill();
  const [skipped, setSkipped] = useState(false);
  const still = reduced || skipped;
  const { step, done, skip } = useSequence(BEATS, reduced);
  const foe = useFoe(result);
  useSceneArt(foe?.cover);
  const won = !!result.won;
  const quests = result.quests ?? [];
  const moments = result.moments ?? [];
  const achievements = useMemo(() => result.achievements ?? [], [result.achievements]);
  const levelRewards = result.levelRewards ?? [];
  const opened = useMemo(() => result.opened ?? [], [result.opened]);
  // a school's graduation: its title and sleeve
  const cosmetics = useMemo(() => result.mode?.cosmetics ?? [], [result.mode]);
  const packs = result.packs ?? 0;
  const coins = useCountUp(result.coins ?? 0, { run: step >= 1, still });

  const skipAll = useCallback(() => {
    setSkipped(true);
    skip();
  }, [skip]);

  // each beat's sound as it lands
  useEffect(() => {
    if (step === 0) playCareerCue(won ? 'win' : 'loss');
    if (step === 1 && (result.coins ?? 0) > 0) playCareerCue('coins');
    if (step === 2 && (result.xp ?? 0) > 0) playCareerCue('xp');
    if (step === 4 && (achievements.length > 0 || levelRewards.length > 0 || opened.length > 0 || cosmetics.length > 0)) playCareerCue('unlock');
    // only on a step change: the rest is read once per result
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [step]);

  // each achievement earned gets its own moment once the scene is told; the hub won't announce it again
  const user = useSession((session) => session.userName);
  useEffect(() => {
    if (!done || achievements.length === 0) return;
    markSeen(user, achievements.map((achievement) => `achievement:${achievement.id}`));
    useCeremonies.getState().show(achievements.map((achievement) => achievementMoment(t, achievement)));
  }, [done, achievements, user, t]);

  // Space or Enter tells the rest at once, before the Continue button takes them
  useEffect(() => {
    if (done) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== ' ' && event.key !== 'Enter') return;
      event.preventDefault();
      // Space presses a button on its release: keep this one from pressing Continue, which has the focus by then
      const swallow = (up: KeyboardEvent) => {
        if (up.key === event.key) up.preventDefault();
      };
      window.addEventListener('keyup', swallow, { capture: true, once: true });
      skipAll();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [done, skipAll]);

  const headline = result.mode?.trial
    ? t(won ? 'career.rewards.trialWon' : 'career.rewards.trialLost', { trial: foe?.name ?? '' })
    : result.career
    ? t(won ? 'career.rewards.won' : 'career.rewards.lost', { opponent: foe?.name ?? result.opponent ?? '' })
    : t(won ? 'career.rewards.wonAi' : 'career.rewards.lostAi');

  return (
    <section className={[styles.results, still ? motion.still : ''].join(' ')} aria-labelledby="career-results-title">
      <header className={[styles.banner, won ? styles.won : styles.lost].join(' ')}>
        {foe && (
          <PortraitCard name={foe.name} colors={foe.colors} cover={foe.cover} size="sm" className={[styles.foe, won ? styles.foeBeaten : ''].join(' ')} />
        )}
        <div className={styles.verdict}>
          <h1 id="career-results-title" className={[styles.stamp, motion.stamp].join(' ')}>{t(won ? 'career.results.victory' : 'career.results.defeat')}</h1>
          <p className={styles.headline}>{headline}</p>
          {foe?.line && <blockquote className={styles.line}>“{foe.line}”</blockquote>}
          {result.note ? <p className={styles.note}>{result.note}</p> : !result.career && <p className={styles.note}>{t('career.rewards.aiNote')}</p>}
        </div>
      </header>

      {/* a roster duel's or a horde's own summary: the nemesis paid back, the turns lived */}
      {step >= 1 && (result.mode?.kind === 'duel' || result.mode?.kind === 'horde') && ((result.mode.lines ?? []).length > 0 || result.mode.title) && (
        <section className={[styles.block, motion.deal].join(' ')} aria-label={result.mode.title || t('career.modes.lastGame')}>
          {result.mode.title && <h2 className={styles.label}>{result.mode.title}</h2>}
          {(result.mode.lines ?? []).length > 0 && (
            <ul className={modeStyles.lines}>
              {(result.mode.lines ?? []).map((line, index) => <li key={index}>{line}</li>)}
            </ul>
          )}
        </section>
      )}

      {step >= 1 && (
        <div className={[styles.pay, motion.deal].join(' ')}>
          <span className={styles.coins}>
            <Art kind="reward" id="coins" size={40} fallback={<Coins size={26} aria-hidden="true" />} />
            <b aria-live="off">+{formatNumber(coins)}</b>
            <span className={styles.srOnly}>{t('career.coins', { count: result.coins ?? 0 })}</span>
          </span>
          <span className={styles.xp}>
            <Art kind="reward" id="xp" size={36} fallback={<Sparkles size={22} aria-hidden="true" />} />
            <b>+{t('career.reward.xp', { xp: result.xp ?? 0 })}</b>
          </span>
        </div>
      )}

      {step >= 2 && <XpSweep result={result} still={still} />}

      {/* the game's moments: a big hit, a storm turn, a comeback (B309) */}
      {step >= 3 && moments.length > 0 && (
        <section className={[styles.block, motion.deal].join(' ')} aria-label={t('career.moments.title')}>
          <h2 className={styles.label}>{t('career.moments.title')}</h2>
          <ul className={styles.moments}>
            {moments.map((moment, index) => <li key={`${moment.id}-${index}`}>{momentLine(t, moment)}</li>)}
          </ul>
        </section>
      )}

      {step >= 3 && quests.length > 0 && (
        <section className={styles.block} aria-label={t('career.rewards.quests')}>
          <h2 className={styles.label}>{t('career.rewards.quests')}</h2>
          <ul className={styles.quests}>
            {quests.map((quest, index) => (
              <li key={quest.id} className={[styles.quest, quest.completed ? styles.questDone : '', motion.deal].join(' ')} style={dealStyle(index)}>
                <span>
                  <b>{quest.text}</b>
                  <small>{quest.completed ? describePay(t, quest.coins, quest.xp) : `${quest.before} → ${quest.after} / ${quest.target}`}</small>
                </span>
                {quest.completed ? <DoneMark label={t('career.rewards.questDone')} /> : <span />}
                <ProgressBar value={fraction(quest.after, quest.target)} label={quest.text ?? ''} done={quest.completed} />
              </li>
            ))}
          </ul>
        </section>
      )}

      {step >= 4 && (achievements.length > 0 || levelRewards.length > 0 || opened.length > 0 || cosmetics.length > 0) && (
        <section className={styles.block} aria-label={t('career.rewards.achievements')}>
          <h2 className={styles.label}>{t('career.results.earned')}</h2>
          <ul className={styles.prizes}>
            {achievements.map((achievement, index) => {
              const tier = resultTier(achievement);
              return (
              <li key={`${achievement.id}-${achievement.tier}`} className={[styles.prize, motion.deal].join(' ')} style={dealStyle(index)} data-tier={tier ?? undefined}>
                <Art kind="achievement" id={achievement.id ?? ''} size={56} fallback={<Trophy size={28} aria-hidden="true" />} />
                {tier && <small className={styles.prizeTier}><i aria-hidden="true" />{t('career.results.tier', { tier: t(`career.hall.tier.${tier}`) })}</small>}
                <b>{achievement.name}</b>
                <small>{achievement.text}</small>
                <small className={styles.prizePay}>
                  {[describePay(t, achievement.coins, achievement.xp), achievement.cosmetic ? describeReward(t, [{ kind: 'cosmetic', cosmetic: achievement.cosmetic }]) : ''].filter(Boolean).join(' · ')}
                </small>
              </li>
              );
            })}
            {levelRewards.map((reward, index) => (
              <li key={`level-${reward.level}`} className={[styles.prize, styles.prizeLevel, motion.deal].join(' ')} style={dealStyle(achievements.length + index)}>
                <Art kind="reward" id="level" size={56} fallback={<Gift size={28} aria-hidden="true" />} />
                <b>{t('career.level', { level: reward.level ?? 0 })}</b>
                <small className={styles.prizePay}>{describeReward(t, rewardParts(reward))}</small>
              </li>
            ))}
            {cosmetics.map((cosmetic, index) => (
              <li key={`cosmetic-${cosmetic.kind}-${cosmetic.id}`} className={[styles.prize, motion.deal].join(' ')} style={dealStyle(index)}>
                <Art kind="reward" id="graduate" size={56} fallback={<GraduationCap size={28} aria-hidden="true" />} />
                <b>{t('career.results.graduated')}</b>
                <small className={styles.prizePay}>{describeReward(t, [{ kind: 'cosmetic', cosmetic }])}</small>
              </li>
            ))}
            {/* a tier the win opened; the hub gives it its full moment on the way back */}
            {opened.map((name, index) => (
              <li key={`opened-${name}`} className={[styles.prize, styles.prizeLevel, motion.deal].join(' ')} style={dealStyle(achievements.length + levelRewards.length + index)}>
                <Art kind="reward" id="unlock" size={56} fallback={<Unlock size={28} aria-hidden="true" />} />
                <b>{name}</b>
                <small className={styles.prizePay}>{t('career.ceremony.tier')}</small>
              </li>
            ))}
          </ul>
        </section>
      )}

      {done && result.weekly && (
        <section className={[styles.block, motion.deal].join(' ')} aria-label={t('career.weekly')}>
          <h2 className={styles.label}>{t('career.weekly')}</h2>
          <WeeklyGoal weekly={result.weekly} />
        </section>
      )}

      <footer className={styles.foot}>
        {done ? (
          <>
            {packs > 0 && (
              <Button variant="print" icon={<Package size={16} />} onClick={() => navigate('/career/shop', { replace: true })} data-nav>
                {t('career.results.openPacks', { count: packs })}
              </Button>
            )}
            <ContinueButton back={back} />
          </>
        ) : (
          <Button variant="quiet" onClick={skipAll}>{t('career.results.skip')}</Button>
        )}
      </footer>
    </section>
  );
}

/** The XP bar, filling from where it stood before the game to where it stands now, one level at a time. */
function XpSweep({ result, still }: { result: CareerGameResult; still: boolean }) {
  const t = useT();
  const segments = useMemo(() => xpSweep(result), [result]);
  const [index, setIndex] = useState(0);
  const [filled, setFilled] = useState(false);
  const shown = still ? segments.length - 1 : Math.min(index, segments.length - 1);
  const segment = segments[shown];
  const duration = Math.max(300, Math.round(900 * (segment.to - segment.from)));

  useEffect(() => {
    if (still) return;
    if (!filled) {
      // a breath before each stretch; level-ups follow on at once
      const start = setTimeout(() => setFilled(true), index === 0 ? 120 : 60);
      return () => clearTimeout(start);
    }
    if (index >= segments.length - 1) return;
    const next = setTimeout(() => {
      setIndex(index + 1);
      setFilled(false);
      playCareerCue('levelUp');
    }, duration + 160);
    return () => clearTimeout(next);
  }, [still, filled, index, segments.length, duration]);

  const width = still || filled ? segment.to : segment.from;
  const levelledUp = segment.level > (segments[0]?.level ?? segment.level);
  return (
    <div className={[styles.sweep, motion.deal].join(' ')}>
      <p className={styles.sweepHead}>
        <b>{t('career.level', { level: segment.level })}</b>
        {levelledUp && <span key={segment.level} className={styles.levelUp}>{t('career.rewards.levelUp', { level: segment.level })}</span>}
        <small>{result.xpForNext ? t('career.xp', { into: result.xpIntoLevel ?? 0, needed: result.xpForNext }) : t('career.xpMax')}</small>
      </p>
      <div className={styles.xpBar} role="progressbar" aria-valuemin={0} aria-valuemax={result.xpForNext || 1} aria-valuenow={result.xpIntoLevel ?? 0} aria-label={t('career.level', { level: result.levelAfter ?? 1 })}>
        <span className={filled && !still ? styles.sweepFill : undefined} style={{ width: `${width * 100}%`, transitionDuration: `${duration}ms` }} />
      </div>
    </div>
  );
}
