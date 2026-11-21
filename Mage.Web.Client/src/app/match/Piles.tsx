import { BookOpen, Castle, Crown, Skull, Sparkles, Ban } from 'lucide-react';
import { useRef, type ReactNode } from 'react';
import type { CommanderStatus } from '../../core/game/commander';
import { playerDungeon, type Dungeon } from '../../core/game/dungeon';
import type { CardView, PlayerView } from '../../protocol/generated/views';
import { useT, type MessageKey, type Translate } from '../i18n';
import './matchMessages';
import { CardFace } from '../ui/CardFace';
import { cardKey } from './boardModel';
import { useFlip, useFlipOrigin } from './flip';
import { useMatchUi } from './matchUi';

type ZoneKind = 'library' | 'libraryTop' | 'graveyard' | 'exile' | 'command' | 'dungeon' | 'commander';

/** "Your graveyard" or "Bob's graveyard": owner null is the player */
function zoneLabel(t: Translate, kind: ZoneKind, owner: string | null): string {
  return owner === null ? t(`match.pile.${kind}.mine` as MessageKey) : t(`match.pile.${kind}.theirs` as MessageKey, { name: owner });
}
import styles from './Piles.module.css';

export interface CommandProps {
  /** the player's commanders, wherever they are (see commandersByPlayer) */
  commanders?: readonly CommanderStatus[];
  /** ids that can be clicked now: a commander in the command zone that can be cast */
  clickable?: ReadonlySet<string>;
  onCast?(id: string): void;
  /** cards the hand shows while they can be played from here (an adventurer in exile, flashback): see PileTop */
  inHand?: ReadonlySet<string>;
}

/** Library, graveyard, exile and command zone of one player, as piles on the mat. */
export function Piles({ player, sleeve, isMe, commanders = [], clickable, onCast, inHand }: { player: PlayerView; sleeve: string; isMe: boolean } & CommandProps) {
  const t = useT();
  const openViewer = useMatchUi((state) => state.openViewer);
  const libraryRef = useRef<HTMLButtonElement>(null);
  useFlipOrigin(`library:${player.playerId}`, libraryRef);
  const graveyard = Object.values(player.graveyard ?? {});
  const exile = Object.values(player.exile ?? {});
  const { waiting, dungeon, others } = splitCommand(player, commanders);
  const owner = isMe ? null : player.name ?? '';
  const zone = (kind: ZoneKind) => zoneLabel(t, kind, owner);
  const libraryZoom = usePileZoom(player.topCard, sleeve);

  return (
    <div className={[styles.piles, isMe ? styles.mine : styles.theirs].join(' ')}>
      <button
        ref={libraryRef}
        type="button"
        className={styles.pile}
        aria-label={t('match.cards', { label: zone('library'), count: player.libraryCount ?? 0 })}
        onClick={() => player.topCard && openViewer({ title: zone('libraryTop'), cards: [player.topCard] })}
        {...libraryZoom}
      >
        <div className={styles.stack} style={{ ['--depth' as string]: Math.min(6, Math.ceil((player.libraryCount ?? 0) / 10)) }}>
          <CardFace card={player.topCard ?? {}} hidden={!player.topCard} sleeve={sleeve} size="small" />
        </div>
        <span className={styles.count}>{player.libraryCount ?? 0}</span>
        <span className={styles.label}>{t('match.pile.library')}</span>
      </button>

      <PileOfCards title={zone('graveyard')} label={t('match.pile.graveyard')} cards={graveyard} sleeve={sleeve} inHand={inHand} onOpen={() => openViewer({ title: zone('graveyard'), cards: [...graveyard].reverse() })} />
      {exile.length > 0 && (
        <PileOfCards title={zone('exile')} label={t('match.pile.exile')} cards={exile} sleeve={sleeve} inHand={inHand} onOpen={() => openViewer({ title: zone('exile'), cards: exile })} />
      )}
      {waiting.map((commander) => (
        <CommanderPile key={commander.id} commander={commander} sleeve={sleeve} castable={!!clickable?.has(commander.id)} onCast={onCast} owner={owner} />
      ))}
      {dungeon && <DungeonPile dungeon={dungeon} owner={owner} playerId={player.playerId!} />}
      {others.length > 0 && (
        <PileOfCards title={zone('command')} label={t('match.pile.command')} cards={others} sleeve={sleeve} onOpen={() => openViewer({ title: zone('command'), cards: others })} />
      )}
    </div>
  );
}

/** Commanders waiting in the command zone, the dungeon, and the zone's other objects (emblems, planes). */
function splitCommand(player: PlayerView, commanders: readonly CommanderStatus[]) {
  const waiting = commanders.filter((commander) => commander.zone === 'command');
  const ids = new Set(waiting.map((commander) => commander.id));
  const dungeon = playerDungeon(player);
  const others = ((player.commandList ?? []) as CardView[])
    .filter((card) => !ids.has(card.id!) && card.mageObjectType !== 'COMMANDER' && (!dungeon || card.id !== dungeon.id));
  return { waiting, dungeon, others };
}

/** The dungeon a player ventures into, face up with the room its marker is in; it opens room by room (#385). */
function DungeonPile({ dungeon, owner, playerId }: { dungeon: Dungeon; owner: string | null; playerId: string }) {
  const t = useT();
  const openDungeon = useMatchUi((state) => state.openDungeon);
  const zoom = usePileZoom(dungeon.card, '');
  const room = dungeon.currentRoom ?? t('match.pile.notEntered');
  return (
    <button type="button" className={styles.pile} aria-label={t('match.pile.dungeonAria', { zone: zoneLabel(t, 'dungeon', owner), name: dungeon.name, room })} onClick={() => openDungeon(playerId)} {...zoom}>
      <div className={styles.stack}>
        <CardFace card={dungeon.card} size="small" />
      </div>
      <span className={styles.room} title={room}>{room}</span>
      <span className={styles.label}><Castle size={16} aria-hidden="true" /> {t('match.pile.dungeon')}</span>
    </button>
  );
}

/** A commander in the command zone, face up: it glows when it can be cast, and says what casting it costs extra. */
function CommanderPile({ commander, sleeve, castable, onCast, owner }: {
  commander: CommanderStatus;
  sleeve: string;
  castable: boolean;
  onCast?(id: string): void;
  owner: string | null;
}) {
  const t = useT();
  const setZoom = useMatchUi((state) => state.setZoom);
  const openViewer = useMatchUi((state) => state.openViewer);
  const label = t('match.pile.commanderAria', { zone: zoneLabel(t, 'commander', owner), name: commander.name })
    + (commander.tax > 0 ? `, ${t('match.pile.tax', { count: commander.tax })}` : '')
    + (castable ? `. ${t('match.pile.castIt')}` : '');
  return (
    <button
      type="button"
      className={[styles.pile, castable ? styles.castable : ''].join(' ')}
      aria-label={label}
      data-object-id={commander.id}
      onClick={() => (castable && onCast ? onCast(commander.id) : openViewer({ title: zoneLabel(t, 'command', owner), cards: [commander.card] }))}
      onPointerEnter={(event) => setZoom({ card: commander.card, x: event.clientX, y: event.clientY, sleeve })}
      onPointerLeave={() => setZoom(null)}
    >
      <div className={styles.stack}>
        <CardFace card={commander.card} sleeve={sleeve} size="small" />
      </div>
      {commander.tax > 0 && <span className={styles.tax} title={t('match.pile.casts', { count: commander.casts })}>+{commander.tax}</span>}
      <span className={styles.label}><Crown size={16} aria-hidden="true" /> {t('match.pile.commander')}</span>
    </button>
  );
}

/** Hovering a pile opens its top card large, as on the table (#165); a finger opens the pile's viewer instead. */
function usePileZoom(card: CardView | undefined, sleeve: string) {
  const setZoom = useMatchUi((state) => state.setZoom);
  return {
    onPointerEnter: (event: React.PointerEvent<HTMLElement>) => {
      if (card && event.pointerType !== 'touch') setZoom({ card, x: event.clientX, y: event.clientY, sleeve, anchor: event.currentTarget });
    },
    onPointerLeave: () => setZoom(null),
  };
}

function PileOfCards({ title, label, cards, sleeve, onOpen, inHand }: { title: string; label: string; cards: CardView[]; sleeve: string; onOpen(): void; inHand?: ReadonlySet<string> }) {
  const t = useT();
  const top = cards[cards.length - 1];
  const zoom = usePileZoom(top, sleeve);
  return (
    <button type="button" className={styles.pile} aria-label={t('match.cards', { label: title, count: cards.length })} onClick={onOpen} disabled={cards.length === 0} {...zoom}>
      {/* a new top card is a new element: it lands on the pile from wherever that card was */}
      <PileTop key={top ? cardKey(top) : 'empty'} top={top} count={cards.length} sleeve={sleeve} inHand={!!top && !!inHand?.has(cardKey(top))} />
      <span className={styles.count}>{cards.length}</span>
      <span className={styles.label}>{label}</span>
    </button>
  );
}

function PileTop({ top, count, sleeve, inHand }: { top: CardView | undefined; count: number; sleeve: string; inHand: boolean }) {
  const ref = useRef<HTMLDivElement>(null);
  // a top card the hand also shows is the hand's to fly: two elements measured under one card would each fly from
  // the other's place on every render, and the card would bounce between the hand and the pile. It lands back here
  // from the hand once the hand lets it go.
  useFlip(top && !inHand ? cardKey(top) : undefined, ref);
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
  const openDungeon = useMatchUi((state) => state.openDungeon);
  const { waiting, dungeon, others } = splitCommand(player, commanders);
  const owner = player.name ?? '';
  const zone = (kind: ZoneKind) => zoneLabel(t, kind, owner);
  const libraryRef = useRef<HTMLSpanElement>(null);
  useFlipOrigin(`library:${player.playerId}`, libraryRef);
  return (
    <div className={styles.mini}>
      <span ref={libraryRef} className={styles.chip} aria-label={t('match.cards', { label: zone('library'), count: player.libraryCount ?? 0 })} title={t('match.pile.library')}>
        <BookOpen size={16} aria-hidden="true" /> {player.libraryCount ?? 0}
      </span>
      <MiniButton icon={<Skull size={16} aria-hidden="true" />} count={graveyard.length} label={zone('graveyard')} onOpen={() => openViewer({ title: zone('graveyard'), cards: [...graveyard].reverse() })} />
      {exile.length > 0 && (
        <MiniButton icon={<Ban size={16} aria-hidden="true" />} count={exile.length} label={zone('exile')} onOpen={() => openViewer({ title: zone('exile'), cards: exile })} />
      )}
      {dungeon && (
        <button
          type="button"
          className={styles.chip}
          aria-label={t('match.pile.dungeonAria', { zone: zone('dungeon'), name: dungeon.name, room: dungeon.currentRoom ?? t('match.pile.notEntered') })}
          title={`${dungeon.name}: ${dungeon.currentRoom ?? t('match.pile.notEntered')}`}
          onClick={() => openDungeon(player.playerId!)}
        >
          <Castle size={16} aria-hidden="true" />
          <span className={styles.chipName}>{dungeon.currentRoom ?? dungeon.name}</span>
        </button>
      )}
      {others.length > 0 && (
        <MiniButton icon={<Sparkles size={16} aria-hidden="true" />} count={others.length} label={zone('command')} onOpen={() => openViewer({ title: zone('command'), cards: others })} />
      )}
      {waiting.map((commander) => (
        <button
          key={commander.id}
          type="button"
          className={[styles.chip, styles.chipCommander].join(' ')}
          aria-label={t('match.pile.commanderIn', { zone: zoneLabel(t, 'commander', owner), name: commander.name })
            + (commander.tax > 0 ? `, ${t('match.pile.tax', { count: commander.tax })}` : '')}
          title={commander.name}
          onClick={() => openViewer({ title: zone('command'), cards: [commander.card] })}
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
