import { WandSparkles } from 'lucide-react';
import type { CareerLimitedRun, ColorSignal, PairAdvice, PickRecord } from '../../protocol/generated/views';
import { useT, type MessageKey } from '../i18n';
import { Button } from '../ui/Button';
import { useLimitedCoach } from './careerModesData';
import styles from './CareerModes.module.css';

const COLOR_KEYS: Record<string, MessageKey> = {
  W: 'career.color.W', U: 'career.color.U', B: 'career.color.B', R: 'career.color.R', G: 'career.color.G',
};
const GRADE_POINTS: Record<string, number> = { A: 4, B: 3, C: 2, D: 1, F: 0 };

/** the overall letter for a set of graded picks: the average of their points, rounded */
function overallGrade(picks: readonly PickRecord[]): string {
  if (picks.length === 0) return '–';
  const average = picks.reduce((sum, pick) => sum + (GRADE_POINTS[pick.grade ?? 'C'] ?? 2), 0) / picks.length;
  return ['F', 'D', 'C', 'B', 'A'][Math.round(average)];
}

function Mana({ color }: { color: string }) {
  return <i className={`ms ms-cost ms-${color.toLowerCase()}`} aria-hidden="true" />;
}

/** The draft trainer (S019): the grade of the pick just made and what the pod is passing. */
export function DraftCoach({ run }: { run: CareerLimitedRun }) {
  const t = useT();
  const picks = run.picks ?? [];
  const last = picks[picks.length - 1];
  const signals = run.signals ?? [];
  return (
    <div className={styles.coach}>
      {last && (
        <p className={styles.coachPick} aria-live="polite">
          <b className={styles.grade} data-grade={last.grade}>{last.grade}</b>
          <span>
            <b>{t('career.coach.lastPick', { round: last.round ?? 1, pick: last.pick ?? 1, grade: last.chosen ?? '' })}</b>
            {' '}{last.note}
          </span>
        </p>
      )}
      <div className={styles.signals} aria-labelledby="draft-signals">
        <h3 id="draft-signals" className={styles.subhead}>{t('career.coach.signals')}</h3>
        {signals.length === 0
          ? <p className={styles.small}>{t('career.coach.signalsEarly')}</p>
          : (
            <>
              <p className={styles.small}>{t('career.coach.signalsHint')}</p>
              <ul className={styles.signalBars}>
                {signals.map((signal: ColorSignal) => (
                  <li key={signal.color}>
                    <Mana color={signal.color ?? ''} />
                    <span className={styles.srOnly}>{t(COLOR_KEYS[signal.color ?? 'W'])}</span>
                    <span className={styles.signalTrack}><span style={{ width: `${signal.strength ?? 0}%` }} /></span>
                    <b>{signal.strength ?? 0}</b>
                  </li>
                ))}
              </ul>
            </>
          )}
      </div>
    </div>
  );
}

/** After the draft: every pick graded, the overall grade and the picks worth a second look. */
export function DraftReview({ picks }: { picks: readonly PickRecord[] }) {
  const t = useT();
  if (picks.length === 0) return null;
  const count = (grade: string) => picks.filter((pick) => pick.grade === grade).length;
  const worst = picks
    .filter((pick) => pick.grade !== 'A')
    .sort((a, b) => (GRADE_POINTS[a.grade ?? 'C'] ?? 2) - (GRADE_POINTS[b.grade ?? 'C'] ?? 2))
    .slice(0, 8);
  return (
    <details className={styles.review}>
      <summary>
        <b className={styles.grade} data-grade={overallGrade(picks)}>{overallGrade(picks)}</b>
        {t('career.coach.review', { grade: overallGrade(picks) })}
      </summary>
      <p className={styles.small}>{t('career.coach.reviewCounts', { a: count('A'), b: count('B'), c: count('C'), d: count('D'), f: count('F'), count: picks.length })}</p>
      {worst.length === 0
        ? <p className={styles.small}>{t('career.coach.reviewClean')}</p>
        : (
          <>
            <h3 className={styles.subhead}>{t('career.coach.reviewWorst')}</h3>
            <ul className={styles.reviewList}>
              {worst.map((pick) => (
                <li key={`${pick.round}-${pick.pick}`}>
                  <b className={styles.grade} data-grade={pick.grade}>{pick.grade}</b>
                  <span>
                    {t('career.coach.pickLine', { round: pick.round ?? 1, pick: pick.pick ?? 1, chosen: pick.chosen ?? '' })}. {pick.note}
                  </span>
                </li>
              ))}
            </ul>
          </>
        )}
    </details>
  );
}

/** The sealed pool coach (S020): the two best colour pairs with their reasons, and a build of either to start from. */
export function PoolCoach({ run, onBuild }: { run: CareerLimitedRun; onBuild(pair: PairAdvice): void }) {
  const t = useT();
  const coach = useLimitedCoach(run.id ?? '', true);
  const pairs = (coach.data?.pairs ?? []).slice(0, 2);
  const curveLabels = ['1', '2', '3', '4', '5', '6+'];
  return (
    <section className={styles.coachPanel} aria-labelledby="pool-coach">
      <h3 id="pool-coach" className={styles.subhead}>{t('career.coach.title')}</h3>
      <p className={styles.small}>{t('career.coach.hint')}</p>
      {coach.isPending && <p className={styles.small}>{t('career.coach.loading')}</p>}
      {coach.isError && <p className={styles.warn}>{t('career.coach.failed')}</p>}
      <ol className={styles.pairs}>
        {pairs.map((pair) => {
          const [first, second] = [...(pair.colors ?? '')];
          const curve = pair.curve ?? [];
          const tallest = Math.max(1, ...curve);
          return (
            <li key={pair.colors} className={styles.pair}>
              <header className={styles.pairHead}>
                <b>
                  <Mana color={first ?? ''} /><Mana color={second ?? ''} />
                  {' '}{t('career.coach.pair', { first: t(COLOR_KEYS[first ?? 'W']), second: t(COLOR_KEYS[second ?? 'U']) })}
                </b>
                <span className={styles.small}>{t('career.coach.score', { score: pair.score ?? 0 })}</span>
              </header>
              <ul className={styles.reasons}>
                {(pair.reasons ?? []).map((reason) => <li key={reason}>{reason}</li>)}
              </ul>
              <div className={styles.curve} role="img" aria-label={`${t('career.coach.curve')}: ${curve.map((n, index) => `${curveLabels[index]}: ${n}`).join(', ')}`}>
                {curve.map((n, index) => (
                  <span key={index}>
                    <i style={{ height: `${(n / tallest) * 100}%` }} />
                    <small>{curveLabels[index]}</small>
                  </span>
                ))}
              </div>
              <Button variant="quiet" size="sm" icon={<WandSparkles size={16} />} onClick={() => onBuild(pair)}>{t('career.coach.build')}</Button>
            </li>
          );
        })}
      </ol>
    </section>
  );
}
