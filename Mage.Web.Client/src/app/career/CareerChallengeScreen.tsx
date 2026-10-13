import { CalendarDays, Swords } from 'lucide-react';
import { useState } from 'react';
import { Navigate } from 'react-router-dom';
import type { CareerChallenge } from '../../protocol/generated/views';
import { api } from '../connection';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { usePlay } from '../stores/play';
import { Button } from '../ui/Button';
import { useCareerState } from './careerData';
import { playMode, useChallenge } from './careerModesData';
import { twistLines } from './careerModesModel';
import { challengeBoardMode, challengeRuleKey } from './ladderModel';
import { ScoreBoard } from './ScoreBoard';
import { useSceneArt } from './careerScene';
import { PortraitCard } from './Portraits';
import { Versus } from './Versus';
import { LineList, ModeResult } from './ModeParts';
import styles from './CareerModes.module.css';

registerMessages(messages);

/** The weekly challenge: the same fixed decks for everyone this week, and a board of who won it fastest. */
export function CareerChallengeScreen() {
  const t = useT();
  const state = useCareerState();
  const profile = state.data?.profile;
  const challenge = useChallenge(!!profile);

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;
  return (
    <div className={styles.page}>
      <div className={styles.scroll}>
        <ModeResult kind="challenge" />
        {challenge.isPending && <p className={styles.note}>{t('career.loading')}</p>}
        {challenge.isError && <p className={styles.error} role="alert">{t('career.failed')}</p>}
        {challenge.data && <Challenge challenge={challenge.data} />}
      </div>
    </div>
  );
}

function Challenge({ challenge }: { challenge: CareerChallenge }) {
  const t = useT();
  const playing = usePlay((play) => play.phase !== 'idle');
  const [versus, setVersus] = useState(false);
  const twists = twistLines(challenge.twists);
  const rule = t(challengeRuleKey(challenge.rule));
  useSceneArt(challenge.cover);

  return (
    <>
      <header className={styles.modeHead}>
        <div>
          <p className={styles.kicker}><CalendarDays size={16} aria-hidden="true" /> {t('career.challenge.week', { week: challenge.week ?? '' })}</p>
          <h1 className={styles.modeTitle}>{challenge.name}</h1>
          <p className={styles.lead}>{challenge.text}</p>
        </div>
      </header>
      <div className={styles.split}>
        <article className={styles.detail} aria-labelledby="challenge-opponent">
          <div className={styles.detailArt}>
            <PortraitCard name={challenge.opponentName ?? ''} cover={challenge.cover} boss size="sm" />
          </div>
          <div className={styles.detailBody}>
            <header className={styles.detailHead}>
              <h2 id="challenge-opponent">{challenge.opponentName}</h2>
            </header>
            <p className={styles.small}>
              {t('career.node.skill', { skill: challenge.skill ?? 1 })} · {t('career.challenge.fixed')}
              {challenge.format ? ` · ${t('career.challenge.format', { format: challenge.format })}` : ''}
            </p>
            <p className={styles.rule}>{rule}</p>
            {twists.length > 0 && (
              <>
                <h3 className={styles.subhead}>{t('career.twist.title')}</h3>
                <LineList lines={twists} className={styles.twists} />
              </>
            )}
            <h3 className={styles.subhead}>{t('career.challenge.yours')}</h3>
            <p className={styles.small}>
              {(challenge.attempts ?? 0) === 0
                ? t('career.challenge.notYet')
                : challenge.won
                  ? t('career.challenge.best', { turns: challenge.bestTurns ?? 0, life: challenge.bestLife ?? 0, count: challenge.attempts ?? 0 })
                  : t('career.challenge.noWin', { count: challenge.attempts ?? 0 })}
            </p>
            <div className={styles.actions}>
              <Button variant="decision" size="lg" icon={<Swords size={18} />} disabled={playing} onClick={() => setVersus(true)} data-nav>
                {t((challenge.attempts ?? 0) > 0 ? 'career.challenge.again' : 'career.challenge.play')}
              </Button>
            </div>
          </div>
        </article>
        <ScoreBoard mode={challengeBoardMode(challenge.boardMode)} board={challenge.week ?? ''} title={t('career.challenge.board')} />
      </div>
      {versus && (
        <Versus
          kicker={t('career.challenge.week', { week: challenge.week ?? '' })}
          opponent={{ name: challenge.opponentName ?? '', cover: challenge.cover, boss: true }}
          deck={{ name: challenge.name ?? '' }}
          stakes={[t('career.challenge.fixed'), ...twists.map((line) => t(line.key, line.vars)), rule]}
          onFight={() => playMode('challenge', () => api.careerChallengePlay())}
          onClose={() => setVersus(false)}
        />
      )}
    </>
  );
}
