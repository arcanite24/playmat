import { ZoomIn } from 'lucide-react';
import type { CardView } from '../../protocol/generated/views';
import { stripMarkup } from '../../core/game/prompt';
import { CardFace } from '../ui/CardFace';
import { Dialog } from '../ui/Dialog';
import { PromptText } from '../ui/PromptText';
import styles from './DeckBuilder.module.css';

function typeLine(card: CardView): string {
  const words = (list: readonly string[] | undefined) => (list ?? []).map((word) => word.charAt(0) + word.slice(1).toLowerCase().replace(/_/g, ' '));
  const left = [...words(card.superTypes), ...words(card.cardTypes)].join(' ');
  const right = words(card.subTypes as string[] | undefined).join(' ');
  return right ? `${left} — ${right}` : left;
}

/**
 * A card in the binder, up close (#250): the picture large, both faces of a double-faced card, and its rules as text,
 * for reading what the hover preview is too quick or too small for.
 */
export function CardInspect({ card, onClose }: { card: CardView | null; onClose(): void }) {
  const back = card?.transformable ? card.secondCardFace : undefined;
  const faces = card ? [card, ...(back ? [back] : [])] : [];
  return (
    <Dialog open={!!card} onOpenChange={(open) => { if (!open) onClose(); }} title={card?.name ?? ''} width="lg">
      {card && (
        <div className={styles.inspect}>
          <div className={styles.inspectFaces}>
            <CardFace card={card} size="large" className={styles.inspectFace} />
            {back && <CardFace card={back} face="back" size="large" className={styles.inspectFace} />}
          </div>
          <div className={styles.inspectText}>
            {faces.map((face, index) => {
              const rules = (face.rules ?? []).map(stripMarkup).filter(Boolean);
              const type = typeLine(face);
              return (
                <section key={index}>
                  {faces.length > 1 && <h3>{face.name}</h3>}
                  {type && <p className={styles.inspectType}>{type}</p>}
                  {rules.length > 0 && <ul>{rules.map((line, at) => <li key={at}><PromptText text={line} /></li>)}</ul>}
                  {face.power && face.toughness && <p className={styles.inspectType}>{face.power}/{face.toughness}</p>}
                </section>
              );
            })}
          </div>
        </div>
      )}
    </Dialog>
  );
}

/** The magnifier on a binder card that opens it up close; the card itself still adds (click) and removes (right-click). */
export function InspectButton({ card, label, onInspect }: { card: CardView; label: string; onInspect(card: CardView): void }) {
  return (
    <button type="button" className={styles.inspectButton} aria-label={label} title={label} onClick={() => onInspect(card)}>
      <ZoomIn size={16} aria-hidden="true" />
    </button>
  );
}
