import { Plus, Trash2 } from 'lucide-react';
import { useMemo, useState } from 'react';
import { ARCHETYPES, countsByName, newPlan, planProblems, setCount, total } from '../../core/decks/sideboardPlans';
import type { DeckCardLists, PlanCard, SideboardPlan } from '../../core/decks/types';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import styles from './SideboardPlans.module.css';

/**
 * Sideboard plans on the deck page (S018): against each kind of deck, what comes in and what goes out. A default
 * plan is applied by itself when sideboarding against the computer in a best-of-three; the training page drills them.
 */
export function SideboardPlansDialog({ open, onOpenChange, deck, onChange }: {
  open: boolean;
  onOpenChange(open: boolean): void;
  deck: DeckCardLists;
  onChange(update: (current: DeckCardLists) => DeckCardLists): void;
}) {
  const plans = deck.sideboardPlans ?? [];
  const [selected, setSelected] = useState<string | null>(null);
  const [label, setLabel] = useState<string>(ARCHETYPES[0]);
  const plan = plans.find((candidate) => candidate.id === selected) ?? plans[0] ?? null;
  const main = useMemo(() => [...countsByName(deck.cards)].sort((a, b) => a[0].localeCompare(b[0])), [deck.cards]);
  const side = useMemo(() => [...countsByName(deck.sideboard ?? [])].sort((a, b) => a[0].localeCompare(b[0])), [deck.sideboard]);

  const update = (id: string, change: (plan: SideboardPlan) => SideboardPlan) => onChange((current) => ({
    ...current,
    sideboardPlans: (current.sideboardPlans ?? []).map((candidate) => (candidate.id === id ? change(candidate) : candidate)),
  }));
  const add = () => {
    const created = newPlan(label.trim() || 'Untitled');
    onChange((current) => ({ ...current, sideboardPlans: [...(current.sideboardPlans ?? []), created] }));
    setSelected(created.id);
  };
  const remove = (id: string) => {
    onChange((current) => ({ ...current, sideboardPlans: (current.sideboardPlans ?? []).filter((candidate) => candidate.id !== id) }));
    setSelected(null);
  };
  const makeDefault = (id: string, on: boolean) => onChange((current) => ({
    ...current,
    sideboardPlans: (current.sideboardPlans ?? []).map((candidate) => ({ ...candidate, default: candidate.id === id ? on : on ? false : candidate.default })),
  }));

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Sideboard plans"
      description="Against each kind of deck: what comes in from the sideboard, what goes out of the main deck."
      width="xl"
      footer={<Button variant="decision" onClick={() => onOpenChange(false)}>Done</Button>}
    >
      <div className={styles.layout}>
        <nav className={styles.plans} aria-label="Plans">
          {plans.map((candidate) => (
            <button
              key={candidate.id}
              type="button"
              className={candidate.id === plan?.id ? styles.planOn : styles.plan}
              aria-current={candidate.id === plan?.id}
              onClick={() => setSelected(candidate.id)}
            >
              vs {candidate.archetype}
              <small>{total(candidate.in)} in · {total(candidate.out)} out{candidate.default ? ' · default' : ''}</small>
            </button>
          ))}
          <div className={styles.add}>
            <input
              list="sideboard-archetypes"
              value={label}
              onChange={(event) => setLabel(event.target.value)}
              aria-label="Kind of deck"
              maxLength={40}
            />
            <datalist id="sideboard-archetypes">
              {ARCHETYPES.map((archetype) => <option key={archetype} value={archetype} />)}
            </datalist>
            <Button size="sm" variant="print" icon={<Plus size={15} />} onClick={add}>Add a plan</Button>
          </div>
        </nav>

        {!plan ? (
          <p className={styles.quiet}>{(deck.sideboard ?? []).length === 0 ? 'Add cards to the sideboard first, then plan what comes in.' : 'Add a plan for a kind of deck you expect to meet.'}</p>
        ) : (
          <div className={styles.editor}>
            <div className={styles.planHead}>
              <h3>Against {plan.archetype}</h3>
              <label className={styles.check}>
                <input type="checkbox" checked={!!plan.default} onChange={(event) => makeDefault(plan.id, event.target.checked)} />
                Apply on its own between games
              </label>
              <Button size="sm" variant="quiet" icon={<Trash2 size={15} />} onClick={() => remove(plan.id)}>Delete plan</Button>
            </div>
            <div className={styles.columns}>
              <Side title={`Bring in (${total(plan.in)})`} cards={side} chosen={plan.in} onCount={(name, count) => update(plan.id, (current) => ({ ...current, in: setCount(current.in, name, count) }))} empty="The sideboard is empty." />
              <Side title={`Take out (${total(plan.out)})`} cards={main} chosen={plan.out} onCount={(name, count) => update(plan.id, (current) => ({ ...current, out: setCount(current.out, name, count) }))} empty="The main deck is empty." />
            </div>
            <label className={styles.note}>
              <span>Why (for the drill)</span>
              <input value={plan.note ?? ''} maxLength={200} onChange={(event) => update(plan.id, (current) => ({ ...current, note: event.target.value }))} placeholder="Lifegain and blockers in; slow threats out" />
            </label>
            {planProblems(deck, plan).map((problem) => <p key={problem} className={styles.problem}>{problem}</p>)}
          </div>
        )}
      </div>
    </Dialog>
  );
}

function Side({ title, cards, chosen, onCount, empty }: {
  title: string;
  cards: [string, number][];
  chosen: readonly PlanCard[];
  onCount(name: string, count: number): void;
  empty: string;
}) {
  const counts = new Map(chosen.map((card) => [card.name, card.count]));
  return (
    <section className={styles.side} aria-label={title}>
      <h4>{title}</h4>
      {cards.length === 0 ? <p className={styles.quiet}>{empty}</p> : (
        <ul>
          {cards.map(([name, available]) => {
            const count = counts.get(name) ?? 0;
            return (
              <li key={name} className={count > 0 ? styles.picked : undefined}>
                <span>{name} <small>of {available}</small></span>
                <span className={styles.stepper}>
                  <button type="button" aria-label={`One fewer ${name}`} disabled={count === 0} onClick={() => onCount(name, count - 1)}>−</button>
                  <b aria-live="polite">{count}</b>
                  <button type="button" aria-label={`One more ${name}`} disabled={count >= available} onClick={() => onCount(name, count + 1)}>+</button>
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
