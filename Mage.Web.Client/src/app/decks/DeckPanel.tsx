import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import {
  ArrowLeftRight, BarChart3, Check, ChevronDown, ChevronLeft, Coins, Columns2, Crown, Image as ImageIcon, LayoutGrid,
  Layers, List, ListPlus, Minus, Mountain, MoreHorizontal, PanelLeftClose, Plus, Redo2, Replace, RotateCcw, Search, TriangleAlert, Undo2,
} from 'lucide-react';
import { useId, useMemo, useState, type DragEvent, type ReactNode } from 'react';
import type { CardView } from '../../protocol/generated/views';
import { BASIC_FOR, MANA_LETTERS, suggestLands, type ManaLetter } from '../../core/decks/analysis';
import { canCommand, formatRules, type FormatRules } from '../../core/decks/formats';
import { appendDeckCardLists } from '../../core/decks/merge';
import type { DeckCardInfo, DeckCardLists } from '../../core/decks/types';
import { openImport, type ImportTarget } from '../stores/importSheet';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { CARD_SIZE_MAX, CARD_SIZE_MIN, useBuilderPrefs, type BuilderLayout, type DeckViewKind, type SectionKey } from './builderPrefs';
import { CardMenu, type CardMenuActions } from './CardMenu';
import { zoneLabel, ZONES } from './zones';
import { CategoryDialog, type CategoryTask } from './CategoryDialog';
import { CoverDialog } from './CoverDialog';
import { DeckInsights } from './DeckInsights';
import { GROUP_BY_LABELS, groupRows, SORT_BY_LABELS, type GroupBy, type SortBy, type ViewGroup } from './deckGroups';
import {
  addCard, categoryOf, changePrinting, countZone, deckCategories, entryKey, manaCurve, movableCopies, moveCard, removeCard, renameCategory, setCategory,
  zoneList, type DeckRow, type DeckZone,
} from './deckModel';
import { carriesCard, readDraggedCard, type DraggedCard, type PreviewKind } from './deckDrag';
import { DeckView } from './DeckView';
import { ExportMenu } from './ExportMenu';
import { ManaCurve } from './ManaCurve';
import { PrintingPicker } from './PrintingPicker';
import { SideboardPlansDialog } from './SideboardPlansDialog';
import { SourceLine } from './import/SourceLine';
import { TokenArtDialog } from './TokenArtDialog';
import { useValidation } from './useValidation';
import styles from './DeckBuilder.module.css';
import panel from './DeckPanel.module.css';

const DEFAULT_FORMAT = 'Constructed - Freeform';
type Change = (update: (deck: DeckCardLists) => DeckCardLists, merge?: string) => void;

export interface DeckPanelProps {
  deck: DeckCardLists;
  zone: DeckZone;
  onZone(zone: DeckZone): void;
  info: ReadonlyMap<string, CardView>;
  formats: { label: string; types: string[] }[];
  rules: FormatRules;
  identity: string[] | null;
  saving: boolean;
  limitFor(card: CardView | undefined, name: string): number;
  /** says why a card at its limit can't take another copy */
  onAtLimit(card: CardView | undefined, name: string, limit: number): void;
  /** the format is the mode's (Career decks are Freeform), not the player's to pick */
  fixedFormat: boolean;
  doneLabel: string;
  /** the mode's banner, under the deck's name */
  banner?: ReactNode;
  layout: BuilderLayout;
  /** null where the layout can't change (a phone shows one pane at a time) */
  onLayout: ((layout: BuilderLayout) => void) | null;
  /** opens the binder over the deck (the deck-first layout) */
  onOpenBinder(): void;
  onChange: Change;
  history: { undo(): void; redo(): void; canUndo: boolean; canRedo: boolean };
  /** a card from the binder dropped on the deck, a tab or a category */
  onDropBinderCard(card: DraggedCard, zone: DeckZone, category?: string | null): void;
  onAddRow(row: DeckRow): void;
  onPreview(card: CardView | null, anchor?: DOMRect, kind?: PreviewKind): void;
  onInspect(card: CardView): void;
  onDone(): void;
  onPlay(): void;
  /**
   * The deck keeps a maybeboard and categories. Free-play decks do; Career and League decks save only what the server
   * plays, so there the two are left out rather than lost on the next load.
   */
  keepsExtras: boolean;
}

export function DeckPanel(props: DeckPanelProps) {
  const {
    deck, zone, onZone, info, formats, rules, identity, saving, fixedFormat, doneLabel, banner, layout, onLayout, onOpenBinder,
    onChange, history, onAtLimit, limitFor, onDropBinderCard, onAddRow, onPreview, onInspect, onDone, onPlay, keepsExtras,
  } = props;
  const format = deck.format || DEFAULT_FORMAT;
  const view = useBuilderPrefs((state) => state.view);
  const storedGroupBy = useBuilderPrefs((state) => state.groupBy);
  const groupBy: GroupBy = !keepsExtras && storedGroupBy === 'category' ? 'type' : storedGroupBy;
  const zones = keepsExtras ? ZONES : ZONES.filter((each) => each !== 'maybeboard');
  const sortBy = useBuilderPrefs((state) => state.sortBy);
  const cardSize = useBuilderPrefs((state) => state.cardSize);
  const commandZone = rules.commandZone;
  const mainCount = countZone(deck, 'cards');
  const sideCount = countZone(deck, 'sideboard');
  const maybeCount = countZone(deck, 'maybeboard');
  // a commander deck counts its command zone; other decks count the main deck only
  const total = commandZone ? mainCount + sideCount : mainCount;
  const validation = useValidation(deck, format);
  const [problemsOpen, setProblemsOpen] = useState(false);
  const problems = validation.data?.errors ?? [];
  const listed = formats.some((group) => group.types.includes(format));

  const entries = zoneList(deck, zone);
  const groups = useMemo(
    () => groupRows(entries, info, { groupBy, sortBy, categories: deck.categories, keepEmpty: view === 'stacks' }),
    [entries, info, groupBy, sortBy, deck.categories, view],
  );
  const categories = useMemo(() => deckCategories(deck), [deck]);
  const curve = useMemo(() => manaCurve(deck.cards, info), [deck.cards, info]);

  const [printingFor, setPrintingFor] = useState<{ row: DeckRow; zone: DeckZone } | null>(null);
  const [categoryTask, setCategoryTask] = useState<CategoryTask | null>(null);
  const [insightsOpen, setInsightsOpen] = useState(false);
  const [plansOpen, setPlansOpen] = useState(false);
  const [tokensOpen, setTokensOpen] = useState(false);
  const [coverOpen, setCoverOpen] = useState(false);

  const roomInCommand = !!commandZone && sideCount < commandZone.max;
  /**
   * Copies moved between zones, within the copy limit (out of the maybeboard) and the command zone's rules; says why
   * when fewer move than asked. True when any moved.
   */
  const move = (from: DeckZone, key: string, card: CardView | undefined, name: string, amount: number, to: DeckZone): boolean => {
    const moveRules = {
      limit: limitFor(card, name),
      command: commandZone ? { allowed: canCommand(card, format), max: commandZone.max } : null,
    };
    const asked = Math.min(amount, zoneList(deck, from).find((entry) => entryKey(entry) === key)?.amount ?? 0);
    const moving = movableCopies(deck, from, key, amount, to, moveRules);
    if (moving < asked) {
      if (commandZone && to === 'sideboard') {
        if (!canCommand(card, format)) notify(`${name} can't lead the deck`, `Only a card that can be your ${commandZone.cardLabel.toLowerCase()} goes in the command zone.`);
        else notify('The command zone is full', `It holds ${commandZone.max}. Put one back in the deck first.`);
      } else {
        onAtLimit(card, name, limitFor(card, name));
      }
    }
    if (moving === 0) return false;
    onChange((current) => moveCard(current, from, key, movableCopies(current, from, key, amount, to, moveRules), to));
    return true;
  };
  const actions: CardMenuActions = {
    onMove: (row, from, to, amount) => move(from, row.key, row.card, row.entry.cardName, amount, to),
    onPrinting: (row, from) => setPrintingFor({ row, zone: from }),
    onInspect: (row) => row.card && onInspect(row.card),
    onCategory: (row, category) => onChange((current) => setCategory(current, row.entry.cardName, category)),
    onNewCategory: (row) => setCategoryTask({ kind: 'new', cardName: row.entry.cardName }),
    onRemoveAll: (row, from) => {
      onPreview(null);
      onChange((current) => removeCard(current, from, row.key, row.entry.amount));
    },
  };
  const menu = (row: DeckRow, className: string) => (
    <CardMenu
      row={row}
      zone={zone}
      commandZone={!!commandZone}
      canCommand={zone !== 'sideboard' && roomInCommand && canCommand(row.card, format)}
      zones={zones}
      categories={keepsExtras ? categories : null}
      category={categoryOf(deck, row.entry.cardName)}
      actions={actions}
      className={className}
    />
  );

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

  const dropOnZone = (to: DeckZone) => (card: DraggedCard) => {
    if (card.zone === 'binder') onDropBinderCard(card, to);
    else if (card.zone !== to) {
      const amount = zoneList(deck, card.zone).find((entry) => entryKey(entry) === card.key)?.amount ?? 1;
      if (move(card.zone, card.key, info.get(card.key), card.name, amount, to)) notify(`${card.name} moved`, `To the ${zoneLabel(to, !!commandZone).toLowerCase()}.`);
    }
  };
  const onDropOnGroup = (card: DraggedCard, group: ViewGroup) => {
    if (card.zone === 'binder') onDropBinderCard(card, zone, group.category ?? null);
    else onChange((current) => setCategory(current, card.name, group.category ?? null));
  };

  return (
    <aside className={panel.panel} aria-label="Deck" data-deck-pane>
      <header className={panel.head}>
        <input
          className={panel.name}
          value={deck.name ?? ''}
          onChange={(event) => {
            const name = event.target.value;
            onChange((current) => ({ ...current, name }), 'name');
          }}
          aria-label="Deck name"
          maxLength={60}
        />
        <div className={panel.meta}>
          <select
            className={panel.format}
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
          <span className={panel.total}>{total}{rules.deckSize ? <small> / {rules.deckSize}</small> : <small> cards</small>}</span>
          {validation.data && (validation.data.valid ? (
            <span className={[styles.chip, styles.chipOk].join(' ')}><Check size={15} aria-hidden="true" /> Legal</span>
          ) : (
            <button type="button" className={[styles.chip, styles.chipBad].join(' ')} onClick={() => setProblemsOpen((open) => !open)} aria-expanded={problemsOpen}>
              <TriangleAlert size={15} aria-hidden="true" /> {problems.length} {problems.length === 1 ? 'problem' : 'problems'}
            </button>
          ))}
          <span className={panel.saved} aria-live="polite">{saving ? 'Saving…' : 'Saved'}</span>
        </div>
      </header>
      {banner}
      {problemsOpen && problems.length > 0 && (
        <ul className={styles.problems}>
          {problems.slice(0, 12).map((problem, index) => (
            <li key={index}><b>{problem.group}</b> {problem.message}</li>
          ))}
        </ul>
      )}

      <div className={panel.toolbar}>
        <div className={panel.zoneTabs} role="tablist" aria-label="Deck zone">
          {zones.map((each) => (
            <ZoneTab
              key={each}
              selected={zone === each}
              label={each === 'maybeboard' ? 'Maybe' : zoneLabel(each, !!commandZone)}
              title={each === 'maybeboard' ? 'Maybeboard: cards you are thinking about. Never played.' : undefined}
              count={each === 'cards' ? mainCount : each === 'sideboard' ? sideCount : maybeCount}
              onSelect={() => onZone(each)}
              onDrop={dropOnZone(each)}
            />
          ))}
        </div>
        <ViewControls
          groupBy={groupBy}
          groupOptions={keepsExtras ? null : ['category']}
          layout={layout}
          onLayout={onLayout}
          history={history}
          add={layout === 'deck' && (
            <Button variant="print" size="sm" icon={<Search size={15} />} onClick={onOpenBinder} data-add-cards title={`Search the binder (/). Cards go to the ${zoneLabel(zone, !!commandZone).toLowerCase()}.`}>
              Add cards
            </Button>
          )}
        />
      </div>

      <DeckDropArea zone={zone} onDrop={(card) => card.zone === 'binder' && onDropBinderCard(card, zone)}>
        <div className={panel.rail}>
          {commandZone && (
            <div className={panel.command}>
              <PanelSection id="command" title={commandZone.label} summary={deck.sideboard.map((entry) => entry.cardName).join(' · ') || 'Empty'}>
                <CommandZone deck={deck} info={info} rules={rules} identity={identity} onChange={onChange} onPreview={onPreview} />
              </PanelSection>
            </div>
          )}
          <div className={panel.stats}>
            <PanelSection id="curve" title="Curve" summary={curveSummary(curve)}>
              <ManaCurve curve={curve} className={panel.curve} />
            </PanelSection>
            <PanelSection id="lands" title="Basic lands" summary={landSummary(deck, info)}>
              <BasicLands deck={deck} info={info} rules={rules} identity={identity} onChange={onChange} />
            </PanelSection>
            {deck.source && <SourceLine source={deck.source} className={styles.source} />}
          </div>
        </div>
        <div className={panel.cards}>
          <DeckView
            kind={view}
            groups={groups}
            zone={zone}
            groupBy={groupBy}
            cardSize={cardSize}
            identity={identity}
            empty={emptyNote(zone, rules, layout)}
            menu={menu}
            onAdd={onAddRow}
            onRemove={(row) => onChange((current) => removeCard(current, zone, row.key))}
            onPreview={onPreview}
            onDropOnGroup={onDropOnGroup}
            onRenameCategory={(category) => setCategoryTask({ kind: 'rename', category })}
          />
        </div>
      </DeckDropArea>

      <footer className={panel.foot}>
        <button type="button" className={panel.back} onClick={onDone} aria-label={doneLabel} title={`Back to ${doneLabel.toLowerCase()}`}>
          <ChevronLeft size={18} aria-hidden="true" />
        </button>
        <ExportMenu getDeck={() => deck} side="top" />
        <DropdownMenu.Root>
          <DropdownMenu.Trigger asChild>
            <Button variant="print" size="sm" icon={<MoreHorizontal size={16} />} aria-label="More deck tools" title="More: from a list, numbers, sideboard plans, token art, cover" />
          </DropdownMenu.Trigger>
          <DropdownMenu.Portal>
            <DropdownMenu.Content className={styles.menu} side="top" align="start" sideOffset={6} collisionPadding={12} onCloseAutoFocus={(event) => event.preventDefault()}>
              {!fixedFormat && (
                <>
                  <DropdownMenu.Item className={styles.menuItem} onSelect={() => openImport(null, intoDeck('replace'))}>
                    <Replace size={16} aria-hidden="true" /> Replace from a list…
                  </DropdownMenu.Item>
                  <DropdownMenu.Item className={styles.menuItem} onSelect={() => openImport(null, intoDeck('add'))}>
                    <ListPlus size={16} aria-hidden="true" /> Add from a list…
                  </DropdownMenu.Item>
                  <DropdownMenu.Separator className={styles.menuSeparator} />
                </>
              )}
              <DropdownMenu.Item className={styles.menuItem} disabled={mainCount === 0} onSelect={() => setInsightsOpen(true)}>
                <BarChart3 size={16} aria-hidden="true" /> Numbers
              </DropdownMenu.Item>
              <DropdownMenu.Item className={styles.menuItem} disabled={mainCount === 0} onSelect={() => setPlansOpen(true)}>
                <ArrowLeftRight size={16} aria-hidden="true" /> Sideboard plans
              </DropdownMenu.Item>
              <DropdownMenu.Item className={styles.menuItem} onSelect={() => setTokensOpen(true)}>
                <Coins size={16} aria-hidden="true" /> Token art
              </DropdownMenu.Item>
              {!fixedFormat && (
                <DropdownMenu.Item className={styles.menuItem} disabled={mainCount + sideCount === 0} onSelect={() => setCoverOpen(true)}>
                  <ImageIcon size={16} aria-hidden="true" /> Cover
                </DropdownMenu.Item>
              )}
            </DropdownMenu.Content>
          </DropdownMenu.Portal>
        </DropdownMenu.Root>
        <Button variant="decision" className={panel.play} onClick={onPlay} disabled={mainCount === 0}>Play this deck</Button>
      </footer>

      <PrintingPicker
        name={printingFor?.row.entry.cardName ?? null}
        current={printingFor?.row.key ?? null}
        onClose={() => setPrintingFor(null)}
        onPick={(printing) => {
          const target = printingFor;
          if (target) onChange((current) => changePrinting(current, target.zone, target.row.key, printing));
        }}
      />
      <CategoryDialog
        key={categoryTask ? JSON.stringify(categoryTask) : 'none'}
        task={categoryTask}
        onClose={() => setCategoryTask(null)}
        onSave={(task, label) => onChange((current) => (task.kind === 'new' ? setCategory(current, task.cardName, label) : renameCategory(current, task.category, label)))}
      />
      <DeckInsights open={insightsOpen} onOpenChange={setInsightsOpen} deck={deck} info={info} />
      <SideboardPlansDialog open={plansOpen} onOpenChange={setPlansOpen} deck={deck} onChange={onChange} />
      <TokenArtDialog open={tokensOpen} onOpenChange={setTokensOpen} deck={deck} onChange={onChange} />
      <CoverDialog
        open={coverOpen}
        onOpenChange={setCoverOpen}
        deck={deck}
        onChange={(next) => onChange((current) => ({ ...current, coverCard: next.coverCard, coverPinned: next.coverPinned }))}
      />
    </aside>
  );
}

function emptyNote(zone: DeckZone, rules: FormatRules, layout: BuilderLayout): string {
  const source = layout === 'deck' ? 'Press Add cards, or /, and click cards to add them' : 'Click cards on the left to add them';
  if (zone === 'cards') return `${source}.`;
  if (zone === 'maybeboard') return `${source} to the maybeboard: cards you are thinking about. They are kept with the deck but never played.`;
  const command = rules.commandZone;
  return command
    ? `${source} to the command zone. ${command.cardLabel}: ${command.min === command.max ? command.min : `${command.min} to ${command.max}`}.`
    : `${source} to the sideboard.`;
}

function curveSummary(curve: number[]): string {
  const spells = curve.reduce((sum, value) => sum + value, 0);
  if (spells === 0) return 'No spells yet';
  const average = curve.reduce((sum, value, index) => sum + value * index, 0) / spells;
  return `${spells} spells · average ${average.toFixed(1)}`;
}

function landSummary(deck: DeckCardLists, info: ReadonlyMap<string, CardView>): string {
  const lands = deck.cards.filter((entry) => info.get(entryKey(entry))?.cardTypes?.includes('LAND')).reduce((sum, entry) => sum + entry.amount, 0);
  return `${lands} ${lands === 1 ? 'land' : 'lands'}`;
}

/** A zone tab; a deck card dropped on it moves there, a binder card is added there. */
function ZoneTab({ selected, label, title, count, onSelect, onDrop }: {
  selected: boolean;
  label: string;
  title?: string;
  count: number;
  onSelect(): void;
  onDrop(card: DraggedCard): void;
}) {
  const [over, setOver] = useState(false);
  return (
    <button
      type="button"
      role="tab"
      aria-selected={selected}
      title={title}
      className={selected ? panel.tabOn : panel.tab}
      data-over={over || undefined}
      onClick={onSelect}
      onDragOver={(event) => {
        if (!carriesCard(event)) return;
        event.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(event) => {
        setOver(false);
        const card = readDraggedCard(event);
        if (!card) return;
        event.preventDefault();
        onDrop(card);
      }}
    >
      {label} <span className={panel.tabCount}>{count}</span>
    </button>
  );
}

/** The scrolling deck view, which takes binder cards dropped anywhere on it. */
function DeckDropArea({ zone, onDrop, children }: { zone: DeckZone; onDrop(card: DraggedCard): void; children: ReactNode }) {
  const [over, setOver] = useState(false);
  const accepts = (event: DragEvent) => carriesCard(event) && event.dataTransfer.effectAllowed !== 'move';
  return (
    <div
      className={panel.scroll}
      role="tabpanel"
      aria-label={`${zoneLabel(zone, false)} cards`}
      data-over={over || undefined}
      onDragOver={(event) => {
        if (!accepts(event)) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = 'copy';
        setOver(true);
      }}
      onDragLeave={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setOver(false);
      }}
      onDrop={(event) => {
        setOver(false);
        if (event.defaultPrevented) return;
        const card = readDraggedCard(event);
        if (!card) return;
        event.preventDefault();
        onDrop(card);
      }}
    >
      {children}
    </div>
  );
}

const VIEWS: { kind: DeckViewKind; label: string; icon: ReactNode }[] = [
  { kind: 'list', label: 'List', icon: <List size={16} aria-hidden="true" /> },
  { kind: 'stacks', label: 'Stacks', icon: <Layers size={16} aria-hidden="true" /> },
  { kind: 'grid', label: 'Grid', icon: <LayoutGrid size={16} aria-hidden="true" /> },
];

/** How the deck is drawn: the view, its grouping and sorting, the card size, the layout, and undo. */
function ViewControls({ groupBy, groupOptions, layout, onLayout, history, add }: {
  groupBy: GroupBy;
  /** groupings this deck can't offer */
  groupOptions: GroupBy[] | null;
  layout: BuilderLayout;
  onLayout: ((layout: BuilderLayout) => void) | null;
  history: DeckPanelProps['history'];
  /** the deck-first layout's way to the binder */
  add: ReactNode;
}) {
  const view = useBuilderPrefs((state) => state.view);
  const sortBy = useBuilderPrefs((state) => state.sortBy);
  const cardSize = useBuilderPrefs((state) => state.cardSize);
  const set = useBuilderPrefs((state) => state.set);
  const reset = useBuilderPrefs((state) => state.reset);
  return (
    <div className={panel.controls}>
      <div className={panel.segmented} role="group" aria-label="Deck view">
        {VIEWS.map((each) => (
          <button key={each.kind} type="button" className={panel.iconButton} aria-pressed={view === each.kind} aria-label={`${each.label} view`} title={each.label} onClick={() => set({ view: each.kind })}>
            {each.icon}
          </button>
        ))}
      </div>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger asChild>
          <button type="button" className={panel.groupButton} aria-label={`Group by ${GROUP_BY_LABELS[groupBy].toLowerCase()}, sort by ${SORT_BY_LABELS[sortBy].toLowerCase()}`}>
            {GROUP_BY_LABELS[groupBy]} <ChevronDown size={14} aria-hidden="true" />
          </button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content className={styles.menu} side="bottom" align="end" sideOffset={6} collisionPadding={12}>
            <DropdownMenu.Label className={styles.menuLabel}>Group by</DropdownMenu.Label>
            <DropdownMenu.RadioGroup value={groupBy} onValueChange={(value) => set({ groupBy: value as GroupBy })}>
              {(Object.keys(GROUP_BY_LABELS) as GroupBy[]).filter((key) => !groupOptions?.includes(key)).map((key) => (
                <DropdownMenu.RadioItem key={key} value={key} className={styles.menuItem}>
                  <span className={styles.menuTick} aria-hidden="true"><DropdownMenu.ItemIndicator><Check size={14} /></DropdownMenu.ItemIndicator></span>
                  {GROUP_BY_LABELS[key]}
                </DropdownMenu.RadioItem>
              ))}
            </DropdownMenu.RadioGroup>
            <DropdownMenu.Separator className={styles.menuSeparator} />
            <DropdownMenu.Label className={styles.menuLabel}>Sort by</DropdownMenu.Label>
            <DropdownMenu.RadioGroup value={sortBy} onValueChange={(value) => set({ sortBy: value as SortBy })}>
              {(Object.keys(SORT_BY_LABELS) as SortBy[]).map((key) => (
                <DropdownMenu.RadioItem key={key} value={key} className={styles.menuItem}>
                  <span className={styles.menuTick} aria-hidden="true"><DropdownMenu.ItemIndicator><Check size={14} /></DropdownMenu.ItemIndicator></span>
                  {SORT_BY_LABELS[key]}
                </DropdownMenu.RadioItem>
              ))}
            </DropdownMenu.RadioGroup>
            <DropdownMenu.Separator className={styles.menuSeparator} />
            <DropdownMenu.Item className={styles.menuItem} onSelect={reset}>
              <RotateCcw size={16} aria-hidden="true" /> Reset the layout
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
      {view !== 'list' && (
        <label className={panel.size} title="Card size">
          <span className="visually-hidden">Card size</span>
          <input type="range" min={CARD_SIZE_MIN} max={CARD_SIZE_MAX} step={2} value={cardSize} onChange={(event) => set({ cardSize: Number(event.target.value) })} />
        </label>
      )}
      <span className={panel.spacer} />
      {add}
      <button type="button" className={panel.iconButton} aria-label="Undo" title="Undo (Ctrl+Z)" disabled={!history.canUndo} onClick={history.undo}><Undo2 size={16} aria-hidden="true" /></button>
      <button type="button" className={panel.iconButton} aria-label="Redo" title="Redo (Ctrl+Shift+Z)" disabled={!history.canRedo} onClick={history.redo}><Redo2 size={16} aria-hidden="true" /></button>
      {onLayout && (
        <button
          type="button"
          className={panel.iconButton}
          aria-pressed={layout === 'deck'}
          aria-label={layout === 'deck' ? 'Show the binder beside the deck' : 'Give the deck the whole page'}
          title={layout === 'deck' ? 'Binder beside the deck' : 'Deck first: the binder slides over'}
          onClick={() => onLayout(layout === 'deck' ? 'split' : 'deck')}
        >
          {layout === 'deck' ? <Columns2 size={16} aria-hidden="true" /> : <PanelLeftClose size={16} aria-hidden="true" />}
        </button>
      )}
    </div>
  );
}

/** A part of the panel the player can fold away; the fold is remembered. */
function PanelSection({ id, title, summary, children }: { id: SectionKey; title: string; summary: string; children: ReactNode }) {
  const collapsed = useBuilderPrefs((state) => !!state.collapsed[id]);
  const toggle = useBuilderPrefs((state) => state.toggleSection);
  const bodyId = useId();
  return (
    <section className={panel.section} data-collapsed={collapsed || undefined}>
      <h3 className={panel.sectionHead}>
        <button type="button" aria-expanded={!collapsed} aria-controls={bodyId} onClick={() => toggle(id)}>
          <ChevronDown size={14} className={panel.chevron} aria-hidden="true" />
          <span className={panel.sectionTitle}>{title}</span>
          <span className={panel.sectionSummary}>{summary}</span>
        </button>
      </h3>
      <div id={bodyId} className={panel.sectionBody} hidden={collapsed}>{children}</div>
    </section>
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
  onChange: Change;
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

/** The commander (or commanders), with the colors the deck may use. */
function CommandZone({ deck, info, rules, identity, onChange, onPreview }: {
  deck: DeckCardLists;
  info: ReadonlyMap<string, CardView>;
  rules: FormatRules;
  identity: string[] | null;
  onChange: Change;
  onPreview(card: CardView | null, anchor?: DOMRect, kind?: PreviewKind): void;
}) {
  const zone = rules.commandZone!;
  const commanders = deck.sideboard;
  return (
    <div className={styles.command}>
      {commanders.length === 0 ? (
        <p className={styles.commandEmpty}>
          <Crown size={16} aria-hidden="true" /> No {zone.cardLabel.toLowerCase()} yet. Open a legendary creature&rsquo;s menu in your deck and make it your commander.
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
                  onPointerEnter={(event) => card && onPreview(card, event.currentTarget.getBoundingClientRect(), 'row')}
                  onPointerLeave={() => onPreview(null)}
                >
                  <div className={styles.commanderArt}>
                    <CardFace card={card ?? { name: entry.cardName }} size="small" />
                  </div>
                  <span className={styles.commanderName}>{entry.cardName}</span>
                  <button type="button" className={styles.commanderBack} aria-label={`Put ${entry.cardName} back in the deck`} title="Back to the deck" onClick={() => onChange((current) => moveCard(current, 'sideboard', key, entry.amount, 'cards'))}>
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
    </div>
  );
}
