import { ArrowLeftRight } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import type { SideboardPlan } from '../../core/decks/types';
import { useDecks } from '../stores/decks';
import { Button } from '../ui/Button';
import styles from './Build.module.css';

/**
 * Between games of a best-of-three (S018): the sideboard plans saved with this deck (when it's one of yours) as
 * one-click swaps. The plan marked default is applied as the screen opens.
 */
export function SideboardPlanBar({ deckName, disabled, onApply }: { deckName: string | undefined; disabled: boolean; onApply(plan: SideboardPlan): void }) {
  const saved = useDecks((state) => state.saved);
  const loaded = useDecks((state) => state.loaded);
  const refresh = useDecks((state) => state.refresh);
  const loadList = useDecks((state) => state.loadList);
  const [plans, setPlans] = useState<SideboardPlan[]>([]);
  const [applied, setApplied] = useState<string | null>(null);
  const applyRef = useRef(onApply);
  const disabledRef = useRef(disabled);
  useEffect(() => {
    applyRef.current = onApply;
    disabledRef.current = disabled;
  }, [onApply, disabled]);
  useEffect(() => {
    if (!loaded) void refresh();
  }, [loaded, refresh]);
  const id = saved.find((deck) => deck.name === deckName)?.id ?? null;
  useEffect(() => {
    if (!id) return;
    let cancelled = false;
    loadList(id).then((deck) => {
      if (cancelled) return;
      const found = deck.sideboardPlans ?? [];
      setPlans(found);
      const preferred = found.find((plan) => plan.default);
      if (preferred && !disabledRef.current) {
        applyRef.current(preferred);
        setApplied(preferred.id);
      }
    }, () => undefined);
    return () => {
      cancelled = true;
    };
  }, [id, loadList]);
  if (!id || plans.length === 0) return null;
  return (
    <div className={styles.planBar} role="group" aria-label="Sideboard plans">
      <ArrowLeftRight size={16} aria-hidden="true" />
      <span>Your plans:</span>
      {plans.map((plan) => (
        <Button
          key={plan.id}
          size="sm"
          variant={applied === plan.id ? 'decision' : 'print'}
          disabled={disabled}
          title={plan.note || undefined}
          onClick={() => {
            onApply(plan);
            setApplied(plan.id);
          }}
        >
          vs {plan.archetype}
        </Button>
      ))}
      {applied && <span className={styles.planNote}>Applied. Click cards to adjust.</span>}
    </div>
  );
}
