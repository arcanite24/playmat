import { Check, Minus, Plus, Sparkles } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import type { CardView } from '../../protocol/generated/views';
import { api } from '../connection';
import { useCardInfo } from '../decks/cardInfo';
import { groupOf } from '../decks/deckModel';
import { MatchScore } from '../match/MatchScore';
import { lessonsFor, useCareerLessons } from '../stores/careerLessons';
import { useEvents, type ConstructState } from '../stores/events';
import { useT } from '../i18n';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { BASICS, countBasics, initialMain, isBasic, toDeckLists, type BasicName } from './buildDeck';
import { applyPlan } from '../../core/decks/sideboardPlans';
import { curveColumns, faceOf, poolEntry, poolKey, type PoolCard } from './pool';
import { SideboardPlanBar } from './SideboardPlanBar';
import styles from './Build.module.css';

/** Deck building for an event: after a draft or with a sealed pool, and sideboarding between games. */
export function BuildScreen() {
  const { tableId = '' } = useParams();
  const navigate = useNavigate();
  const construct = useEvents((state) => (state.construct?.tableId === tableId ? state.construct : null));
  if (!construct) {
    return (
      <div className={styles.screen}>
        <div className={styles.empty}>
          <h1>Nothing to build</h1>
          <p>Deck building for this event isn't open in this tab.</p>
          <Button variant="print" onClick={() => navigate('/events')}>Back to events</Button>
        </div>
      </div>
    );
  }
  return <Builder key={`${construct.tableId}:${construct.kind}`} construct={construct} />;
}

function Builder({ construct }: { construct: ConstructState }) {
  const submit = useEvents((state) => state.submit);
  const pool = useMemo(
    () => [...Object.values(construct.deck.cards ?? {}), ...Object.values(construct.deck.sideboard ?? {})] as PoolCard[],
    [construct.deck],
  );
  const [main, setMain] = useState<ReadonlySet<string>>(() => initialMain(construct.deck, construct.limited));
  const [basics, setBasicCounts] = useState<Record<BasicName, number>>(() => countBasics(Object.values(construct.deck.cards ?? {}) as PoolCard[]));
  // nothing is saved until the player changes something: the server already holds the deck as it came
  const [touched, setTouched] = useState(false);
  const setBasics = (next: (current: Record<BasicName, number>) => Record<BasicName, number>) => {
    setTouched(true);
    setBasicCounts(next);
  };
  const [colors, setColors] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const entries = useMemo(() => pool.map(poolEntry), [pool]);
  const info = useCardInfo(entries);

  const inDeck = useMemo(() => pool.filter((card) => main.has(card.id!)), [pool, main]);
  const inPool = useMemo(() => pool.filter((card) => !main.has(card.id!) && !(construct.limited && isBasic(card)) && matchesColors(card, info, colors)), [pool, main, construct.limited, info, colors]);
  const deckColumns = useMemo(() => curveColumns(inDeck, info), [inDeck, info]);
  const poolColumns = useMemo(() => curveColumns(inPool, info), [inPool, info]);
  const landCount = Object.values(basics).reduce((sum, value) => sum + value, 0);
  const total = inDeck.length + (construct.limited ? landCount : 0);
  const minimum = construct.limited ? 40 : 60;

  const wire = useMemo(() => toDeckLists(pool, main, construct.limited ? basics : null), [pool, main, basics, construct.limited]);
  const lessons = lessonsFor(useCareerLessons((state) => state.plan), construct.tableId);
  const lesson = construct.kind === 'sideboard' ? lessons?.sideboard ?? null : null;

  // keep the server's copy current, so a timeout submits what's on screen
  useEffect(() => {
    if (construct.submitted || !touched) return;
    const timer = setTimeout(() => api.deckSave(construct.tableId, wire).catch(() => undefined), 1200);
    return () => clearTimeout(timer);
  }, [wire, touched, construct.tableId, construct.submitted]);

  const toggle = (card: PoolCard) => {
    if (construct.submitted) return;
    setTouched(true);
    setMain((current) => {
      const next = new Set(current);
      if (next.has(card.id!)) next.delete(card.id!);
      else next.add(card.id!);
      return next;
    });
  };

  return (
    <div className={[styles.screen, lesson ? styles.screenLesson : ''].join(' ')}>
      <header className={styles.top}>
        <div>
          <h1 className={styles.title}>{construct.kind === 'construct' ? 'Build your deck' : 'Adjust your deck'}</h1>
          {construct.kind === 'sideboard' && <MatchScore tableId={construct.tableId} />}
        </div>
        <div className={styles.topRight}>
          {construct.deadline && !construct.submitted && <Countdown deadline={construct.deadline} />}
          <div className={styles.count} aria-live="polite">
            <b className={total >= minimum ? styles.countOk : ''}>{total}</b>
            <span>/ {minimum} cards{construct.limited ? ` · ${inDeck.length} spells, ${landCount} lands` : ''}</span>
          </div>
          {construct.submitted ? (
            <span className={styles.submitted}><Check size={18} aria-hidden="true" /> Deck submitted — waiting for the others</span>
          ) : (
            <Button
              variant="decision"
              size="lg"
              busy={busy}
              disabled={total < minimum}
              onClick={async () => {
                setBusy(true);
                await submit(wire);
                setBusy(false);
              }}
            >
              Submit deck
            </Button>
          )}
        </div>
      </header>
      {lesson && <SideboardLesson text={lesson} />}
      {construct.kind === 'sideboard' && !construct.limited && (
        <SideboardPlanBar
          deckName={construct.deck.name}
          disabled={construct.submitted}
          onApply={(plan) => {
            setTouched(true);
            setMain((current) => applyPlan(pool.map((card) => ({ id: card.id!, name: info.get(poolKey(card))?.name ?? card.name ?? '' })), current, plan));
          }}
        />
      )}

      <section className={styles.zone} aria-label={`Card pool, ${inPool.length} cards`}>
        <div className={styles.zoneHead}>
          <h2>Pool <span>{inPool.length}</span></h2>
          <div className={styles.pips} role="group" aria-label="Show colors">
            {BASICS.map((basic) => {
              const on = colors.includes(basic.color);
              return (
                <button
                  key={basic.color}
                  type="button"
                  className={[styles.pip, on ? styles.pipOn : ''].join(' ')}
                  aria-pressed={on}
                  aria-label={`Show ${basic.color} cards`}
                  onClick={() => setColors((current) => (on ? current.filter((color) => color !== basic.color) : [...current, basic.color]))}
                >
                  <i className={`ms ms-cost ms-${basic.symbol}`} aria-hidden="true" />
                </button>
              );
            })}
          </div>
          <p className={styles.tip}>Click a card to move it between the pool and your deck.</p>
        </div>
        <Columns columns={poolColumns} info={info} onClick={toggle} empty="Every card is in your deck." />
      </section>

      <section className={[styles.zone, styles.deckZone].join(' ')} aria-label={`Deck, ${inDeck.length} cards`}>
        <div className={styles.zoneHead}>
          <h2>Deck <span>{inDeck.length}</span></h2>
          {construct.limited && (
            <div className={styles.basics} aria-label="Basic lands">
              {BASICS.map((basic) => (
                <div key={basic.name} className={styles.basic}>
                  <i className={`ms ms-cost ms-${basic.symbol}`} aria-hidden="true" />
                  <button type="button" aria-label={`One less ${basic.name}`} disabled={basics[basic.name] === 0 || construct.submitted} onClick={() => setBasics((current) => ({ ...current, [basic.name]: current[basic.name] - 1 }))}><Minus size={13} /></button>
                  <b aria-label={`${basics[basic.name]} ${basic.name}`}>{basics[basic.name]}</b>
                  <button type="button" aria-label={`One more ${basic.name}`} disabled={construct.submitted} onClick={() => setBasics((current) => ({ ...current, [basic.name]: current[basic.name] + 1 }))}><Plus size={13} /></button>
                </div>
              ))}
              <Button variant="quiet" size="sm" icon={<Sparkles size={15} />} disabled={construct.submitted || inDeck.length === 0} onClick={() => setBasics(() => suggestLands(inDeck, info, minimum - inDeck.length))}>
                Suggest lands
              </Button>
            </div>
          )}
        </div>
        <Columns columns={deckColumns} info={info} onClick={toggle} empty="Click cards in the pool to add them." />
      </section>
    </div>
  );
}

function Columns({ columns, info, onClick, empty }: {
  columns: { label: string; cards: PoolCard[] }[];
  info: ReadonlyMap<string, CardView>;
  onClick(card: PoolCard): void;
  empty: string;
}) {
  if (columns.length === 0) return <p className={styles.note}>{empty}</p>;
  return (
    <div className={styles.columns}>
      {columns.map((column) => (
        <div key={column.label} className={styles.column}>
          <span className={styles.columnLabel}>{column.label} <small>{column.cards.length}</small></span>
          <div className={styles.columnCards} style={{ height: 176 + (column.cards.length - 1) * 30 }}>
            {column.cards.map((card, index) => (
              <button
                key={card.id}
                type="button"
                className={styles.card}
                style={{ top: index * 30, zIndex: index }}
                onClick={() => onClick(card)}
                aria-label={card.name}
              >
                <CardFace card={faceOf(card, info)} size="small" />
              </button>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

function Countdown({ deadline }: { deadline: number }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);
  const left = Math.max(0, Math.round((deadline - now) / 1000));
  const minutes = Math.floor(left / 60);
  const seconds = String(left % 60).padStart(2, '0');
  return <span className={[styles.countdown, left < 60 ? styles.countdownLow : ''].join(' ')} role="timer">{minutes}:{seconds}</span>;
}

function matchesColors(card: PoolCard, info: ReadonlyMap<string, CardView>, colors: string[]): boolean {
  if (colors.length === 0) return true;
  const color = info.get(poolKey(card))?.color;
  if (!color) return true;
  const own = BASICS.filter((basic) => color[basic.color]).map((basic) => basic.color);
  // colorless cards fit any deck
  return own.length === 0 || own.every((value) => colors.includes(value));
}

/** Basic lands split by the colored mana symbols in the deck's costs. */
function suggestLands(deck: PoolCard[], info: ReadonlyMap<string, CardView>, wanted: number): Record<BasicName, number> {
  const symbols: Record<BasicName, number> = { Plains: 0, Island: 0, Swamp: 0, Mountain: 0, Forest: 0 };
  for (const card of deck) {
    const details = info.get(poolKey(card));
    if (!details || groupOf(details) === 'lands') continue;
    const cost = (details.manaCostLeftStr ?? []).join('');
    for (const basic of BASICS) {
      const matches = cost.match(new RegExp(`\\{[^}]*${basic.symbol.toUpperCase()}[^}]*\\}`, 'g'));
      symbols[basic.name] += matches?.length ?? 0;
    }
  }
  const lands = Math.max(0, wanted);
  const totalSymbols = Object.values(symbols).reduce((sum, value) => sum + value, 0);
  const result: Record<BasicName, number> = { Plains: 0, Island: 0, Swamp: 0, Mountain: 0, Forest: 0 };
  if (totalSymbols === 0 || lands === 0) return result;
  let given = 0;
  for (const basic of BASICS) {
    result[basic.name] = Math.floor((symbols[basic.name] / totalSymbols) * lands);
    given += result[basic.name];
  }
  // hand out the remainder to the colors used most
  const order = [...BASICS].sort((a, b) => symbols[b.name] - symbols[a.name]);
  for (let i = 0; given < lands; i++, given++) result[order[i % order.length].name]++;
  return result;
}

/** A school's word on sideboarding, between the games of its best-of-three final. */
function SideboardLesson({ text }: { text: string }) {
  const t = useT();
  return (
    <aside className={styles.lesson} aria-label={t('match.lesson.sideboard')}>
      <b>{t('match.lesson.sideboard')}</b>
      <p>{text}</p>
    </aside>
  );
}
