import { CalendarCheck, ChevronRight, Package, Swords, Target } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { useCareerDaily, useCareerLevels, useCareerQuests, useCareerState, useCareerWeekly } from './careerData';
import { describeReward, fraction, rewardParts } from './progressModel';
import { dailySteps, nextLevelReward, packsPath } from './todayModel';
import styles from './TodayPanel.module.css';

registerMessages(messages);

/**
 * Today (B278): the day's quests, the daily wins ladder, the weekly goal, packs to open and the next level's reward,
 * each leading somewhere to make progress. On Home and in the progress drawer.
 */
export default function TodayPanel({ onNavigate }: { onNavigate?: () => void }) {
  const t = useT();
  const navigate = useNavigate();
  const state = useCareerState();
  const profile = state.data?.profile;
  const on = !!profile;
  const quests = useCareerQuests(on);
  const daily = useCareerDaily(on);
  const weekly = useCareerWeekly(on);
  const levels = useCareerLevels(on);
  if (!profile) return null;
  const go = (path: string) => {
    onNavigate?.();
    navigate(path);
  };
  const steps = dailySteps(daily.data);
  const reward = nextLevelReward(levels.data ?? undefined);
  const packs = profile.packTokens ?? 0;
  const week = weekly.data;

  return (
    <section className={styles.today} aria-labelledby="today-title">
      <h2 id="today-title" className={styles.title}>{t('career.today.title')}</h2>

      <div className={styles.block}>
        <h3 className={styles.head}><Target size={15} aria-hidden="true" /> {t('career.today.quests')}</h3>
        {(quests.data?.quests?.length ?? 0) === 0
          ? <p className={styles.note}>{t('career.quests.none')}</p>
          : (
            <ul className={styles.quests}>
              {quests.data!.quests!.map((quest) => (
                <li key={quest.slot}>
                  <span className={styles.questText}>{quest.text}</span>
                  <span className={styles.bar} aria-hidden="true"><i style={{ width: `${fraction(quest.progress, quest.target) * 100}%` }} /></span>
                  <span className={styles.count}>{quest.progress ?? 0}/{quest.target ?? 0}</span>
                </li>
              ))}
            </ul>
          )}
      </div>

      {steps.length > 0 && (
        <div className={styles.block}>
          <h3 className={styles.head}><CalendarCheck size={15} aria-hidden="true" /> {t('career.today.daily')}</h3>
          <ol className={styles.ladder} aria-label={t('career.today.dailyLabel', { wins: daily.data?.wins ?? 0, count: steps.length })}>
            {steps.map((step, index) => (
              <li key={index} className={[styles.step, step.paid ? styles.paid : '', step.next ? styles.next : ''].join(' ')}
                title={t('career.today.dailyStep', { n: index + 1, coins: step.coins })}>
                {step.coins}
              </li>
            ))}
          </ol>
          {!daily.data?.firstWinPaid && (daily.data?.firstWinXp ?? 0) > 0 && (
            <p className={styles.note}>{t('career.today.firstWin', { xp: daily.data!.firstWinXp! })}</p>
          )}
        </div>
      )}

      {week && (
        <div className={styles.block}>
          <h3 className={styles.head}><Swords size={15} aria-hidden="true" /> {t('career.weekly')}</h3>
          <p className={styles.note}>
            {t('career.weekly.wins', { count: week.wins ?? 0 })}{' · '}
            {week.nextGoal ? t('career.weekly.next', { goal: week.nextGoal }) : t('career.weekly.done')}
          </p>
        </div>
      )}

      {reward && (
        <p className={styles.note}>{t('career.today.nextLevel', { level: reward.level ?? 0, reward: describeReward(t, rewardParts(reward)) })}</p>
      )}

      <div className={styles.actions}>
        {packs > 0 && (
          <button type="button" className={styles.action} onClick={() => go(packsPath(profile))}>
            <Package size={16} aria-hidden="true" />
            <span>{t('career.hud.packs', { count: packs })}</span>
            <ChevronRight size={16} aria-hidden="true" />
          </button>
        )}
        <button type="button" className={styles.action} onClick={() => go('/career')}>
          <span>{t(profile.collection ? 'career.today.openCareer' : 'career.today.startCareer')}</span>
          <ChevronRight size={16} aria-hidden="true" />
        </button>
      </div>
    </section>
  );
}
