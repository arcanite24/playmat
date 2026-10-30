import { BookOpen, Crown, Skull, Sparkles, Ban } from 'lucide-react';
import { useRef, type ReactNode } from 'react';
import type { CommanderStatus } from '../../core/game/commander';
import type { CardView, PlayerView } from '../../protocol/generated/views';
import { useT } from '../i18n';
import './matchMessages';
import { CardFace } from '../ui/CardFace';
import { cardKey } from './boardModel';
import { shuffledLibrary } from '../../core/game/libraryList';
import { useFlip, useFlipOrigin } from './flip';
import { useMatchUi } from './matchUi';
import styles from './Piles.module.css';

export interface CommandProps {
  /** the player's commanders, wherever they are (see commandersByPlayer) */
  commanders?: readonly CommanderStatus[];
  /** ids that can be clicked now: a commander in the command zone that can be cast */
  clickable?: ReadonlySet<string>;
  onCast?(id: string): void;
}

/** Library, graveyard, exile and command zone of one player, as piles on the mat. */
export function Piles({ player, sleeve, isMe, commanders = [], clickable, onCast }: { player: PlayerView; sleeve: string; isMe: boolean } & CommandProps) {
  const t = useT();
  const openViewer = useMatchUi((state) => state.openViewer);
  const libraryRef = useRef<HTMLButtonElement>(null);
  useFlipOrigin(`library:${player.playerId}`, libraryRef);
  const graveyard = Object.values(player.graveyard ?? {});
  const exile = Object.values(player.exile ?? {});
  const { waiting, others } = splitCommand(player, commanders);
  const name = isMe ? 'Your' : `${player.name}'s`;
  // your own library's cards (names and counts only, sent to you alone), shuffled on every opening
  const openLibrary = () => {
    const entries = shuffledLibrary(player.libraryCards);
    const cards = entries.map((entry, i): CardView => ({ id: `library-list-${i}`, name: entry.name, displayName: entry.name }));
    const counts = Object.fromEntries(entries.map((entry, i) => [`library-list-${i}`, entry.count]));
    const note = player.topCard?.name ? t('match.library.orderTop', { name: player.topCard.name }) : t('match.library.order');
    openViewer({ title: `${name} library (${player.libraryCount ?? 0} cards)`, cards, counts, note });
  };

  return (
    <div className={[styles.piles, isMe ? styles.mine : styles.theirs].join(' ')}>
      <button
        ref={libraryRef}
        type="button"
        className={styles.pile}
        aria-label={t('match.cards', { label: `${name} library`, count: player.libraryCount ?? 0 })}
        onClick={() => (isMe && player.libraryCards ? openLibrary() : player.topCard && openViewer({ title: `${name} library (top card)`, cards: [player.topCard] }))}
      >
        <div className={styles.stack} style={{ ['--depth' as string]: Math.min(6, Math.ceil((player.libraryCount ?? 0) / 10)) }}>
          <CardFace card={player.topCard ?? {}} hidden={!player.topCard} sleeve={sleeve} size="small" />
        </div>
        <span className={styles.count}>{player.libraryCount ?? 0}</span>
        <span className={styles.label}>Library</span>
      </button>

      <PileOfCards title={`${name} graveyard`} label="Graveyard" cards={graveyard} sleeve={sleeve} onOpen={() => openViewer({ title: `${name} graveyard`, cards: [...graveyard].reverse() })} />
      {exile.length > 0 && (
        <PileOfCards title={`${name} exile`} label="Exile" cards={exile} sleeve={sleeve} onOpen={() => openViewer({ title: `${name} exile`, cards: exile })} />
      )}
      {waiting.map((commander) => (
        <CommanderPile key={commander.id} commander={commander} sleeve={sleeve} castable={!!clickable?.has(commander.id)} onCast={onCast} owner={name} />
      ))}
      {others.length > 0 && (
        <PileOfCards title={`${name} command zone`} label="Command" cards={others} sleeve={sleeve} onOpen={() => openViewer({ title: `${name} command zone`, cards: others })} />
      )}
    </div>
  );
}

/** Commanders waiting in the command zone, and the zone's other objects (emblems, a dungeon, planes). */
function splitCommand(player: PlayerView, commanders: readonly CommanderStatus[]) {
  const waiting = commanders.filter((commander) => commander.zone === 'command');
  const ids = new Set(waiting.map((commander) => commander.id));
  const others = ((player.commandList ?? []) as CardView[]).filter((card) => !ids.has(card.id!) && card.mageObjectType !== 'COMMANDER');
  return { waiting, others };
}

/** A commander in the command zone, face up: it glows when it can be cast, and says what casting it costs extra. */
function CommanderPile({ commander, sleeve, castable, onCast, owner }: {
  commander: CommanderStatus;
  sleeve: string;
  castable: boolean;
  onCast?(id: string): void;
  owner: string;
}) {
  const setZoom = useMatchUi((state) => state.setZoom);
  const openViewer = useMatchUi((state) => state.openViewer);
  const label = `${owner} commander, ${commander.name}${commander.tax > 0 ? `, costs ${commander.tax} more` : ''}${castable ? '. Cast it' : ''}`;
  return (
    <button
      type="button"
      className={[styles.pile, castable ? styles.castable : ''].join(' ')}
      aria-label={label}
      data-object-id={commander.id}
      onClick={() => (castable && onCast ? onCast(commander.id) : openViewer({ title: `${owner} command zone`, cards: [commander.card] }))}
      onPointerEnter={(event) => setZoom({ card: commander.card, x: event.clientX, y: event.clientY, sleeve })}
      onPointerLeave={() => setZoom(null)}
    >
      <div className={styles.stack}>
        <CardFace card={commander.card} sleeve={sleeve} size="small" />
      </div>
      {commander.tax > 0 && <span className={styles.tax} title={`Cast ${commander.casts} ${commander.casts === 1 ? 'time' : 'times'} from the command zone`}>+{commander.tax}</span>}
      <span className={styles.label}><Crown size={16} aria-hidden="true" /> Commander</span>
    </button>
  );
}

function PileOfCards({ title, label, cards, sleeve, onOpen }: { title: string; label: string; cards: CardView[]; sleeve: string; onOpen(): void }) {
  const t = useT();
  const top = cards[cards.length - 1];
  return (
    <button type="button" className={styles.pile} aria-label={t('match.cards', { label: title, count: cards.length })} onClick={onOpen} disabled={cards.length === 0}>
      {/* a new top card is a new element: it lands on the pile from wherever that card was */}
      <PileTop key={top ? cardKey(top) : 'empty'} top={top} count={cards.length} sleeve={sleeve} />
      <span className={styles.count}>{cards.length}</span>
      <span className={styles.label}>{label}</span>
    </button>
  );
}

function PileTop({ top, count, sleeve }: { top: CardView | undefined; count: number; sleeve: string }) {
  const ref = useRef<HTMLDivElement>(null);
  useFlip(top ? cardKey(top) : undefined, ref);
  return (
    <div ref={ref} className={[styles.stack, count === 0 ? styles.empty : ''].join(' ')} style={{ ['--depth' as string]: Math.min(4, count) }}>
      {top && <CardFace card={top} sleeve={sleeve} size="small" />}
    </div>
  );
}

/**
 * An opponent's zones at a multiplayer table, where full piles don't fit: one row of small counters under their seat.
 * Their commander waiting in the command zone shows by name with its tax.
 */
export function MiniPiles({ player, commanders = [] }: { player: PlayerView } & Pick<CommandProps, 'commanders'>) {
  const t = useT();
  const openViewer = useMatchUi((state) => state.openViewer);
  const graveyard = Object.values(player.graveyard ?? {});
  const exile = Object.values(player.exile ?? {});
  const { waiting, others } = splitCommand(player, commanders);
  const name = `${player.name}'s`;
  const libraryRef = useRef<HTMLSpanElement>(null);
  useFlipOrigin(`library:${player.playerId}`, libraryRef);
  return (
    <div className={styles.mini}>
      <span ref={libraryRef} className={styles.chip} aria-label={t('match.cards', { label: `${name} library`, count: player.libraryCount ?? 0 })} title="Library">
        <BookOpen size={16} aria-hidden="true" /> {player.libraryCount ?? 0}
      </span>
      <MiniButton icon={<Skull size={16} aria-hidden="true" />} count={graveyard.length} label={`${name} graveyard`} onOpen={() => openViewer({ title: `${name} graveyard`, cards: [...graveyard].reverse() })} />
      {exile.length > 0 && (
        <MiniButton icon={<Ban size={16} aria-hidden="true" />} count={exile.length} label={`${name} exile`} onOpen={() => openViewer({ title: `${name} exile`, cards: exile })} />
      )}
      {others.length > 0 && (
        <MiniButton icon={<Sparkles size={16} aria-hidden="true" />} count={others.length} label={`${name} command zone`} onOpen={() => openViewer({ title: `${name} command zone`, cards: others })} />
      )}
      {waiting.map((commander) => (
        <button
          key={commander.id}
          type="button"
          className={[styles.chip, styles.chipCommander].join(' ')}
          aria-label={`${name} commander ${commander.name} is in the command zone${commander.tax > 0 ? `, costs ${commander.tax} more` : ''}`}
          onClick={() => openViewer({ title: `${name} command zone`, cards: [commander.card] })}
        >
          <Crown size={16} aria-hidden="true" />
          <span className={styles.chipName}>{commander.name.split(',')[0]}</span>
          {commander.tax > 0 && <b>+{commander.tax}</b>}
        </button>
      ))}
    </div>
  );
}

function MiniButton({ icon, count, label, onOpen }: { icon: ReactNode; count: number; label: string; onOpen(): void }) {
  const t = useT();
  return (
    <button type="button" className={styles.chip} aria-label={t('match.cards', { label, count })} title={label} onClick={onOpen} disabled={count === 0}>
      {icon} {count}
    </button>
  );
}
