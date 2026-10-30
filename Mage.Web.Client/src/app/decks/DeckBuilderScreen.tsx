import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { ArrowLeftRight, BarChart3, Check, ChevronLeft, ClipboardPaste, Columns3, Crown, Images, ListPlus, Minus, Mountain, Plus, Replace, Sparkles, TriangleAlert } from 'lucide-react';
import { useMyCardStyles } from '../ui/cardStyles';
import { useCallback, useEffect, useMemo, useState, type ComponentType } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import type { CardView } from '../../protocol/generated/views';
import { BASIC_FOR, MANA_LETTERS, suggestLands, type ManaLetter } from '../../core/decks/analysis';
import { canCommand, commanderIdentity, formatMenu, formatRules, withinIdentity, type FormatRules } from '../../core/decks/formats';
import { appendDeckCardLists } from '../../core/decks/merge';
import { deckStorage } from '../../core/decks/DeckStorageService';
import type { DeckCardInfo, DeckCardLists } from '../../core/decks/types';
import { useServerState } from '../queries';
import { STARTER_PREFIX, useDecks } from '../stores/decks';
import { openImport, type ImportTarget } from '../stores/importSheet';
import { notify } from '../stores/toasts';
import { readJson, writeJson } from '../stores/persist';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { ManaCost } from '../ui/ManaCost';
import { useCardInfo, useCardInfoStore } from './cardInfo';
import { Collection } from './Collection';
import { DeckInsights } from './DeckInsights';
import { SideboardPlansDialog } from './SideboardPlansDialog';
import { PrintingPicker } from './PrintingPicker';
import { ExportMenu } from './ExportMenu';
import { ManaCurve } from './ManaCurve';
import { ManaColumns } from './ManaColumns';
import { SourceLine } from './import/SourceLine';
import { useImportPaste } from './import/useImportShortcuts';
import { useValidation } from './useValidation';
import {
  addCard, changePrinting, copiesByName, copyLimit, countZone, entryKey, finalizeDeck, groupDeck, manaCurve, moveCard,
  printingOf, removeCard, type DeckRow, type DeckZone,
} from './deckModel';
import styles from './DeckBuilder.module.css';

const DEFAULT_FORMAT = 'Constructed - Freeform';

// the deck by mana value in place of the collection, remembered per browser
type DeckView = 'list' | 'mana';
const DECK_VIEW_KEY = 'playmat.deckBuilder.view';

/** What the card pool on the left gets from the builder (the collection of every card, or a player's own binder). */
export interface BinderProps {
  format: string;
  identity: string[] | null;
  counts: ReadonlyMap<string, number>;
  limitOf(card: CardView): number;
  onAdd(card: CardView): void;
  onRemove(card: CardView): void;
  onPreview(card: CardView | null, anchor?: DOMRect): void;
}

/**
 * A deck kept somewhere other than the deck library, built from a pool of owned cards (Career): its own binder, its own
 * saving, its own way back, a fixed format, and a cap per card on top of the format's.
 */
export interface BuilderMode {
  Binder: ComponentType<BinderProps>;
  /** copies the player may put in, by card name */
  allowance(name: string): number;
  save(deck: DeckCardLists): void;
  doneLabel: string;
  onDone(): void;
  onPlay(): void;
}

/** Opens the deck named in the URL: starter decks are copied into the library first, "new" makes an empty deck. */
export function DeckBuilderScreen() {
  const { deckId = '' } = useParams();
  return <DeckLoader key={deckId} deckId={deckId} />;
}

function DeckLoader({ deckId }: { deckId: string }) {
  const navigate = useNavigate();
  const [deck, setDeck] = useState<DeckCardLists | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      if (deckId === 'new') {
        const id = await deckStorage.saveDeck({ name: 'New deck', format: DEFAULT_FORMAT, cards: [], sideboard: [] }, { forceNew: true });
        await useDecks.getState().refresh();
        useDecks.getState().select(id);
        if (!cancelled) navigate(`/decks/${id}`, { replace: true });
        return;
      }
      if (deckId.startsWith(STARTER_PREFIX)) {
        if (!useDecks.getState().loaded) await useDecks.getState().refresh();
        const { id } = await useDecks.getState().loadForPlay(deckId);
        if (!cancelled) navigate(`/decks/${id}`, { replace: true });
        return;
      }
      const loaded = await deckStorage.loadDeck(deckId);
      if (cancelled) return;
      if (!loaded) setError('This deck is no longer saved in this browser.');
      else setDeck({ ...loaded, id: deckId, format: loaded.format || DEFAULT_FORMAT });
    })().catch((reason) => !cancelled && setError(reason instanceof Error ? reason.message : String(reason)));
    return () => {
      cancelled = true;
    };
  }, [deckId, navigate]);

  if (error) {
    return (
      <div className={styles.missing}>
        <h1>Deck not found</h1>
        <p>{error}</p>
        <Button variant="print" onClick={() => navigate('/decks')}>Back to decks</Button>
      </div>
    );
  }
  if (!deck) return <div className={styles.missing}><p>Opening the deck…</p></div>;
  return <DeckBuilder key={deck.id} initial={deck} />;
}

/** The builder itself; `mode` builds a deck outside the library (Career). */
export function DeckBuilder({ initial, mode }: { initial: DeckCardLists; mode?: BuilderMode }) {
  const navigate = useNavigate();
  const [deck, setDeck] = useState(initial);
  const [zone, setZone] = useState<DeckZone>('cards');
  const [preview, setPreview] = useState<{ card: CardView; anchor: DOMRect; side: 'auto' | 'left' } | null>(null);
  const [view, setViewState] = useState<DeckView>(() => readJson<DeckView>(DECK_VIEW_KEY, 'list'));
  const setView = useCallback((next: DeckView) => {
    setViewState(next);
    writeJson(DECK_VIEW_KEY, next);
  }, []);
  // a deck pasted here becomes its own deck; "From a list…" brings cards into this one
  useImportPaste();
  const allEntries = useMemo(() => [...deck.cards, ...deck.sideboard], [deck.cards, deck.sideboard]);
  const info = useCardInfo(allEntries);
  const format = deck.format || DEFAULT_FORMAT;
  const rules = formatRules(format);
  const server = useServerState();
  const formats = useMemo(() => formatMenu(server.data?.deckTypes ?? [DEFAULT_FORMAT]), [server.data]);
  const identity = useCommanderIdentity(deck, info, rules);

  // autosave: the deck in storage always matches the screen, a moment later
  const [savedDeck, setSavedDeck] = useState(initial);
  const saving = deck !== savedDeck;
  useEffect(() => {
    if (!saving) return;
    const timer = setTimeout(() => {
      const finished = finalizeDeck(deck, useCardInfoStore.getState().info);
      if (mode) {
        mode.save(finished);
        setSavedDeck(deck);
        return;
      }
      void deckStorage.saveDeck(finished)
        .then(() => setSavedDeck(deck))
        .catch((reason) => notify("Couldn't save the deck", String(reason), 'error'));
    }, 500);
    return () => clearTimeout(timer);
  }, [deck, saving, mode]);
  useEffect(() => () => {
    if (!mode) void useDecks.getState().refresh();
  }, [mode]);

  const counts = useMemo(() => {
    const map = new Map<string, number>();
    for (const entry of allEntries) map.set(entry.cardName.toLowerCase(), (map.get(entry.cardName.toLowerCase()) ?? 0) + entry.amount);
    return map;
  }, [allEntries]);
  const limitFor = useCallback((card: CardView | undefined, name: string) => {
    const limit = format === 'Limited' ? Infinity : copyLimit(card, name, rules.singleton);
    return mode ? Math.min(limit, mode.allowance(name)) : limit;
  }, [format, rules.singleton, mode]);
  const limitOf = useCallback((card: CardView) => limitFor(card, card.name ?? ''), [limitFor]);

  // a card already at its limit says why nothing was added
  const atLimit = useCallback((card: CardView | undefined, name: string, limit: number) => {
    if (mode && limit < copyLimit(card, name, rules.singleton)) notify(limit === 1 ? 'You own 1 copy' : `You own ${limit} copies`, `Every copy of ${name} you own is in the deck.`);
    else notify(`${limit} copies at most`, `${name} is already at its limit.`);
  }, [mode, rules.singleton]);
  const add = useCallback((card: CardView) => {
    const limit = limitOf(card);
    if (copiesByName(deck, card.name ?? '') >= limit) {
      atLimit(card, card.name ?? '', limit);
      return;
    }
    setDeck((current) => (copiesByName(current, card.name ?? '') >= limit ? current : addCard(current, zone, printingOf(card))));
  }, [deck, zone, limitOf, atLimit]);
  const removeOne = useCallback((card: CardView) => {
    setDeck((current) => {
      const name = (card.name ?? '').toLowerCase();
      // prefer the zone being edited, then the other one
      for (const from of [zone, zone === 'cards' ? 'sideboard' : 'cards'] as DeckZone[]) {
        const entry = current[from].find((candidate) => candidate.cardName.toLowerCase() === name);
        if (entry) return removeCard(current, from, entryKey(entry));
      }
      return current;
    });
  }, [zone]);
  // a card in the mana value columns: click for one more, right-click for one less
  const addRow = useCallback((row: DeckRow) => {
    const name = row.entry.cardName;
    const limit = limitFor(row.card, name);
    if (copiesByName(deck, name) >= limit) {
      atLimit(row.card, name, limit);
      return;
    }
    setDeck((current) => (copiesByName(current, name) >= limit
      ? current
      : addCard(current, zone, { cardName: name, setCode: row.entry.setCode ?? '', cardNumber: row.entry.cardNumber ?? '' })));
  }, [deck, zone, limitFor, atLimit]);
  const removeRow = useCallback((row: DeckRow) => setDeck((current) => removeCard(current, zone, row.key)), [zone]);
  // collection cards preview beside the card; deck rows preview to the left of the deck list
  const onPreview = useCallback((card: CardView | null, anchor?: DOMRect) => setPreview(card && anchor ? { card, anchor, side: 'auto' } : null), []);
  const onRowPreview = useCallback((card: CardView | null, anchor?: DOMRect) => setPreview(card && anchor ? { card, anchor, side: 'left' } : null), []);

  return (
    <div className={styles.page}>
      {view === 'mana' ? (
        <section className={styles.collection} aria-label="Deck by mana value">
          <div className={styles.filters}>
            <span className={styles.toggle}>{zone === 'cards' ? 'Deck' : rules.commandZone ? 'Command zone' : 'Sideboard'} by mana value. Click a card for one more, right-click for one less.</span>
            <button type="button" className={styles.reset} onClick={() => setView('list')}>Back to the cards</button>
          </div>
          <div className={styles.results}>
            {deck[zone].length === 0
              ? <p className={styles.note}>No cards here yet.</p>
              : <ManaColumns entries={deck[zone]} info={info} onAdd={addRow} onRemove={removeRow} onPreview={onPreview} />}
          </div>
        </section>
      ) : mode ? (
        <mode.Binder format={format} identity={identity} counts={counts} limitOf={limitOf} onAdd={add} onRemove={removeOne} onPreview={onPreview} />
      ) : (
        <Collection format={format === 'Limited' ? '' : format} identity={identity} counts={counts} limitOf={limitOf} onAdd={add} onRemove={removeOne} onPreview={onPreview} />
      )}
      <DeckPanel
        deck={deck}
        zone={zone}
        view={view}
        onView={setView}
        info={info}
        formats={formats}
        rules={rules}
        identity={identity}
        saving={saving}
        limitFor={limitFor}
        fixedFormat={!!mode}
        doneLabel={mode?.doneLabel ?? 'Decks'}
        onZone={setZone}
        onChange={setDeck}
        onPreview={onRowPreview}
        onDone={mode ? mode.onDone : () => navigate('/decks')}
        onPlay={mode ? mode.onPlay : () => {
          useDecks.getState().select(deck.id!);
          navigate('/');
        }}
      />
      {preview && <Preview {...preview} />}
    </div>
  );
}

/** The command zone's color identity once it holds a commander (null: not a commander deck, or no commander yet). */
function useCommanderIdentity(deck: DeckCardLists, info: ReadonlyMap<string, CardView>, rules: FormatRules): string[] | null {
  return useMemo(() => {
    if (!rules.commandZone) return null;
    const commanders = deck.sideboard.map((entry) => info.get(entryKey(entry))).filter((card): card is CardView => !!card);
    return commanders.length > 0 ? commanderIdentity(commanders) : null;
  }, [deck.sideboard, info, rules.commandZone]);
}

function DeckPanel({ deck, zone, view, onView, info, formats, rules, identity, saving, limitFor, fixedFormat, doneLabel, onZone, onChange, onPreview, onDone, onPlay }: {
  deck: DeckCardLists;
  zone: DeckZone;
  view: DeckView;
  onView(view: DeckView): void;
  info: ReadonlyMap<string, CardView>;
  formats: { label: string; types: string[] }[];
  rules: FormatRules;
  identity: string[] | null;
  saving: boolean;
  limitFor(card: CardView | undefined, name: string): number;
  /** the format is the mode's (Career decks are Freeform), not the player's to pick */
  fixedFormat: boolean;
  doneLabel: string;
  onZone(zone: DeckZone): void;
  onChange(update: (deck: DeckCardLists) => DeckCardLists): void;
  onPreview(card: CardView | null, anchor?: DOMRect): void;
  onDone(): void;
  onPlay(): void;
}) {
  const format = deck.format || DEFAULT_FORMAT;
  const groups = useMemo(() => groupDeck(deck[zone], info), [deck, zone, info]);
  const curve = useMemo(() => manaCurve(deck.cards, info), [deck.cards, info]);
  const mainCount = countZone(deck, 'cards');
  const sideCount = countZone(deck, 'sideboard');
  const validation = useValidation(deck, format);
  const [problemsOpen, setProblemsOpen] = useState(false);
  const problems = validation.data?.errors ?? [];
  const [printingFor, setPrintingFor] = useState<DeckRow | null>(null);
  const [insightsOpen, setInsightsOpen] = useState(false);
  const [plansOpen, setPlansOpen] = useState(false);
  const commandZone = rules.commandZone;
  const listed = formats.some((group) => group.types.includes(format));
  // a commander deck counts its command zone; other decks count the main deck only
  const total = commandZone ? mainCount + sideCount : mainCount;

  /** where the import sheet puts a list for this deck: in place of its cards, or added to them */
  function intoDeck(mode: 'replace' | 'add'): ImportTarget {
    return {
      kind: 'into',
      mode,
      deckName: deck.name || 'this deck',
      apply: (fresh) => onChange((current) => (mode === 'replace'
        ? { ...current, cards: fresh.cards, sideboard: fresh.sideboard, format: fresh.format || current.format, source: fresh.source ?? current.source }
        : appendDeckCardLists(current, fresh))),
    };
  }

  return (
    <aside className={styles.deck} aria-label="Deck">
      <header className={styles.deckHead}>
        <input
          className={styles.deckName}
          value={deck.name ?? ''}
          onChange={(event) => {
            const name = event.target.value;
            onChange((current) => ({ ...current, name }));
          }}
          aria-label="Deck name"
          maxLength={60}
        />
        <select
          className={styles.select}
          disabled={fixedFormat}
          value={format}
          onChange={(event) => {
            const value = event.target.value;
            onChange((current) => ({ ...current, format: value }));
          }}
          aria-label="Format"
        >
          {!listed && <option value={format}>{formatRules(format).label}</option>}
          {formats.map((group) => (
            <optgroup key={group.label} label={group.label}>
              {group.types.map((type) => <option key={type} value={type}>{formatRules(type).label}</option>)}
            </optgroup>
          ))}
        </select>
      </header>

      <div className={styles.status}>
        <span className={styles.total}>{total}{rules.deckSize ? <small> / {rules.deckSize}</small> : <small> cards</small>}</span>
        {validation.data && (validation.data.valid ? (
          <span className={[styles.chip, styles.chipOk].join(' ')}><Check size={15} aria-hidden="true" /> Legal</span>
        ) : (
          <button type="button" className={[styles.chip, styles.chipBad].join(' ')} onClick={() => setProblemsOpen((open) => !open)} aria-expanded={problemsOpen}>
            <TriangleAlert size={15} aria-hidden="true" /> {problems.length} {problems.length === 1 ? 'problem' : 'problems'}
          </button>
        ))}
        <span className={styles.saved} aria-live="polite">{saving ? 'Saving…' : 'Saved'}</span>
      </div>
      {problemsOpen && problems.length > 0 && (
        <ul className={styles.problems}>
          {problems.slice(0, 12).map((problem, index) => (
            <li key={index}><b>{problem.group}</b> {problem.message}</li>
          ))}
        </ul>
      )}

      {commandZone && <CommandZone deck={deck} info={info} rules={rules} identity={identity} onChange={onChange} onPreview={onPreview} />}

      <div className={styles.tabs}>
        <div className={styles.zoneTabs} role="tablist" aria-label="Deck zone">
          <button type="button" role="tab" aria-selected={zone === 'cards'} className={zone === 'cards' ? styles.tabOn : styles.tab} onClick={() => onZone('cards')}>Deck {mainCount}</button>
          <button type="button" role="tab" aria-selected={zone === 'sideboard'} className={zone === 'sideboard' ? styles.tabOn : styles.tab} onClick={() => onZone('sideboard')}>
            {commandZone ? 'Command zone' : 'Sideboard'} {sideCount}
          </button>
        </div>
        <button
          type="button"
          className={[styles.viewToggle, view === 'mana' ? styles.viewToggleOn : ''].join(' ')}
          aria-pressed={view === 'mana'}
          title={view === 'mana' ? 'Back to the cards' : 'Lay out the deck in columns by mana value'}
          onClick={() => onView(view === 'mana' ? 'list' : 'mana')}
        >
          <Columns3 size={15} aria-hidden="true" /> By mana value
        </button>
      </div>

      <div className={styles.list} role="tabpanel">
        {groups.length === 0 && (
          <p className={styles.note}>
            {zone === 'cards'
              ? 'Click cards on the left to add them.'
              : commandZone
                ? `Cards added while this tab is open go to the command zone. ${commandZone.cardLabel}: ${commandZone.min === commandZone.max ? commandZone.min : `${commandZone.min} to ${commandZone.max}`}.`
                : 'Cards added while this tab is open go to the sideboard.'}
          </p>
        )}
        {groups.map((group) => (
          <section key={group.key} className={styles.group}>
            <h3 className={styles.groupHead}>{group.label}<span>{group.count}</span></h3>
            <ul>
              {group.rows.map((row) => (
                <li
                  key={row.key}
                  className={styles.row}
                  onPointerEnter={(event) => row.card && onPreview(row.card, event.currentTarget.closest('aside')?.getBoundingClientRect() ?? event.currentTarget.getBoundingClientRect())}
                  onPointerLeave={() => onPreview(null)}
                >
                  <span className={styles.rowCount}>{row.entry.amount}</span>
                  <span className={styles.rowName}>{row.entry.cardName}</span>
                  <StyledMark card={row.entry.cardName} />
                  {identity && zone === 'cards' && row.card && !withinIdentity(row.card, identity) && (
                    <TriangleAlert size={14} className={styles.offColor} aria-label="Outside your commander's colors" />
                  )}
                  {row.card && <ManaCost cost={row.card.manaCostLeftStr} size="sm" />}
                  <span className={styles.rowActions}>
                    <button type="button" aria-label={`One less ${row.entry.cardName}`} onClick={() => onChange((current) => removeCard(current, zone, row.key))}><Minus size={14} /></button>
                    <button
                      type="button"
                      aria-label={`One more ${row.entry.cardName}`}
                      onClick={() => onChange((current) => {
                        const limit = limitFor(row.card, row.entry.cardName);
                        return copiesByName(current, row.entry.cardName) >= limit ? current : addCard(current, zone, { cardName: row.entry.cardName, setCode: row.entry.setCode ?? '', cardNumber: row.entry.cardNumber ?? '' });
                      })}
                    >
                      <Plus size={14} />
                    </button>
                    {commandZone && zone === 'cards' ? (
                      canCommand(row.card, format) && sideCount < commandZone.max && (
                        <button type="button" aria-label={`Make ${row.entry.cardName} your ${commandZone.cardLabel.toLowerCase()}`} title={`To the command zone`} onClick={() => onChange((current) => moveCard(current, 'cards', row.key))}>
                          <Crown size={14} />
                        </button>
                      )
                    ) : (
                      <button
                        type="button"
                        aria-label={zone === 'cards' ? 'Move one to sideboard' : 'Move one to deck'}
                        title={zone === 'cards' ? 'To sideboard' : 'To deck'}
                        onClick={() => onChange((current) => moveCard(current, zone, row.key))}
                      >
                        <ArrowLeftRight size={14} />
                      </button>
                    )}
                    <button type="button" aria-label={`Choose the printing of ${row.entry.cardName}`} title="Printing" onClick={() => setPrintingFor(row)}>
                      <Images size={14} />
                    </button>
                  </span>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>

      <ManaCurve curve={curve} className={styles.curve} />

      <BasicLands deck={deck} info={info} rules={rules} identity={identity} onChange={onChange} />

      {deck.source && <SourceLine source={deck.source} className={styles.source} />}

      <footer className={styles.deckFoot}>
        <Button variant="quiet" size="sm" icon={<ChevronLeft size={16} />} onClick={onDone}>{doneLabel}</Button>
        {!fixedFormat && <DropdownMenu.Root>
          <DropdownMenu.Trigger asChild>
            <Button variant="print" size="sm" icon={<ClipboardPaste size={16} />}>From a list…</Button>
          </DropdownMenu.Trigger>
          <DropdownMenu.Portal>
            <DropdownMenu.Content className={styles.menu} side="top" align="start" sideOffset={6} onCloseAutoFocus={(event) => event.preventDefault()}>
              <DropdownMenu.Item className={styles.menuItem} onSelect={() => openImport(null, intoDeck('replace'))}>
                <Replace size={16} aria-hidden="true" /> Replace deck
              </DropdownMenu.Item>
              <DropdownMenu.Item className={styles.menuItem} onSelect={() => openImport(null, intoDeck('add'))}>
                <ListPlus size={16} aria-hidden="true" /> Add to deck
              </DropdownMenu.Item>
            </DropdownMenu.Content>
          </DropdownMenu.Portal>
        </DropdownMenu.Root>}
        <ExportMenu getDeck={() => deck} side="top" />
        <Button variant="print" size="sm" icon={<BarChart3 size={16} />} onClick={() => setInsightsOpen(true)} disabled={mainCount === 0}>Numbers</Button>
        <Button variant="print" size="sm" icon={<ArrowLeftRight size={16} />} onClick={() => setPlansOpen(true)} disabled={mainCount === 0}>Sideboard plans</Button>
        <Button variant="decision" onClick={onPlay} disabled={mainCount === 0}>Play this deck</Button>
      </footer>
      <PrintingPicker
        name={printingFor?.entry.cardName ?? null}
        current={printingFor?.key ?? null}
        onClose={() => setPrintingFor(null)}
        onPick={(printing) => {
          const row = printingFor;
          if (row) onChange((current) => changePrinting(current, zone, row.key, printing));
        }}
      />
      <DeckInsights open={insightsOpen} onOpenChange={setInsightsOpen} deck={deck} info={info} />
      <SideboardPlansDialog open={plansOpen} onOpenChange={setPlansOpen} deck={deck} onChange={onChange} />
    </aside>
  );
}

const BASICS: { name: string; symbol: string }[] = [
  { name: 'Plains', symbol: 'w' },
  { name: 'Island', symbol: 'u' },
  { name: 'Swamp', symbol: 'b' },
  { name: 'Mountain', symbol: 'r' },
  { name: 'Forest', symbol: 'g' },
];

/** Basic lands without searching: steppers for the five basics, and a fill to the usual land count. */
function BasicLands({ deck, info, rules, identity, onChange }: {
  deck: DeckCardLists;
  info: ReadonlyMap<string, CardView>;
  rules: FormatRules;
  identity: string[] | null;
  onChange(update: (deck: DeckCardLists) => DeckCardLists): void;
}) {
  const size = rules.deckSize ?? Math.max(rules.minDeck, countZone(deck, 'cards'));
  const allowed = (identity ?? MANA_LETTERS) as ManaLetter[];
  // the command zone counts toward a commander deck's size, so the lands fill what is left of the main deck
  const suggestion = suggestLands(deck.cards, (entry) => info.get(entryKey(entry as DeckCardInfo)), rules.commandZone ? size - countZone(deck, 'sideboard') : size, allowed);
  const adding = Object.entries(suggestion.add);
  function fill() {
    onChange((current) => adding.reduce((next, [name, amount]) => {
      const existing = next.cards.find((candidate) => candidate.cardName === name);
      return addCard(next, 'cards', existing
        ? { cardName: name, setCode: existing.setCode ?? '', cardNumber: existing.cardNumber ?? '' }
        : { cardName: name, setCode: '', cardNumber: '' }, amount);
    }, current));
    notify('Lands added', adding.map(([name, amount]) => `${amount} ${name}`).join(', '));
  }
  return (
    <div className={styles.landsBlock}>
      <div className={styles.basics} aria-label="Basic lands">
        {BASICS.map((basic) => {
          const entry = deck.cards.find((candidate) => candidate.cardName === basic.name);
          const count = deck.cards.filter((candidate) => candidate.cardName === basic.name).reduce((sum, candidate) => sum + candidate.amount, 0);
          return (
            <div key={basic.name} className={styles.basic}>
              <i className={`ms ms-cost ms-${basic.symbol}`} aria-hidden="true" />
              <button type="button" aria-label={`One less ${basic.name}`} disabled={!entry} onClick={() => entry && onChange((current) => removeCard(current, 'cards', entryKey(entry)))}><Minus size={13} /></button>
              <b aria-label={`${count} ${basic.name}`}>{count}</b>
              <button
                type="button"
                aria-label={`One more ${basic.name}`}
                onClick={() => onChange((current) => addCard(current, 'cards', entry
                  ? { cardName: entry.cardName, setCode: entry.setCode ?? '', cardNumber: entry.cardNumber ?? '' }
                  : { cardName: basic.name, setCode: '', cardNumber: '' }))}
              >
                <Plus size={13} />
              </button>
            </div>
          );
        })}
      </div>
      {adding.length > 0 && (
        <button type="button" className={styles.fill} onClick={fill} title={`Brings the deck to ${suggestion.target} lands, split by the colors of its spells`}>
          <Mountain size={14} aria-hidden="true" /> Fill to {suggestion.target} lands: {adding.map(([name, amount]) => `${amount} ${name}`).join(', ')}
        </button>
      )}
    </div>
  );
}

/** The commander (or commanders) at the head of the list, with the colors the deck may use. */
function CommandZone({ deck, info, rules, identity, onChange, onPreview }: {
  deck: DeckCardLists;
  info: ReadonlyMap<string, CardView>;
  rules: FormatRules;
  identity: string[] | null;
  onChange(update: (deck: DeckCardLists) => DeckCardLists): void;
  onPreview(card: CardView | null, anchor?: DOMRect): void;
}) {
  const zone = rules.commandZone!;
  const commanders = deck.sideboard;
  return (
    <section className={styles.command} aria-label={zone.label}>
      {commanders.length === 0 ? (
        <p className={styles.commandEmpty}>
          <Crown size={16} aria-hidden="true" /> No {zone.cardLabel.toLowerCase()} yet. Point at a legendary creature in your deck and press its crown.
        </p>
      ) : (
        <>
          <ul className={styles.commanders}>
            {commanders.map((entry) => {
              const key = entryKey(entry);
              const card = info.get(key);
              return (
                <li
                  key={key}
                  className={styles.commander}
                  onPointerEnter={(event) => card && onPreview(card, event.currentTarget.closest('aside')?.getBoundingClientRect() ?? event.currentTarget.getBoundingClientRect())}
                  onPointerLeave={() => onPreview(null)}
                >
                  <div className={styles.commanderArt}>
                    <CardFace card={card ?? { name: entry.cardName }} size="small" />
                  </div>
                  <span className={styles.commanderName}>{entry.cardName}</span>
                  <button type="button" className={styles.commanderBack} aria-label={`Put ${entry.cardName} back in the deck`} title="Back to the deck" onClick={() => onChange((current) => moveCard(current, 'sideboard', key, entry.amount))}>
                    <ArrowLeftRight size={14} />
                  </button>
                </li>
              );
            })}
          </ul>
          <p className={styles.identity}>
            <span>Colors</span>
            {identity && identity.length > 0
              ? identity.map((letter) => <i key={letter} className={`ms ms-cost ms-${letter.toLowerCase()}`} aria-label={BASIC_FOR[letter as ManaLetter]} />)
              : <i className="ms ms-cost ms-c" aria-label="Colorless" />}
          </p>
        </>
      )}
    </section>
  );
}

function Preview({ card, anchor, side }: { card: CardView; anchor: DOMRect; side: 'auto' | 'left' }) {
  const width = 300;
  const height = width * (88 / 63);
  const gap = 16;
  // beside the hovered card, on whichever side has room; never over the deck list on the right
  const deckEdge = window.innerWidth - 400;
  const fitsRight = side === 'auto' && anchor.right + gap + width <= deckEdge;
  const left = fitsRight ? anchor.right + gap : Math.max(gap, anchor.left - gap - width);
  const top = Math.max(12, Math.min(window.innerHeight - height - 12, anchor.top + anchor.height / 2 - height / 2));
  return (
    <div className={styles.preview} style={{ left, top, width }} aria-hidden="true">
      <CardFace card={card} size="large" />
    </div>
  );
}

/** A card wearing one of the account's card styles (capsule machines) is marked in the list. */
function StyledMark({ card }: { card: string }) {
  const worn = useMyCardStyles()?.[card];
  return worn ? <Sparkles size={13} className={styles.styledMark} aria-label="Wears a card style" /> : null;
}
