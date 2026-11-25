import { X } from 'lucide-react';
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type ComponentType, type CSSProperties, type KeyboardEvent as ReactKeyboardEvent } from 'react';
import { createPortal } from 'react-dom';
import { useNavigate, useParams } from 'react-router-dom';
import type { CardView } from '../../protocol/generated/views';
import { commanderIdentity, formatMenu, formatRules, type FormatRules } from '../../core/decks/formats';
import { deckStorage } from '../../core/decks/DeckStorageService';
import type { DeckCardLists } from '../../core/decks/types';
import { useServerState } from '../queries';
import { STARTER_PREFIX, useDecks } from '../stores/decks';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { BINDER_MIN, PANEL_DEFAULT, PANEL_MIN, useBuilderPrefs, type BuilderLayout } from './builderPrefs';
import { useCardInfo, useCardInfoStore } from './cardInfo';
import { CardInspect } from './CardInspect';
import { zoneLabel } from './zones';
import { placePreview } from './previewPlacement';
import { Collection } from './Collection';
import { DeckPanel } from './DeckPanel';
import type { DraggedCard, PreviewKind } from './deckDrag';
import { useImportPaste } from './import/useImportShortcuts';
import { Splitter } from './Splitter';
import { useDeckHistory } from './useDeckHistory';
import {
  addCard, copiesByName, copyLimit, countZone, entryKey, finalizeDeck, printingOf, removeCard, setCategory, zoneList, type DeckRow, type DeckZone,
  type PrintingRef,
} from './deckModel';
import styles from './DeckBuilder.module.css';

const DEFAULT_FORMAT = 'Constructed - Freeform';
const SPLITTER_W = 14;

/** What the card pool on the left gets from the builder (the collection of every card, or a player's own binder). */
export interface BinderProps {
  format: string;
  identity: string[] | null;
  counts: ReadonlyMap<string, number>;
  limitOf(card: CardView): number;
  onAdd(card: CardView): void;
  onRemove(card: CardView): void;
  onPreview(card: CardView | null, anchor?: DOMRect): void;
  /** opens a card up close (#250) */
  onInspect(card: CardView): void;
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
  /** shown above the deck list, given the deck as it is on screen (the League's missing cards and Craft missing) */
  Banner?: ComponentType<{ deck: DeckCardLists }>;
}

/**
 * The binder sheet covers most of the deck under it: Tab and Shift+Tab stay inside the sheet while it's open (Done and
 * Escape leave). The deck isn't made inert, so binder cards can still be dropped on what shows of it.
 */
function keepTabInside(event: ReactKeyboardEvent<HTMLDivElement>) {
  if (event.key !== 'Tab') return;
  const focusable = [...event.currentTarget.querySelectorAll<HTMLElement>(
    'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
  )].filter((element) => element.getClientRects().length > 0);
  if (focusable.length === 0) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
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
  const { deck, change: setDeck, undo, redo, canUndo, canRedo } = useDeckHistory(initial);
  const [zone, setZone] = useState<DeckZone>('cards');
  // a phone shows one pane at a time: the binder to pick from, or the deck list (the page's CSS decides)
  const [pane, setPane] = useState<'binder' | 'deck'>('binder');
  const [preview, setPreview] = useState<PreviewState | null>(null);
  const [inspected, setInspected] = useState<CardView | null>(null);
  const onInspect = useCallback((card: CardView) => {
    setPreview(null);
    setInspected(card);
  }, []);
  // a deck pasted here becomes its own deck; "Replace / Add from a list…" (under More) brings cards into this one
  useImportPaste();
  const deckEntries = useMemo(() => [...deck.cards, ...deck.sideboard], [deck.cards, deck.sideboard]);
  const allEntries = useMemo(() => [...deckEntries, ...zoneList(deck, 'maybeboard')], [deckEntries, deck]);
  const info = useCardInfo(allEntries);
  const format = deck.format || DEFAULT_FORMAT;
  const rules = formatRules(format);
  const server = useServerState();
  const formats = useMemo(() => formatMenu(server.data?.deckTypes ?? [DEFAULT_FORMAT]), [server.data]);
  const identity = useCommanderIdentity(deck, info, rules);

  // the layout: the binder beside the deck (split, with a seam to drag) or over it (deck first); a phone has its switch
  const phone = useMediaQuery('(max-width: 720px), (max-height: 480px)');
  const storedLayout = useBuilderPrefs((state) => state.layout);
  const layout: BuilderLayout = phone ? 'split' : storedLayout;
  const storedWidth = useBuilderPrefs((state) => state.panelWidth);
  const setPrefs = useBuilderPrefs((state) => state.set);
  const [dragWidth, setDragWidth] = useState<number | null>(null);
  const pageRef = useRef<HTMLDivElement>(null);
  const binderRef = useRef<HTMLDivElement>(null);
  const deckPaneRef = useRef<HTMLDivElement>(null);
  const pageWidth = useWidth(pageRef);
  const maxPanel = Math.max(PANEL_MIN, pageWidth - BINDER_MIN - SPLITTER_W);
  const panelWidth = Math.min(maxPanel, Math.max(PANEL_MIN, dragWidth ?? storedWidth));
  const [binderOpen, setBinderOpen] = useState(false);
  const binderShown = layout === 'split' || binderOpen;
  const openBinder = useCallback(() => {
    setBinderOpen(true);
    // after the drawer is shown: the search takes the keys
    requestAnimationFrame(() => binderRef.current?.querySelector<HTMLInputElement>('input[aria-label="Search cards"], input[type="search"], input')?.focus());
  }, []);
  const closeBinder = useCallback(() => {
    // focus inside the drawer would be left in an inert pane: it goes back to the button that opened it
    const hadFocus = !!binderRef.current?.contains(document.activeElement);
    setBinderOpen(false);
    setPreview(null);
    if (hadFocus) deckPaneRef.current?.querySelector<HTMLButtonElement>('[data-add-cards]')?.focus();
  }, []);
  const onLayout = useCallback((next: BuilderLayout) => {
    setPrefs({ layout: next });
    setBinderOpen(false);
    setPreview(null);
  }, [setPrefs]);

  // "/" searches the binder (opening it over a deck-first page); Escape puts the binder away again
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
      const target = event.target as HTMLElement | null;
      if (event.key === '/' && !target?.closest('input, textarea, select, [contenteditable="true"], [role="dialog"], [role="menu"]')) {
        event.preventDefault();
        if (layout === 'deck') openBinder();
        else binderRef.current?.querySelector<HTMLInputElement>('input')?.focus();
      } else if (event.key === 'Escape' && layout === 'deck' && binderOpen && !target?.closest('[role="dialog"], [role="menu"]')) {
        // handled: Career's Escape-to-go-back leaves the screen alone
        event.preventDefault();
        closeBinder();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [layout, binderOpen, openBinder, closeBinder]);

  // autosave: the deck in storage always matches the screen, a moment later (and flushes on unmount so the last edit isn't lost)
  const [savedDeck, setSavedDeck] = useState(initial);
  const saving = deck !== savedDeck;
  const pendingSave = useRef<{ deck: DeckCardLists; mode: BuilderMode | undefined } | null>(null);
  useEffect(() => {
    pendingSave.current = saving ? { deck, mode } : null;
    if (!saving) return;
    const timer = setTimeout(() => {
      pendingSave.current = null;
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
    const latest = pendingSave.current;
    if (latest) {
      pendingSave.current = null;
      const finished = finalizeDeck(latest.deck, useCardInfoStore.getState().info);
      if (latest.mode) latest.mode.save(finished);
      else void deckStorage.saveDeck(finished).catch(() => undefined);
    }
    if (!mode) void useDecks.getState().refresh();
  }, [mode]);

  // copies in the deck and sideboard, by name: what the binder marks and the copy limit counts (the maybeboard never does)
  const counts = useMemo(() => {
    const map = new Map<string, number>();
    for (const entry of deckEntries) map.set(entry.cardName.toLowerCase(), (map.get(entry.cardName.toLowerCase()) ?? 0) + entry.amount);
    return map;
  }, [deckEntries]);
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
  /** one copy of a printing into a zone, unless the deck already holds as many as it may (the maybeboard has no limit) */
  const addPrinting = useCallback((printing: PrintingRef, card: CardView | undefined, to: DeckZone, category?: string | null) => {
    const name = printing.cardName;
    const limit = to === 'maybeboard' ? Infinity : limitFor(card, name);
    if (copiesByName(deck, name) >= limit) {
      atLimit(card, name, limit);
      return;
    }
    setDeck((current) => {
      if (copiesByName(current, name) >= limit) return current;
      const added = addCard(current, to, printing);
      return category === undefined ? added : setCategory(added, name, category);
    });
  }, [deck, limitFor, atLimit, setDeck]);
  const add = useCallback((card: CardView) => addPrinting(printingOf(card), card, zone), [addPrinting, zone]);
  const removeOne = useCallback((card: CardView) => {
    setDeck((current) => {
      const name = (card.name ?? '').toLowerCase();
      // prefer the zone being edited, then the others
      for (const from of [zone, ...(['cards', 'sideboard', 'maybeboard'] as DeckZone[]).filter((each) => each !== zone)]) {
        const entry = zoneList(current, from).find((candidate) => candidate.cardName.toLowerCase() === name);
        if (entry) return removeCard(current, from, entryKey(entry));
      }
      return current;
    });
  }, [zone, setDeck]);
  // a card in the deck view: one more of the same printing
  const addRow = useCallback((row: DeckRow) => addPrinting(
    { cardName: row.entry.cardName, setCode: row.entry.setCode ?? '', cardNumber: row.entry.cardNumber ?? '' }, row.card, zone,
  ), [addPrinting, zone]);
  const onDropBinderCard = useCallback((card: DraggedCard, to: DeckZone, category?: string | null) => {
    if (!card.printing) return;
    addPrinting(card.printing, useCardInfoStore.getState().info.get(card.key), to, category);
    if (to !== zone) notify(`${card.name} added`, `To the ${zoneLabel(to, !!rules.commandZone).toLowerCase()}.`);
  }, [addPrinting, zone, rules.commandZone]);

  // binder cards preview beside the card, never over the deck beside them; deck rows preview outside the deck panel
  const onPreview = useCallback((card: CardView | null, anchor?: DOMRect) => setPreview(card && anchor
    ? { card, anchor, avoid: layout === 'split' ? deckPaneRef.current?.getBoundingClientRect() : undefined }
    : null), [layout]);
  const onDeckPreview = useCallback((card: CardView | null, anchor?: DOMRect, kind?: PreviewKind) => setPreview(card && anchor
    ? { card, anchor, outside: kind === 'row' ? deckPaneRef.current?.getBoundingClientRect() : undefined }
    : null), []);

  const Binder = mode?.Binder;
  return (
    <div
      ref={pageRef}
      className={styles.page}
      data-pane={pane}
      data-layout={layout}
      data-binder={layout === 'deck' ? (binderOpen ? 'open' : 'closed') : undefined}
      style={{ '--panel-w': `${panelWidth}px`, '--splitter-w': `${SPLITTER_W}px` } as CSSProperties}
    >
      <div className={styles.paneSwitch} role="group" aria-label="Show">
        <button type="button" aria-pressed={pane === 'binder'} className={pane === 'binder' ? styles.paneOn : styles.paneOff} onClick={() => setPane('binder')}>Card binder</button>
        <button type="button" aria-pressed={pane === 'deck'} className={pane === 'deck' ? styles.paneOn : styles.paneOff} onClick={() => setPane('deck')}>Deck list · {countZone(deck, 'cards')}</button>
      </div>
      <div
        ref={binderRef}
        className={styles.binderPane}
        inert={!binderShown}
        role={layout === 'deck' ? 'region' : undefined}
        aria-label={layout === 'deck' ? 'Card binder' : undefined}
        onKeyDown={layout === 'deck' && binderOpen ? keepTabInside : undefined}
      >
        {layout === 'deck' && (
          <div className={styles.drawerHead}>
            <span className={styles.drawerTitle}>Add cards</span>
            <span className={styles.drawerTarget}>to the {zoneLabel(zone, !!rules.commandZone).toLowerCase()}</span>
            <Button variant="quiet" size="sm" icon={<X size={16} />} onClick={closeBinder} aria-label="Close the binder">Done</Button>
          </div>
        )}
        {Binder ? (
          <Binder format={format} identity={identity} counts={counts} limitOf={limitOf} onAdd={add} onRemove={removeOne} onPreview={onPreview} onInspect={onInspect} />
        ) : (
          <Collection format={format === 'Limited' ? '' : format} identity={identity} counts={counts} limitOf={limitOf} onAdd={add} onRemove={removeOne} onPreview={onPreview} onInspect={onInspect} />
        )}
      </div>
      {layout === 'split' && !phone && (
        <Splitter
          width={panelWidth}
          min={PANEL_MIN}
          max={maxPanel}
          defaultWidth={PANEL_DEFAULT}
          onChange={setDragWidth}
          onCommit={(width) => {
            setDragWidth(null);
            setPrefs({ panelWidth: width });
          }}
        />
      )}
      <div ref={deckPaneRef} className={styles.deckPane}>
        <DeckPanel
          deck={deck}
          zone={zone}
          onZone={setZone}
          info={info}
          formats={formats}
          rules={rules}
          identity={identity}
          saving={saving}
          limitFor={limitFor}
          onAtLimit={atLimit}
          keepsExtras={!mode}
          fixedFormat={!!mode}
          doneLabel={mode?.doneLabel ?? 'Decks'}
          banner={mode?.Banner ? <mode.Banner deck={deck} /> : null}
          layout={layout}
          onLayout={phone ? null : onLayout}
          onOpenBinder={openBinder}
          onChange={setDeck}
          history={{ undo, redo, canUndo, canRedo }}
          onDropBinderCard={onDropBinderCard}
          onAddRow={addRow}
          onPreview={onDeckPreview}
          onInspect={onInspect}
          onDone={mode ? mode.onDone : () => navigate('/decks')}
          onPlay={mode ? mode.onPlay : () => {
            useDecks.getState().select(deck.id!);
            navigate('/');
          }}
        />
      </div>
      {preview && <Preview {...preview} />}
      <CardInspect card={inspected} onClose={() => setInspected(null)} />
    </div>
  );
}

function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(() => typeof window !== 'undefined' && window.matchMedia?.(query).matches);
  useEffect(() => {
    const list = window.matchMedia?.(query);
    if (!list) return;
    const update = () => setMatches(list.matches);
    update();
    list.addEventListener('change', update);
    return () => list.removeEventListener('change', update);
  }, [query]);
  return !!matches;
}

/** An element's width, kept current as the window or the shell around it changes. */
function useWidth(ref: React.RefObject<HTMLElement | null>): number {
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return;
    setWidth(element.getBoundingClientRect().width);
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(([entry]) => setWidth(entry.contentRect.width));
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref]);
  return width || window.innerWidth;
}

/** The command zone's color identity once it holds a commander (null: not a commander deck, or no commander yet). */
function useCommanderIdentity(deck: DeckCardLists, info: ReadonlyMap<string, CardView>, rules: FormatRules): string[] | null {
  return useMemo(() => {
    if (!rules.commandZone) return null;
    const commanders = deck.sideboard.map((entry) => info.get(entryKey(entry))).filter((card): card is CardView => !!card);
    return commanders.length > 0 ? commanderIdentity(commanders) : null;
  }, [deck.sideboard, info, rules.commandZone]);
}


interface PreviewState {
  card: CardView;
  anchor: DOMRect;
  /** keep clear of this box (the deck panel beside the binder) */
  avoid?: DOMRect;
  /** prefer a place beside this box, outside it (a list row's preview goes beside the deck panel) */
  outside?: DOMRect;
}

function Preview({ card, anchor, avoid, outside }: PreviewState) {
  const width = 300;
  const height = width * (88 / 63);
  // a double-faced card shows its back face beside the front, smaller, as the table's card zoom does (#163)
  const back = card.transformable ? card.secondCardFace : undefined;
  const span = back ? width * 1.62 + 12 : width;
  const left = placePreview(anchor, span, window.innerWidth, avoid, outside);
  const top = Math.max(12, Math.min(window.innerHeight - height - 12, anchor.top + anchor.height / 2 - height / 2));
  // on the page itself: inside Career's screen, whose entrance animation moves it, a fixed box is placed against that
  // screen rather than the window, and the clamp above pushed the preview off the bottom (#250)
  return createPortal(
    <div className={styles.preview} style={{ left, top, width: span }} aria-hidden="true">
      <div className={styles.previewFaces}>
        <CardFace card={card} size="large" style={{ width }} />
        {back && <CardFace card={back} face="back" size="normal" style={{ width: width * 0.62 }} />}
      </div>
    </div>,
    document.body,
  );
}
