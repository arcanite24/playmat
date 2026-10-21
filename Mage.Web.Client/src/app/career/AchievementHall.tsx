import { ChevronDown, HelpCircle, Lock, Trophy } from 'lucide-react';
import { useMemo, useState, type CSSProperties } from 'react';
import type { CareerAchievement, CareerAchievementTier, CareerHall, CareerMastery } from '../../protocol/generated/views';
import { formatNumber, registerMessages, useT, type MessageKey } from '../i18n';
import messages from '../i18n/en/career';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { Art } from './Art';
import { SHOWCASE_SIZE, TIER_NAMES, tierName } from './badges';
import { ProgressBar } from './CareerScreen';
import { useCareerLook, useCareerMasteries } from './careerData';
import {
  badgeChoices, badgeLabel, familiesOf, familyTally, formatShare, hallList, holderShare, isComplete, isHidden, rankName, rankProgress,
  tiersEarned, toggleBadge, type HallFilter,
} from './hallModel';
import { dealStyle } from './motion';
import motion from './motion.module.css';
import { cosmeticReward, describePay, fraction } from './progressModel';
import { BadgeMedal } from './ShowcaseBadges';
import styles from './AchievementHall.module.css';

registerMessages(messages);

/**
 * The achievement hall (B304–B307): points and their rank, the badges on show, what was earned lately, then every
 * achievement by family with its tiers, what the next one pays and how many players have it.
 */
export function AchievementHall({ hall }: { hall: CareerHall }) {
  const t = useT();
  const [filter, setFilter] = useState<HallFilter>('all');
  const achievements = useMemo(() => hall.achievements ?? [], [hall.achievements]);
  const families = useMemo(() => familiesOf(achievements), [achievements]);
  const tally = useMemo(() => familyTally(achievements), [achievements]);
  const shown = useMemo(() => hallList(achievements, filter), [achievements, filter]);
  const names = useMemo(() => new Map(achievements.map((achievement) => [achievement.id ?? '', achievement.name ?? ''])), [achievements]);
  const dates = useMemo(() => new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' }), []);

  return (
    <section className={styles.hall} aria-label={t('career.progress.achievements')}>
      <Points hall={hall} earned={tally.all.earned} total={tally.all.total} />
      <Showcase achievements={achievements} names={names} />
      {(hall.recent?.length ?? 0) > 0 && (
        <section className={styles.recent} aria-labelledby="hall-recent">
          <h3 id="hall-recent" className={styles.label}>{t('career.hall.recent')}</h3>
          <ul className={styles.recentList}>
            {hall.recent!.map((achievement, index) => {
              const tier = achievement.tiers?.length ? tierName(achievement.tier ?? 0) : null;
              return (
                <li key={`${achievement.id}-${achievement.tier}`} className={[styles.recentItem, motion.deal].join(' ')} style={dealStyle(index)} data-tier={tier ?? undefined}>
                  <Art kind="achievement" id={achievement.id ?? ''} size={40} fallback={<Trophy size={22} aria-hidden="true" />} />
                  <span>
                    <b>{achievement.name}</b>
                    <small>{[tier ? t(`career.hall.tier.${tier}`) : null, achievement.at ? dates.format(new Date(achievement.at)) : null].filter(Boolean).join(' · ')}</small>
                  </span>
                </li>
              );
            })}
          </ul>
        </section>
      )}
      <div className={styles.filters} role="group" aria-label={t('career.hall.filter')}>
        {(['all', ...families] as HallFilter[]).map((family) => (
          <button key={family} type="button" className={styles.filter} aria-pressed={filter === family} onClick={() => setFilter(family)} data-nav>
            {family === 'all' ? t('career.hall.all') : t(`career.family.${family}` as MessageKey)}
            <small>{tally[family].earned}/{tally[family].total}</small>
          </button>
        ))}
      </div>
      {shown.length === 0 ? <p className={styles.note}>{t('career.hall.empty')}</p> : (
        <ul className={styles.cards}>
          {shown.map((achievement, index) => (
            <AchievementCard key={achievement.id} achievement={achievement} players={hall.players} index={index} dates={dates} />
          ))}
        </ul>
      )}
    </section>
  );
}

/** The points total, its rank and the bar to the next one. */
function Points({ hall, earned, total }: { hall: CareerHall; earned: number; total: number }) {
  const t = useT();
  const { rank, points, next, fill } = rankProgress(hall);
  const nextName = next === null ? null : rankName(t, rank + 1, hall.rankNames);
  const toNext = next === null ? t('career.hall.topRank') : t('career.hall.toNext', { count: next - points, rank: nextName ?? '' });
  return (
    <header className={styles.points}>
      <span className={styles.pointsTotal}>
        <b>{formatNumber(points)}</b>
        <small>{t('career.hall.kicker')}</small>
      </span>
      <span className={styles.rank}>
        <span className={styles.rankName}>{rankName(t, rank, hall.rankNames)}</span>
        <ProgressBar value={fill} label={toNext} done={next === null} />
        <small>{toNext}</small>
      </span>
      <small className={styles.tally}>{t('career.hall.tiers', { earned, total })}</small>
    </header>
  );
}

/** The badges on show (up to three), and the earned ones to choose from (B307). */
function Showcase({ achievements, names }: { achievements: CareerAchievement[]; names: Map<string, string> }) {
  const t = useT();
  const user = useSession((session) => session.userName);
  const badges = useCareerLook((look) => look.looks[user.toLowerCase()]?.badges);
  const current = useMemo(() => badges ?? [], [badges]);
  const [open, setOpen] = useState(false);
  const masteries = useCareerMasteries(true);
  const choices = useMemo(() => badgeChoices(achievements, masteries.data ?? NO_MASTERIES), [achievements, masteries.data]);
  const full = current.length >= SHOWCASE_SIZE;

  function toggle(id: string) {
    useCareerLook.getState().choose(user, { badges: toggleBadge(current, id) });
  }

  return (
    <section className={styles.showcase} aria-labelledby="hall-showcase">
      <div className={styles.showcaseHead}>
        <h3 id="hall-showcase" className={styles.label}>{t('career.showcase.title')}</h3>
        <ul className={styles.slots}>
          {Array.from({ length: SHOWCASE_SIZE }, (_, index) => {
            const id = current[index];
            return (
              <li key={id ?? `slot-${index}`} className={styles.slot}>
                {id ? (
                  <button type="button" className={styles.slotBadge} onClick={() => toggle(id)} aria-label={t('career.showcase.remove', { name: badgeLabel(t, id, names) })} title={badgeLabel(t, id, names)} data-nav>
                    <BadgeMedal id={id} size={44} />
                  </button>
                ) : <span className={styles.slotEmpty} role="img" aria-label={t('career.showcase.slot')} />}
              </li>
            );
          })}
        </ul>
        <Button variant="quiet" size="sm" onClick={() => setOpen(!open)} aria-expanded={open} data-nav>
          {open ? t('career.showcase.done') : t('career.showcase.choose')}
        </Button>
      </div>
      {open && (
        choices.length === 0 ? <p className={styles.note}>{t('career.showcase.empty')}</p> : (
          <>
            <p className={styles.note}>{full ? t('career.showcase.full') : t('career.showcase.lead', { count: current.length, max: SHOWCASE_SIZE })}</p>
            <ul className={styles.choices}>
              {choices.map((choice) => {
                const on = current.includes(choice.id);
                const label = badgeLabel(t, choice.id, names);
                return (
                  <li key={choice.id}>
                    <button
                      type="button"
                      className={styles.choice}
                      aria-pressed={on}
                      disabled={!on && full}
                      onClick={() => toggle(choice.id)}
                      data-nav
                    >
                      <BadgeMedal id={choice.id} size={36} />
                      <span>{label}</span>
                    </button>
                  </li>
                );
              })}
            </ul>
          </>
        )
      )}
    </section>
  );
}

const NO_MASTERIES: CareerMastery[] = [];

/** One achievement: its emblem, words, the bar to its next tier, tier pips, what the next tier pays and who has it. */
function AchievementCard({ achievement, players, index, dates }: { achievement: CareerAchievement; players?: number; index: number; dates: Intl.DateTimeFormat }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const hidden = isHidden(achievement);
  const complete = isComplete(achievement);
  const earned = tiersEarned(achievement);
  const tiers = achievement.tiers ?? [];
  const part = complete ? 1 : fraction(achievement.progress, achievement.target);
  const share = holderShare(achievement.holders, players);
  const pay = [
    describePay(t, achievement.coins, achievement.xp),
    achievement.cosmetic?.kind && achievement.cosmetic.id ? cosmeticReward(t, achievement.cosmetic) : '',
  ].filter(Boolean).join(' · ');
  const state = hidden ? styles.hidden : complete ? styles.complete : earned > 0 ? styles.started : '';
  const detailsId = `hall-tiers-${achievement.id}`;

  return (
    <li className={[styles.card, state, motion.deal].join(' ')} style={dealStyle(index)}>
      <span className={styles.emblem} style={{ '--part': part } as CSSProperties} aria-hidden="true">
        {hidden
          ? <Art kind="reward" id="secret" size={56} fallback={<HelpCircle size={26} />} />
          : <Art kind="achievement" id={achievement.id ?? ''} size={56} fallback={earned > 0 ? <Trophy size={26} /> : <Lock size={22} />} />}
      </span>
      <span className={styles.body}>
        <b className={styles.name}>{hidden ? t('career.hall.secretName') : achievement.name}</b>
        <small className={styles.text}>{hidden ? t('career.hall.secretHint') : achievement.text}</small>
        {!hidden && !complete && (achievement.target ?? 0) > 1 && (
          <span className={styles.progress}>
            <ProgressBar value={part} label={achievement.text ?? achievement.name ?? ''} />
            <small>{formatNumber(achievement.progress ?? 0)} / {formatNumber(achievement.target ?? 0)}</small>
          </span>
        )}
        {tiers.length > 0 && <TierPips tiers={tiers} earned={earned} players={players} />}
        {!hidden && (complete
          ? <small className={styles.done}>{tiers.length > 0 ? t('career.hall.allTiers') : achievement.at ? t('career.achievements.earned', { date: dates.format(new Date(achievement.at)) }) : null}</small>
          : pay && <span className={styles.pay}>{t(tiers.length > 0 ? 'career.hall.next' : 'career.hall.pays', { pay })}</span>)}
        <span className={styles.foot}>
          <span>{t('career.hall.pointsOf', { points: achievement.points ?? 0, total: achievement.pointsTotal ?? 0 })}</span>
          {share !== null && <span>{share > 0 ? t('career.hall.share', { share: formatShare(share) }) : t('career.hall.shareNone')}</span>}
          {tiers.length > 0 && !hidden && (
            <button type="button" className={styles.more} aria-expanded={open} aria-controls={detailsId} onClick={() => setOpen(!open)} data-nav>
              {t('career.hall.tiersShow')}
              <ChevronDown size={14} aria-hidden="true" />
            </button>
          )}
        </span>
        {open && (
          <ol id={detailsId} className={styles.tierList}>
            {tiers.map((tier, at) => <TierRow key={at} tier={tier} index={at} earned={at < earned} players={players} />)}
          </ol>
        )}
      </span>
    </li>
  );
}

/** A pip per tier, filled once earned, in its metal; hovering one tells how many players have it. */
function TierPips({ tiers, earned, players }: { tiers: CareerAchievementTier[]; earned: number; players?: number }) {
  const t = useT();
  return (
    <span className={styles.pips} role="img" aria-label={t('career.hall.tierCount', { earned, total: tiers.length })}>
      {tiers.map((tier, index) => {
        const name = TIER_NAMES[Math.min(index, TIER_NAMES.length - 1)];
        const share = holderShare(tier.holders, players);
        const title = [t(`career.hall.tier.${name}`), share !== null ? t('career.hall.share', { share: formatShare(share) }) : null].filter(Boolean).join(' · ');
        return <i key={index} data-tier={name} className={index < earned ? styles.pipOn : undefined} title={title} />;
      })}
    </span>
  );
}

function TierRow({ tier, index, earned, players }: { tier: CareerAchievementTier; index: number; earned: boolean; players?: number }) {
  const t = useT();
  const name = TIER_NAMES[Math.min(index, TIER_NAMES.length - 1)];
  const share = holderShare(tier.holders, players);
  const pay = [describePay(t, tier.coins, tier.xp), tier.cosmetic?.kind && tier.cosmetic.id ? cosmeticReward(t, tier.cosmetic) : ''].filter(Boolean).join(' · ');
  return (
    <li className={earned ? styles.tierEarned : undefined} data-tier={name}>
      <i className={styles.tierPip} aria-hidden="true" />
      <b>{t(`career.hall.tier.${name}`)}</b>
      <span>{formatNumber(tier.target ?? 0)}</span>
      <span className={styles.tierPay}>{pay}</span>
      <span>{t('career.hall.tierPoints', { points: tier.points ?? 0 })}</span>
      <span>{share !== null ? formatShare(share) : ''}</span>
    </li>
  );
}
