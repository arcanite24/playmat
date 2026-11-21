import { RotateCcw, Shuffle } from 'lucide-react';
import { useMemo, useState } from 'react';
import { expandDeck, shuffle } from '../../core/decks/analysis';
import { simCard, type SimCard } from '../../core/decks/simulate';
import type { DeckCardInfo } from '../../core/decks/types';
import { EMPTY_STATS, goldfish, record, verdict, type Goldfish, type Opposition, type TrainerStats, type Verdict } from '../../core/train/mulligan';
import type { CardView } from '../../protocol/generated/views';
import { useCardInfo } from '../decks/cardInfo';
import { formatNumber, useT, type Translate } from '../i18n';
import { RichText } from '../i18n/RichText';
import { entryKey } from '../decks/deckModel';
import { readJson, writeJson } from '../stores/persist';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { DeckSelect } from './DeckSelect';
import { useDeckList } from './trainData';
import styles from './Train.module.css';

const STATS_KEY = 'playmat.train.mulligan';
const OPPOSITION = [
  { id: 'aggro', label: 'train.mull.vs.aggro' },
  { id: 'midrange', label: 'train.mull.vs.midrange' },
  { id: 'control', label: 'train.mull.vs.control' },
] as const satisfies readonly { id: Opposition; label: string }[];

const percentOf = (part: number, whole: number) => formatNumber(whole ? part / whole : 0, { style: 'percent', maximumFractionDigits: 0 });

/** The table's reason, in the player's language (the English one when the verdict has no code). */
function reasonText(t: Translate, table: Verdict): string {
  const why = table.why;
  if (!why) return table.reason;
  switch (why.code) {
    case 'noLands':
    case 'oneLand':
      return t(`train.why.${why.code}`, { size: why.size });
    case 'flood':
      return t('train.why.flood', { lands: why.lands, spells: why.spells });
    case 'fine':
    case 'fineColors':
      return t(`train.why.${why.code}`, { lands: why.lands, count: why.early });
    default:
      return t(`train.why.${why.code}`);
  }
}

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
  const t = useT();
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
        <h2 id="mulligan-title">{t('train.mull.title')}</h2>
        <p>{t('train.mull.lead')}</p>
      </header>
      <div className={styles.controls}>
        <DeckSelect value={deckId} onChange={(id) => { setDeckId(id); setDeal(null); }} />
        <label className={styles.field}>
          <span>{t('train.mull.against')}</span>
          <select value={against} onChange={(event) => setAgainst(event.target.value as Opposition)}>
            {OPPOSITION.map((option) => <option key={option.id} value={option.id}>{t(option.label)}</option>)}
          </select>
        </label>
        <label className={styles.check}>
          <input type="checkbox" checked={onThePlay} onChange={(event) => setOnThePlay(event.target.checked)} />
          {t('train.mull.onThePlay')}
        </label>
        <Button variant="decision" icon={<Shuffle size={16} />} disabled={library.length < 7} onClick={() => newHand()}>{t('train.mull.deal')}</Button>
      </div>
      {error && <p className={styles.error} role="alert">{error}</p>}
      {deckId && deck && library.length < 7 && <p className={styles.quiet}>{t('train.mull.tooSmall')}</p>}

      {deal && (
        <>
          <p className={styles.quiet}>
            {deal.mulligans === 0
              ? t('train.mull.seven')
              : t('train.mull.bottom', { size: 7 - deal.mulligans, count: deal.mulligans, chosen: deal.bottom.size })}
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
                  aria-label={chosen ? t('train.mull.onBottom', { name: card.entry.cardName }) : card.entry.cardName}
                  onClick={() => toggleBottom(index)}
                >
                  <CardFace card={view} size="normal" />
                </button>
              );
            })}
          </div>
          {!deal.decided ? (
            <div className={styles.controls}>
              <Button variant="decision" disabled={!ready} onClick={() => decide(true)}>{t('train.mull.keep')}</Button>
              <Button variant="print" disabled={!ready || deal.mulligans >= 3} onClick={() => decide(false)}>{t('train.mull.mulligan')}</Button>
            </div>
          ) : (
            <Decision deal={deal} onNext={() => newHand()} onMulligan={() => newHand(deal.mulligans + 1)} />
          )}
        </>
      )}

      <footer className={styles.stats} aria-label={t('train.mull.record')}>
        <span><RichText text={t('train.mull.hands', { count: stats.hands })} parts={{ n: <b>{formatNumber(stats.hands)}</b> }} /></span>
        <span><RichText text={t('train.mull.kept')} parts={{ percent: <b>{percentOf(stats.keeps, stats.hands)}</b> }} /></span>
        <span><RichText text={t('train.mull.agreed')} parts={{ percent: <b>{percentOf(stats.agreed, stats.hands)}</b> }} /></span>
        <span><RichText text={t('train.mull.stalled')} parts={{ percent: <b>{percentOf(stats.stalls, stats.keeps)}</b> }} /></span>
        {stats.hands > 0 && (
          <Button size="sm" variant="quiet" icon={<RotateCcw size={14} />} onClick={() => { setStats(EMPTY_STATS); writeJson(STATS_KEY, EMPTY_STATS); }}>{t('train.mull.reset')}</Button>
        )}
      </footer>
    </section>
  );
}

function Decision({ deal, onNext, onMulligan }: { deal: Deal; onNext(): void; onMulligan(): void }) {
  const t = useT();
  const decided = deal.decided!;
  const agreed = decided.kept === decided.table.keep;
  return (
    <div className={styles.result} aria-live="polite">
      <p className={agreed ? styles.agree : styles.disagree}>
        <b>{t(decided.kept ? 'train.mull.youKept' : 'train.mull.youShipped')} {t(decided.table.keep ? 'train.mull.tableKeeps' : 'train.mull.tableShips')}</b> {reasonText(t, decided.table)}
      </p>
      {decided.fish && (
        <>
          <table className={styles.table}>
            <thead>
              <tr><th scope="col">{t('train.mull.turn')}</th><th scope="col">{t('train.mull.drew')}</th><th scope="col">{t('train.mull.land')}</th><th scope="col">{t('train.mull.cast')}</th></tr>
            </thead>
            <tbody>
              {decided.fish.turns.map((turn) => (
                <tr key={turn.turn}>
                  <th scope="row">{t('train.mull.turnShort', { turn: turn.turn })}</th>
                  <td>{turn.drew ?? '—'}</td>
                  <td>{turn.land ?? <em>{t('train.mull.missed')}</em>}</td>
                  <td>{turn.cast.length ? turn.cast.join(', ') : '—'}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className={styles.quiet}>
            {t(decided.fish.stalled ? 'train.mull.fishStalled' : 'train.mull.fishFine')}
            {' '}{t('train.mull.manaSpent', { count: decided.fish.manaSpent })}
          </p>
        </>
      )}
      <div className={styles.controls}>
        {!decided.kept && deal.mulligans < 3 && <Button variant="decision" onClick={onMulligan}>{t('train.mull.dealSize', { count: 6 - deal.mulligans })}</Button>}
        <Button variant={decided.kept ? 'decision' : 'print'} onClick={onNext}>{t('train.mull.newHand')}</Button>
      </div>
    </div>
  );
}
