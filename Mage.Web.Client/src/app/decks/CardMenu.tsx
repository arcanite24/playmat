import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { useRef } from 'react';
import { Check, ChevronRight, Crown, FolderPlus, FolderX, Images, MoreHorizontal, MoveRight, Trash2, ZoomIn } from 'lucide-react';
import type { DeckRow, DeckZone } from './deckModel';
import { zoneLabel } from './zones';
import styles from './DeckBuilder.module.css';

export interface CardMenuActions {
  /** copies to another zone; the command zone only offers to take a card that may command */
  onMove(row: DeckRow, from: DeckZone, to: DeckZone, amount: number): void;
  onPrinting(row: DeckRow, zone: DeckZone): void;
  onInspect(row: DeckRow): void;
  onCategory(row: DeckRow, category: string | null): void;
  onNewCategory(row: DeckRow): void;
  onRemoveAll(row: DeckRow, zone: DeckZone): void;
}

/**
 * Everything else a card in the deck can do, behind its "more" button: move it between the deck, sideboard (or command
 * zone) and maybeboard, file it in a category, change its printing, look at it up close, take it out.
 */
export function CardMenu({ row, zone, zones, commandZone, canCommand, categories, category, actions, className }: {
  row: DeckRow;
  zone: DeckZone;
  /** the zones this deck has (no maybeboard in a Career or League deck) */
  zones: readonly DeckZone[];
  /** a commander deck: the sideboard is the command zone */
  commandZone: boolean;
  /** the card may go to the command zone now (it can command, and there is room) */
  canCommand: boolean;
  /** the deck's categories; null where the deck can't keep them */
  categories: string[] | null;
  category: string | undefined;
  actions: CardMenuActions;
  className?: string;
}) {
  const name = row.entry.cardName;
  const amount = row.entry.amount;
  // focus goes back to the menu's button when it closes, unless a dialog it opened takes it
  const handedOff = useRef(false);
  const handOff = (run: () => void) => () => {
    handedOff.current = true;
    run();
  };
  const targets = zones.filter((to) => to !== zone && !(commandZone && to === 'sideboard' && !canCommand));
  return (
    <DropdownMenu.Root modal={false}>
      <DropdownMenu.Trigger asChild>
        <button type="button" className={className} aria-label={`More for ${name}`} title="More" onClick={(event) => event.stopPropagation()} onContextMenu={(event) => event.stopPropagation()}>
          <MoreHorizontal size={15} aria-hidden="true" />
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content className={styles.menu} side="bottom" align="end" sideOffset={4} collisionPadding={12} onCloseAutoFocus={(event) => {
          if (handedOff.current) event.preventDefault();
          handedOff.current = false;
        }}>
          <DropdownMenu.Label className={styles.menuLabel}>{amount} × {name}</DropdownMenu.Label>
          {targets.map((to) => (
            <DropdownMenu.Item key={to} className={styles.menuItem} onSelect={() => actions.onMove(row, zone, to, commandZone && to === 'sideboard' ? amount : 1)}>
              {commandZone && to === 'sideboard' ? <Crown size={16} aria-hidden="true" /> : <MoveRight size={16} aria-hidden="true" />}
              {commandZone && to === 'sideboard' ? 'Make it your commander' : `One to the ${zoneLabel(to, commandZone).toLowerCase()}`}
            </DropdownMenu.Item>
          ))}
          {amount > 1 && targets.filter((to) => !(commandZone && to === 'sideboard')).map((to) => (
            <DropdownMenu.Item key={`all-${to}`} className={styles.menuItem} onSelect={() => actions.onMove(row, zone, to, amount)}>
              <MoveRight size={16} aria-hidden="true" /> All {amount} to the {zoneLabel(to, commandZone).toLowerCase()}
            </DropdownMenu.Item>
          ))}
          <DropdownMenu.Separator className={styles.menuSeparator} />
          {categories && (
            <DropdownMenu.Sub>
              <DropdownMenu.SubTrigger className={styles.menuItem}>
                <FolderPlus size={16} aria-hidden="true" /> Category{category ? `: ${category}` : ''}
                <ChevronRight size={14} className={styles.menuChevron} aria-hidden="true" />
              </DropdownMenu.SubTrigger>
              <DropdownMenu.Portal>
                <DropdownMenu.SubContent className={styles.menu} sideOffset={4} collisionPadding={12}>
                  {categories.map((label) => (
                    <DropdownMenu.Item key={label} className={styles.menuItem} onSelect={() => actions.onCategory(row, label)}>
                      <span className={styles.menuTick} aria-hidden="true">{label === category && <Check size={14} />}</span>
                      {label}
                    </DropdownMenu.Item>
                  ))}
                  {categories.length > 0 && <DropdownMenu.Separator className={styles.menuSeparator} />}
                  <DropdownMenu.Item className={styles.menuItem} onSelect={handOff(() => actions.onNewCategory(row))}>
                    <FolderPlus size={16} aria-hidden="true" /> New category…
                  </DropdownMenu.Item>
                  {category && (
                    <DropdownMenu.Item className={styles.menuItem} onSelect={() => actions.onCategory(row, null)}>
                      <FolderX size={16} aria-hidden="true" /> Out of {category}
                    </DropdownMenu.Item>
                  )}
                </DropdownMenu.SubContent>
              </DropdownMenu.Portal>
            </DropdownMenu.Sub>
          )}
          <DropdownMenu.Item className={styles.menuItem} onSelect={handOff(() => actions.onPrinting(row, zone))}>
            <Images size={16} aria-hidden="true" /> Choose printing…
          </DropdownMenu.Item>
          {row.card && (
            <DropdownMenu.Item className={styles.menuItem} onSelect={handOff(() => actions.onInspect(row))}>
              <ZoomIn size={16} aria-hidden="true" /> Look at it up close
            </DropdownMenu.Item>
          )}
          <DropdownMenu.Separator className={styles.menuSeparator} />
          <DropdownMenu.Item className={[styles.menuItem, styles.menuDanger].join(' ')} onSelect={() => actions.onRemoveAll(row, zone)}>
            <Trash2 size={16} aria-hidden="true" /> Take {amount > 1 ? `all ${amount}` : 'it'} out
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
