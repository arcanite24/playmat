import type { CSSProperties } from 'react';
import { useCardImage } from '../../core/images/useCardImage';
import { useT } from '../i18n';
import type { RosterDeck } from '../stores/decks';
import { useSettings } from '../stores/settings';
import { useCosmetics } from './cosmeticsCatalog';
import { sleeveBackOf } from './sleeveArt';
import styles from './DeckBox.module.css';

export interface DeckBoxProps {
  deck: RosterDeck;
  sleeve: string;
  selected?: boolean;
  onSelect?(): void;
  onActivate?(): void;
}

/** A deck in the roster: a sleeved deck box with its cover art window and a name plate. */
export function DeckBox({ deck, sleeve, selected, onSelect, onActivate }: DeckBoxProps) {
  const t = useT();
  const art = useCardImage(deck.cover, 'front', 'art_crop');
  // a Career or supporter sleeve shows its painted back on the box too, as it does on the cards (see cardBackOf)
  const paintedArt = useSettings((state) => state.settings.paintedArt);
  useCosmetics((state) => state.byKey);
  const back = sleeveBackOf(sleeve);
  const painted = back && (paintedArt || back.startsWith('data:')) ? back : null;
  return (
    <button
      type="button"
      className={[styles.box, painted ? styles.painted : '', selected ? styles.selected : ''].join(' ')}
      style={{ '--sleeve': sleeve, ...(painted ? { '--sleeve-art': `url("${painted}")` } : {}) } as CSSProperties}
      aria-pressed={selected}
      onClick={onSelect}
      onDoubleClick={onActivate}
    >
      <span className={styles.window} style={typeof art === 'string' ? { backgroundImage: `url("${art}")` } : undefined} aria-hidden="true">
        {/* no art (yet): the deck's initial, printed */}
        {typeof art !== 'string' && <span className={styles.initial}>{deck.name.slice(0, 1).toUpperCase()}</span>}
      </span>
      <span className={styles.plate}>
        <span className={styles.name}>{deck.name}</span>
        <span className={styles.note}>
          {deck.colors.length > 0 && (
            <span className={styles.pips} aria-label={t('ui.colors', { colors: deck.colors.join('') })}>
              {deck.colors.map((color) => <i key={color} className={`ms ms-cost ms-${color.toLowerCase()}`} aria-hidden="true" />)}
            </span>
          )}
          {deck.fromStarter ? t('decks.fromStarter', { count: deck.cardCount }) : deck.note}
        </span>
      </span>
    </button>
  );
}
