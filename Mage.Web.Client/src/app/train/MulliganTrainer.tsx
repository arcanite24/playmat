import { RotateCcw, Shuffle } from 'lucide-react';
import { useMemo, useState } from 'react';
import { expandDeck, shuffle } from '../../core/decks/analysis';
import { simCard, type SimCard } from '../../core/decks/simulate';
import type { DeckCardInfo } from '../../core/decks/types';
import { EMPTY_STATS, goldfish, record, verdict, type Goldfish, type Opposition, type TrainerStats, type Verdict } from '../../core/train/mulligan';
import type { CardView } from '../../protocol/generated/views';
import { useCardInfo } from '../decks/cardInfo';
import { entryKey } from '../decks/deckModel';
import { readJson, writeJson } from '../stores/persist';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { DeckSelect } from './DeckSelect';
import { useDeckList } from './trainData';
import styles from './Train.module.css';

const STATS_KEY = 'playmat.train.mulligan';
const OPPOSITION: { id: Opposition; label: string }[] = [
  { id: 'aggro', label: 'Aggro (fast creatures and burn)' },
  { id: 'midrange', label: 'Midrange (a fair fight)' },
  { id: 'control', label: 'Control (answers and card draw)' },
];

interface Card {
  entry: DeckCardInfo;
  sim: SimCard;
}

interface Deal {
  order: Card[];
  mulligans: number;
  /** cards chosen for the bottom after a mulligan */
  bottom: Set<number>;
  decided: null | { kept: boolean; table: Verdict; fish: Goldfish | null };
}

/** Keep or ship (S015): deal from your deck, decide, see the table's verdict, and goldfish what you kept. */
export function MulliganTrainer() {
  const [deckId, setDeckId] = useState<string | null>(null);
  const [against, setAgainst] = useState<Opposition>('midrange');
  const [onThePlay, setOnThePlay] = useState(true);
  const { deck, error } = useDeckList(deckId);
  const entries = useMemo(() => deck?.cards ?? [], [deck]);
  const info = useCardInfo(entries);
  const [deal, setDeal] = useState<Deal | null>(null);
  const [stats, setStats] = useState<TrainerStats>(() => readJson(STATS_KEY, EMPTY_STATS));

  const library = useMemo(
    () => expandDeck(entries).map((entry) => ({ entry, sim: simCard(entry.cardName, info.get(entryKey(entry))) })),
    [entries, info],
  );

  function newHand(mulligans = 0) {
    setDeal({ order: shuffle(library), mulligans, bottom: new Set(), decided: null });
  }

  const hand = deal ? deal.order.slice(0, 7) : [];
  const keptCards = deal ? hand.filter((_, index) => !deal.bottom.has(index)) : [];
  const needBottom = deal ? deal.mulligans : 0;
  const ready = !deal || deal.bottom.size === needBottom;

  function decide(kept: boolean) {
    if (!deal) return;
    const table = verdict(keptCards.map((card) => card.sim), against, deal.mulligans);
    let fish: Goldfish | null = null;
    if (kept) {
      const rest = [...deal.order.slice(7), ...hand.filter((_, index) => deal.bottom.has(index))];
      fish = goldfish(keptCards.map((card) => card.sim), rest.map((card) => card.sim), onThePlay, 5);
    }
    const next = record(stats, kept, table, fish ? fish.stalled : null);
    setStats(next);
    writeJson(STATS_KEY, next);
    setDeal({ ...deal, decided: { kept, table, fish } });
  }

  function toggleBottom(index: number) {
    if (!deal || deal.decided) return;
    const bottom = new Set(deal.bottom);
    if (bottom.has(index)) bottom.delete(index);
    else if (bottom.size < needBottom) bottom.add(index);
    setDeal({ ...deal, bottom });
  }

  return (
    <section className={styles.tool} aria-labelledby="mulligan-title">
      <header className={styles.toolHead}>
        <h2 id="mulligan-title">Mulligan trainer</h2>
        <p>Keep or ship an opening hand, see what the table would do, then watch the hand play out alone for five turns.</p>
      </header>
      <div className={styles.controls}>
        <DeckSelect value={deckId} onChange={(id) => { setDeckId(id); setDeal(null); }} />
        <label className={styles.field}>
          <span>Across the table</span>
          <select value={against} onChange={(event) => setAgainst(event.target.value as Opposition)}>
            {OPPOSITION.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}
          </select>
        </label>
        <label className={styles.check}>
          <input type="checkbox" checked={onThePlay} onChange={(event) => setOnThePlay(event.target.checked)} />
          On the play
        </label>
        <Button variant="decision" icon={<Shuffle size={16} />} disabled={library.length < 7} onClick={() => newHand()}>Deal a hand</Button>
      </div>
      {error && <p className={styles.error} role="alert">{error}</p>}
      {deckId && deck && library.length < 7 && <p className={styles.quiet}>This deck has fewer than seven cards.</p>}

      {deal && (
        <>
          <p className={styles.quiet}>
            {deal.mulligans === 0 ? 'Your opening seven.' : `Mulligan to ${7 - deal.mulligans}: pick ${deal.mulligans} card${deal.mulligans === 1 ? '' : 's'} to put on the bottom (${deal.bottom.size} chosen).`}
          </p>
          <div className={styles.hand}>
            {hand.map((card, index) => {
              const view: CardView = info.get(entryKey(card.entry)) ?? { name: card.entry.cardName, expansionSetCode: card.entry.setCode ?? undefined, cardNumber: card.entry.cardNumber ?? undefined };
              const chosen = deal.bottom.has(index);
              return (
                <button
                  key={index}
                  type="button"
                  className={[styles.handCard, chosen ? styles.bottomed : ''].join(' ')}
                  disabled={needBottom === 0 || !!deal.decided}
                  aria-pressed={needBottom > 0 ? chosen : undefined}
                  aria-label={`${card.entry.cardName}${chosen ? ', on the bottom' : ''}`}
                  onClick={() => toggleBottom(index)}
                >
                  <CardFace card={view} size="normal" />
                </button>
              );
            })}
          </div>
          {!deal.decided ? (
            <div className={styles.controls}>
              <Button variant="decision" disabled={!ready} onClick={() => decide(true)}>Keep</Button>
              <Button variant="print" disabled={!ready || deal.mulligans >= 3} onClick={() => decide(false)}>Mulligan</Button>
            </div>
          ) : (
            <Decision deal={deal} onNext={() => newHand()} onMulligan={() => newHand(deal.mulligans + 1)} />
          )}
        </>
      )}

      <footer className={styles.stats} aria-label="Your record">
        <span><b>{stats.hands}</b> hands</span>
        <span><b>{stats.hands ? Math.round((stats.keeps / stats.hands) * 100) : 0}%</b> kept</span>
        <span><b>{stats.hands ? Math.round((stats.agreed / stats.hands) * 100) : 0}%</b> agreed with the table</span>
        <span><b>{stats.keeps ? Math.round((stats.stalls / stats.keeps) * 100) : 0}%</b> of keeps stalled on lands</span>
        {stats.hands > 0 && (
          <Button size="sm" variant="quiet" icon={<RotateCcw size={14} />} onClick={() => { setStats(EMPTY_STATS); writeJson(STATS_KEY, EMPTY_STATS); }}>Reset</Button>
        )}
      </footer>
    </section>
  );
}

function Decision({ deal, onNext, onMulligan }: { deal: Deal; onNext(): void; onMulligan(): void }) {
  const decided = deal.decided!;
  const agreed = decided.kept === decided.table.keep;
  return (
    <div className={styles.result} aria-live="polite">
      <p className={agreed ? styles.agree : styles.disagree}>
        <b>{decided.kept ? 'You kept' : 'You shipped it'}. The table would {decided.table.keep ? 'keep' : 'mulligan'}.</b> {decided.table.reason}
      </p>
      {decided.fish && (
        <>
          <table className={styles.table}>
            <thead>
              <tr><th scope="col">Turn</th><th scope="col">Drew</th><th scope="col">Land</th><th scope="col">Cast</th></tr>
            </thead>
            <tbody>
              {decided.fish.turns.map((turn) => (
                <tr key={turn.turn}>
                  <th scope="row">T{turn.turn}</th>
                  <td>{turn.drew ?? '—'}</td>
                  <td>{turn.land ?? <em>missed</em>}</td>
                  <td>{turn.cast.length ? turn.cast.join(', ') : '—'}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className={styles.quiet}>
            {decided.fish.stalled ? 'This keep missed a land drop in the first four turns.' : 'Every land drop made through turn four.'}
            {' '}{decided.fish.manaSpent} mana spent over five turns.
          </p>
        </>
      )}
      <div className={styles.controls}>
        {!decided.kept && deal.mulligans < 3 && <Button variant="decision" onClick={onMulligan}>Deal {6 - deal.mulligans}</Button>}
        <Button variant={decided.kept ? 'decision' : 'print'} onClick={onNext}>New hand</Button>
      </div>
    </div>
  );
}
