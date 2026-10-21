import * as RadixDialog from '@radix-ui/react-dialog';
import { Check, Info, X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { isStackAbility } from '../../core/game/cards';
import { keywordMarks } from '../../core/game/keywords';
import { explainCard, LINE_KIND_HELP, LINE_KIND_LABEL, type LineKind, type Term } from '../../core/game/glossary';
import { stripMarkup } from '../../core/game/prompt';
import type { CardView, GameView, PermanentView } from '../../protocol/generated/views';
import { AbilityCard } from '../ui/AbilityCard';
import { showsBackFace } from '../../core/images/imageLinks';
import { CardFace } from '../ui/CardFace';
import { ManaCost } from '../ui/ManaCost';
import { PromptText } from '../ui/PromptText';
import { useMatchUi } from './matchUi';
import { useStage } from './stageContext';
import { useLongPress } from './useLongPress';
import styles from './CardDetail.module.css';

/** Every card the table shows, by id: battlefield, hands, stack, graveyards, exile and revealed cards. */
function findCard(view: GameView | null, id: string, extra: readonly CardView[]): CardView | null {
  if (!view) return extra.find((card) => card.id === id) ?? null;
  const pools: ({ [key: string]: CardView } | undefined)[] = [view.myHand, view.stack];
  for (const player of view.players ?? []) pools.push(player.battlefield, player.graveyard, player.exile);
  for (const zone of view.exiles ?? []) pools.push(zone);
  for (const zone of [...(view.revealed ?? []), ...(view.lookedAt ?? [])]) pools.push(zone.cards);
  for (const pool of pools) {
    const card = pool?.[id];
    if (card) return card;
  }
  return extra.find((card) => card.id === id) ?? null;
}

function typeLine(card: CardView): string {
  const words = (list: readonly string[] | undefined) => (list ?? []).map((word) => word.charAt(0) + word.slice(1).toLowerCase().replace(/_/g, ' '));
  const left = [...words(card.superTypes), ...words(card.cardTypes)].join(' ');
  const right = words(card.subTypes as string[] | undefined).join(' ');
  return right ? `${left} — ${right}` : left;
}

/** What the picture can't show: state on the table, changed numbers, who has it, what is attached. */
function stateOf(card: PermanentView, view: GameView | null, attacking: ReadonlySet<string>, blocking: ReadonlySet<string>) {
  const chips: { label: string; tone?: 'damage' | 'good' }[] = [];
  const id = card.id ?? '';
  if (attacking.has(id)) chips.push({ label: 'Attacking', tone: 'damage' });
  if (blocking.has(id)) chips.push({ label: 'Blocking' });
  if (card.tapped) chips.push({ label: 'Tapped' });
  const creature = (card.cardTypes ?? []).includes('CREATURE');
  if (card.summoningSickness && creature && card.zone === 'BATTLEFIELD') chips.push({ label: 'Summoning sick' });
  if ((card.damage ?? 0) > 0) chips.push({ label: `${card.damage} damage`, tone: 'damage' });
  for (const counter of card.counters ?? []) {
    if ((counter.count ?? 0) > 0) chips.push({ label: `${counter.count} × ${counter.name}` });
  }
  for (const mark of keywordMarks(card)) {
    if (mark.gained) chips.push({ label: `Gained ${mark.name.toLowerCase()}`, tone: 'good' });
  }
  if (card.isToken) chips.push({ label: 'Token' });
  if (card.copy || card.originalIsCopy) chips.push({ label: 'Copy' });
  if (card.faceDown) chips.push({ label: 'Face down' });
  if (card.phasedIn === false) chips.push({ label: 'Phased out' });

  const names = (ids: readonly string[] | undefined) => (ids ?? []).map((other) => findCard(view, other, [])?.name).filter(Boolean) as string[];
  const notes: string[] = [];
  if (card.nameController && card.nameOwner && card.nameController !== card.nameOwner) {
    notes.push(`Controlled by ${card.nameController}, owned by ${card.nameOwner}.`);
  }
  if (card.attachedTo) {
    const target = findCard(view, card.attachedTo, [])?.name
      ?? view?.players?.find((player) => player.playerId === card.attachedTo)?.name;
    if (target) notes.push(`Attached to ${target}.`);
  }
  const attachments = names(card.attachments);
  if (attachments.length > 0) notes.push(`With ${attachments.join(', ')} attached.`);

  let stats: { label: string; value: string; base?: string } | null = null;
  const printed = (value: string | undefined, original: CardView['originalPower']) => value
    ?? original?.cardValue ?? (original?.baseValue !== undefined ? String(original.baseValue) : undefined);
  if (creature && card.power !== undefined && card.toughness !== undefined) {
    const value = `${card.power}/${card.toughness}`;
    const power = printed(card.original?.power, card.originalPower);
    const toughness = printed(card.original?.toughness, card.originalToughness);
    const base = power !== undefined && toughness !== undefined ? `${power}/${toughness}` : undefined;
    stats = { label: 'Power / Toughness', value, base: base && base !== value ? base : undefined };
  } else if ((card.cardTypes ?? []).includes('PLANESWALKER') && card.loyalty) {
    stats = { label: 'Loyalty', value: card.loyalty, base: card.startingLoyalty && card.startingLoyalty !== card.loyalty ? card.startingLoyalty : undefined };
  } else if ((card.cardTypes ?? []).includes('BATTLE') && card.defense) {
    stats = { label: 'Defense', value: card.defense, base: card.startingDefense && card.startingDefense !== card.defense ? card.startingDefense : undefined };
  }
  return { chips, notes, stats };
}

/**
 * The detail view (right click, a long press on a touch screen, or the context-menu key on a focused card): the card at full size beside what it is
 * doing on the table and what its text means, the way Arena explains a card. Opens from any card on the stage.
 */
export function CardDetail({ view, extra, sleeveOf, attacking, blocking }: {
  view: GameView | null;
  /** cards shown by an open overlay that aren't on the table (a choice among cards) */
  extra: readonly CardView[];
  sleeveOf(card: CardView): string;
  attacking: ReadonlySet<string>;
  blocking: ReadonlySet<string>;
}) {
  const stage = useStage();
  const detail = useMatchUi((state) => state.detail);
  const openDetail = useMatchUi((state) => state.openDetail);
  const [backFace, setBackFace] = useState(false);
  const sheet = useRef<HTMLDivElement>(null);

  // the card under a pointer, when it can be opened: anything on the stage that carries its id
  const cardAt = useCallback((element: EventTarget | null) => {
    const target = element instanceof Element ? element.closest<HTMLElement>('[data-object-id], [data-card-id]') : null;
    const id = target?.dataset.objectId ?? target?.dataset.cardId;
    const card = id ? findCard(view, id, extra) : null;
    return card && id && !card.hideInfo ? { id, card } : null;
  }, [view, extra]);

  // one listener for the whole stage: any card that carries its id can be opened
  useEffect(() => {
    const element = stage.element;
    if (!element) return;
    const onContextMenu = (event: MouseEvent) => {
      event.preventDefault();
      // a finger held on a card opened it already (Android also calls that a context menu)
      if ((event as PointerEvent).pointerType === 'touch' || (event as PointerEvent).pointerType === 'pen') return;
      const found = cardAt(event.target);
      if (!found) return;
      setBackFace(false);
      openDetail(found);
    };
    element.addEventListener('contextmenu', onContextMenu);
    return () => element.removeEventListener('contextmenu', onContextMenu);
  }, [stage.element, cardAt, openDetail]);

  // touch has no hover and no right button: a finger held on a card opens it (one dragged from hand, or onto an attacker, isn't)
  useLongPress(stage.element, {
    onLongPress: (target) => {
      const found = cardAt(target);
      if (!found || useMatchUi.getState().detail) return false;
      setBackFace(false);
      openDetail(found);
      return true;
    },
    isBusy: () => {
      const ui = useMatchUi.getState();
      return !!ui.dragging || !!ui.blockDrag;
    },
  });

  // the latest state of the card while it stays on the table; the snapshot once it has left
  const live = detail ? findCard(view, detail.id, extra) ?? detail.card : null;
  const card = live && backFace && live.secondCardFace ? live.secondCardFace : live;
  const explanation = useMemo(() => (card ? explainCard(isStackAbility(card) ? { ...card, cardTypes: [] } : card) : null), [card]);
  if (!detail || !live || !card || !explanation) return null;

  // closing waits a tick: the key that closed it must not also reach the board's own shortcuts
  const close = () => setTimeout(() => openDetail(null), 0);
  const ability = isStackAbility(live);
  const permanent = live as PermanentView;
  const { chips, notes, stats } = stateOf(permanent, view, attacking, blocking);
  const kinds = [...new Set(explanation.lines.map((line) => line.kind))]
    .filter((kind): kind is Exclude<LineKind, 'keyword' | 'spell' | 'static'> => !['keyword', 'spell', 'static'].includes(kind));
  const terms: Term[] = [
    ...explanation.terms,
    ...kinds.map((kind) => ({ name: `${LINE_KIND_LABEL[kind]} ability`, text: LINE_KIND_HELP[kind] })),
  ];
  const name = ability ? (live as CardView & { sourceCard?: CardView }).sourceCard?.name ?? 'Ability' : card.displayName ?? card.name ?? '';
  const sleeve = sleeveOf(live);
  const canTurn = !!live.secondCardFace && !ability;

  return (
    <RadixDialog.Root open onOpenChange={(open) => !open && close()}>
      <RadixDialog.Portal container={stage.element}>
        <RadixDialog.Overlay
          className={styles.scrim}
          onContextMenu={(event) => {
            event.preventDefault();
            // the finger that opened it is still down: Android follows the long press with a context menu
            if ((event.nativeEvent as PointerEvent).pointerType !== 'touch') close();
          }}
        />
        <RadixDialog.Content
          ref={sheet}
          className={styles.sheet}
          aria-describedby={undefined}
          // the sheet itself takes focus: a ring on the close button would be the first thing seen
          onOpenAutoFocus={(event) => {
            event.preventDefault();
            sheet.current?.focus();
          }}
          onContextMenu={(event) => event.preventDefault()}
        >
          <div className={styles.cardColumn}>
            <div className={styles.card}>
              {ability
                ? <AbilityCard ability={live} sleeve={sleeve} />
                : <CardFace card={card} face={backFace && !showsBackFace(live) ? 'back' : 'front'} size="large" sleeve={sleeve} />}
            </div>
            {canTurn && (
              <button type="button" className={styles.turn} onClick={() => setBackFace((value) => !value)}>
                {backFace ? 'Show the front face' : 'Show the back face'}
              </button>
            )}
          </div>

          <div className={styles.info}>
            <header className={styles.head}>
              <div className={styles.titleRow}>
                <RadixDialog.Title className={styles.name}>{name}</RadixDialog.Title>
                {!ability && <ManaCost cost={card.manaCostLeftStr} size="lg" />}
              </div>
              <p className={styles.type}>{ability ? (card.abilityType?.startsWith('TRIGGERED') ? 'Triggered ability' : 'Activated ability') : typeLine(card)}</p>
            </header>

            {(stats || chips.length > 0 || notes.length > 0 || explanation.hints.length > 0) && (
              <section className={styles.state} aria-label="On the table">
                {stats && (
                  <p className={styles.stats}>
                    <span className={styles.statsLabel}>{stats.label}</span>
                    <span className={styles.statsValue}>{stats.value}</span>
                    {stats.base && <span className={styles.statsBase}>printed {stats.base}</span>}
                  </p>
                )}
                {chips.length > 0 && (
                  <ul className={styles.chips}>
                    {chips.map((chip) => <li key={chip.label} className={chip.tone === 'damage' ? styles.chipDamage : styles.chip}>{chip.label}</li>)}
                  </ul>
                )}
                {notes.map((note) => <p key={note} className={styles.note}>{note}</p>)}
                {explanation.hints.length > 0 && (
                  <ul className={styles.hints}>
                    {explanation.hints.map((hint) => (
                      <li key={hint.text} className={styles[hint.tone]}>
                        {hint.tone === 'good' ? <Check size={20} aria-label="True now" /> : hint.tone === 'bad' ? <X size={20} aria-label="Not true now" /> : <Info size={20} aria-hidden="true" />}
                        <span><PromptText text={hint.text} /></span>
                      </li>
                    ))}
                  </ul>
                )}
              </section>
            )}

            {explanation.lines.length > 0 && (
              <section className={styles.section}>
                <h3 className={styles.sectionTitle}>Card text</h3>
                <ol className={styles.lines}>
                  {explanation.lines.map((line, index) => (
                    <li key={index} className={styles.line}>
                      <span className={styles.kind}>{LINE_KIND_LABEL[line.kind]}</span>
                      <span className={styles.lineText}><PromptText text={stripMarkup(line.text)} /></span>
                    </li>
                  ))}
                </ol>
              </section>
            )}

            {terms.length > 0 && (
              <section className={styles.section}>
                <h3 className={styles.sectionTitle}>Mechanics</h3>
                <dl className={styles.terms}>
                  {terms.map((term) => (
                    <div key={term.name} className={styles.term}>
                      <dt>{term.name}</dt>
                      <dd><PromptText text={term.text} /></dd>
                    </div>
                  ))}
                </dl>
              </section>
            )}
          </div>

          <RadixDialog.Close className={styles.close} aria-label="Close">
            <X size={26} />
          </RadixDialog.Close>
        </RadixDialog.Content>
      </RadixDialog.Portal>
    </RadixDialog.Root>
  );
}
