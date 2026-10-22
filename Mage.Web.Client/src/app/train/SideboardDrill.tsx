import { Shuffle } from 'lucide-react';
import { useMemo, useState } from 'react';
import { countsByName, gradeDrill, setCount, total, type DrillGrade } from '../../core/decks/sideboardPlans';
import type { PlanCard, SideboardPlan } from '../../core/decks/types';
import { Button } from '../ui/Button';
import { DeckSelect } from './DeckSelect';
import { useDeckList } from './trainData';
import styles from './Train.module.css';

/**
 * The sideboard drill (S018): a kind of deck across the table, your main deck and sideboard, and the question of what
 * you'd swap. The answer is graded against the plan you saved on the deck page.
 */
export function SideboardDrill() {
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
        <h2 id="drill-title">Sideboard drill</h2>
        <p>You know the kind of deck across the table. What comes in, and what goes out? Your answer is checked against the plans you saved on the deck page.</p>
      </header>
      <div className={styles.controls}>
        <DeckSelect value={deckId} onChange={(id) => { setDeckId(id); setPlanId(null); setGrade(null); }} />
        <Button variant="decision" icon={<Shuffle size={16} />} disabled={plans.length === 0} onClick={next}>{plan ? 'Another matchup' : 'Start'}</Button>
      </div>
      {error && <p className={styles.error} role="alert">{error}</p>}
      {deck && plans.length === 0 && (
        <p className={styles.quiet}>This deck has no sideboard plans yet. Open it on the Decks page and use Sideboard plans to write one for each kind of deck you expect.</p>
      )}
      {plan && (
        <>
          <p className={styles.agree}><b>Game two against {plan.archetype}.</b> What do you change?</p>
          <div className={styles.cards}>
            <Picker title={`Bring in (${total(answerIn)})`} cards={side} chosen={answerIn} disabled={!!grade} onCount={(name, count) => setAnswerIn((current) => setCount(current, name, count))} />
            <Picker title={`Take out (${total(answerOut)})`} cards={main} chosen={answerOut} disabled={!!grade} onCount={(name, count) => setAnswerOut((current) => setCount(current, name, count))} />
          </div>
          {!grade ? (
            <div className={styles.controls}>
              <Button variant="decision" onClick={() => setGrade(gradeDrill(plan, answerIn, answerOut))}>Check my answer</Button>
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
                <button type="button" aria-label={`One fewer ${name}`} disabled={disabled || count === 0} onClick={() => onCount(name, count - 1)}>−</button>
                <b>{count}</b>
                <button type="button" aria-label={`One more ${name}`} disabled={disabled || count >= available} onClick={() => onCount(name, count + 1)}>+</button>
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function Graded({ plan, grade }: { plan: SideboardPlan; grade: DrillGrade }) {
  const list = (cards: PlanCard[]) => cards.map((card) => `${card.count} ${card.name}`).join(', ');
  return (
    <div className={styles.result} aria-live="polite">
      <p className={grade.score >= 0.999 ? styles.agree : styles.disagree}>
        <b>{Math.round(grade.score * 100)}% of your plan.</b> {grade.score >= 0.999 ? 'Exactly what you planned.' : 'Here\'s where your answer and your plan differ.'}
      </p>
      <ul className={styles.quiet}>
        {grade.missedIn.length > 0 && <li>Your plan also brings in <span className={styles.good}>{list(grade.missedIn)}</span>.</li>}
        {grade.extraIn.length > 0 && <li>Your plan doesn't bring in <span className={styles.bad}>{list(grade.extraIn)}</span>.</li>}
        {grade.missedOut.length > 0 && <li>Your plan also takes out <span className={styles.good}>{list(grade.missedOut)}</span>.</li>}
        {grade.extraOut.length > 0 && <li>Your plan keeps <span className={styles.bad}>{list(grade.extraOut)}</span>.</li>}
      </ul>
      {plan.note && <p className={styles.quiet}>Your note: {plan.note}</p>}
    </div>
  );
}
