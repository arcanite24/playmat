import { Shuffle } from 'lucide-react';
import { useMemo, useState } from 'react';
import { countsByName, gradeDrill, setCount, total, type DrillGrade } from '../../core/decks/sideboardPlans';
import type { PlanCard, SideboardPlan } from '../../core/decks/types';
import { formatNumber, useT } from '../i18n';
import { RichText } from '../i18n/RichText';
import { Button } from '../ui/Button';
import { DeckSelect } from './DeckSelect';
import { useDeckList } from './trainData';
import styles from './Train.module.css';

/**
 * The sideboard drill (S018): a kind of deck across the table, your main deck and sideboard, and the question of what
 * you'd swap. The answer is graded against the plan you saved on the deck page.
 */
export function SideboardDrill() {
  const t = useT();
  const [deckId, setDeckId] = useState<string | null>(null);
  const { deck, error } = useDeckList(deckId);
  const plans = useMemo(() => (deck?.sideboardPlans ?? []).filter((plan) => plan.in.length + plan.out.length > 0), [deck]);
  const [planId, setPlanId] = useState<string | null>(null);
  const plan = plans.find((candidate) => candidate.id === planId) ?? null;
  const [answerIn, setAnswerIn] = useState<PlanCard[]>([]);
  const [answerOut, setAnswerOut] = useState<PlanCard[]>([]);
  const [grade, setGrade] = useState<DrillGrade | null>(null);

  const main = useMemo(() => [...countsByName(deck?.cards ?? [])].sort((a, b) => a[0].localeCompare(b[0])), [deck]);
  const side = useMemo(() => [...countsByName(deck?.sideboard ?? [])].sort((a, b) => a[0].localeCompare(b[0])), [deck]);

  function next() {
    if (plans.length === 0) return;
    const others = plans.length > 1 ? plans.filter((candidate) => candidate.id !== planId) : plans;
    const picked = others[Math.floor(Math.random() * others.length)];
    setPlanId(picked.id);
    setAnswerIn([]);
    setAnswerOut([]);
    setGrade(null);
  }

  return (
    <section className={styles.tool} aria-labelledby="drill-title">
      <header className={styles.toolHead}>
        <h2 id="drill-title">{t('train.drill.title')}</h2>
        <p>{t('train.drill.lead')}</p>
      </header>
      <div className={styles.controls}>
        <DeckSelect value={deckId} onChange={(id) => { setDeckId(id); setPlanId(null); setGrade(null); }} />
        <Button variant="decision" icon={<Shuffle size={16} />} disabled={plans.length === 0} onClick={next}>{t(plan ? 'train.drill.another' : 'train.drill.start')}</Button>
      </div>
      {error && <p className={styles.error} role="alert">{error}</p>}
      {deck && plans.length === 0 && (
        <p className={styles.quiet}>{t('train.drill.noPlans')}</p>
      )}
      {plan && (
        <>
          <p className={styles.agree}><b>{t('train.drill.question', { archetype: plan.archetype })}</b> {t('train.drill.change')}</p>
          <div className={styles.cards}>
            <Picker title={t('train.drill.in', { count: total(answerIn) })} cards={side} chosen={answerIn} disabled={!!grade} onCount={(name, count) => setAnswerIn((current) => setCount(current, name, count))} />
            <Picker title={t('train.drill.out', { count: total(answerOut) })} cards={main} chosen={answerOut} disabled={!!grade} onCount={(name, count) => setAnswerOut((current) => setCount(current, name, count))} />
          </div>
          {!grade ? (
            <div className={styles.controls}>
              <Button variant="decision" onClick={() => setGrade(gradeDrill(plan, answerIn, answerOut))}>{t('train.drill.check')}</Button>
            </div>
          ) : (
            <Graded plan={plan} grade={grade} />
          )}
        </>
      )}
    </section>
  );
}

function Picker({ title, cards, chosen, disabled, onCount }: { title: string; cards: [string, number][]; chosen: readonly PlanCard[]; disabled: boolean; onCount(name: string, count: number): void }) {
  const t = useT();
  const counts = new Map(chosen.map((card) => [card.name, card.count]));
  return (
    <div>
      <h3 className={styles.quiet}>{title}</h3>
      <div className={styles.cards}>
        {cards.map(([name, available]) => {
          const count = counts.get(name) ?? 0;
          return (
            <div key={name} className={styles.pick}>
              <span>{name}</span>
              <span className={styles.stepper}>
                <button type="button" aria-label={t('train.drill.fewer', { name })} disabled={disabled || count === 0} onClick={() => onCount(name, count - 1)}>−</button>
                <b>{count}</b>
                <button type="button" aria-label={t('train.drill.more', { name })} disabled={disabled || count >= available} onClick={() => onCount(name, count + 1)}>+</button>
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function Graded({ plan, grade }: { plan: SideboardPlan; grade: DrillGrade }) {
  const t = useT();
  const list = (cards: PlanCard[]) => cards.map((card) => `${card.count} ${card.name}`).join(', ');
  const exact = grade.score >= 0.999;
  return (
    <div className={styles.result} aria-live="polite">
      <p className={exact ? styles.agree : styles.disagree}>
        <b>{t('train.drill.score', { percent: formatNumber(grade.score, { style: 'percent', maximumFractionDigits: 0 }) })}</b> {t(exact ? 'train.drill.exact' : 'train.drill.differ')}
      </p>
      <ul className={styles.quiet}>
        {grade.missedIn.length > 0 && <li><RichText text={t('train.drill.missedIn')} parts={{ cards: <span className={styles.good}>{list(grade.missedIn)}</span> }} /></li>}
        {grade.extraIn.length > 0 && <li><RichText text={t('train.drill.extraIn')} parts={{ cards: <span className={styles.bad}>{list(grade.extraIn)}</span> }} /></li>}
        {grade.missedOut.length > 0 && <li><RichText text={t('train.drill.missedOut')} parts={{ cards: <span className={styles.good}>{list(grade.missedOut)}</span> }} /></li>}
        {grade.extraOut.length > 0 && <li><RichText text={t('train.drill.extraOut')} parts={{ cards: <span className={styles.bad}>{list(grade.extraOut)}</span> }} /></li>}
      </ul>
      {plan.note && <p className={styles.quiet}>{t('train.drill.note', { note: plan.note })}</p>}
    </div>
  );
}
