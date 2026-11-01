import { useEffect, useMemo, useState } from 'react';
import type { CardView, PermanentView } from '../../protocol/generated/views';
import { isStackAbility } from '../../core/game/cards';
import { explainCard } from '../../core/game/glossary';
import { AbilityCard } from '../ui/AbilityCard';
import { showsBackFace } from '../../core/images/imageLinks';
import { CardFace } from '../ui/CardFace';
import { useT } from '../i18n';
import { useCardStyle } from '../ui/cardStyles';
import { PromptText } from '../ui/PromptText';
import { useMatchUi } from './matchUi';
import { toStagePoint, useStage } from './stageContext';
import { lastPointerType } from './useLongPress';
import styles from './CardZoom.module.css';

const ZOOM_WIDTH = 340;
/** Settings: bigger card zoom, for reading on a small screen (#147) */
const BIG_WIDTH = 440;
/** broadcast: big enough to read on a stream, at the edge of the table */
const LARGE_WIDTH = 520;
const DELAY_MS = 260;
const EDGE = 24;
/** the Mechanics panel beside the card, and the gap between them */
const PANEL_WIDTH = 300;
const LARGE_PANEL_WIDTH = 420;
const PANEL_GAP = 14;

/**
 * The hovered card, large, beside the pointer (after a short delay so sweeping the board stays calm). `large`
 * (watching, for a stream): bigger still, at the edge of the table away from the pointer, where it hides less.
 */
export function CardZoom({ large = false, big = false }: { large?: boolean; big?: boolean }) {
  const zoom = useMatchUi((state) => state.zoom);
  const dragging = useMatchUi((state) => state.dragging);
  const detailOpen = useMatchUi((state) => !!state.detail);
  const t = useT();
  const stage = useStage();
  const [settled, setSettled] = useState<typeof zoom>(null);
  const hovered = zoom && !dragging && settled === zoom ? zoom.card : null;
  // what the card's keywords and mechanics mean, beside it (an ability on the stack has no card types)
  const terms = useMemo(() => (hovered ? explainCard(isStackAbility(hovered) ? { ...hovered, cardTypes: [] } : hovered).terms : []), [hovered]);

  useEffect(() => {
    if (!zoom || dragging) return;
    const timer = setTimeout(() => setSettled(zoom), DELAY_MS);
    return () => clearTimeout(timer);
  }, [zoom, dragging]);

  // show the hovered card only once the pointer has rested on it
  const shown = zoom && !dragging && settled === zoom ? zoom : null;
  // a card wearing a card style shows its painting: the rules text goes under it
  const styled = !!useCardStyle(shown && !isStackAbility(shown.card) ? shown.card as PermanentView : undefined);
  // a card that left its place while hovered (cast from hand, died) never reports the pointer leaving it
  // ...or that changed zone under the same id (a spell becoming a permanent): only the hovered element counts
  const element = shown?.anchor
    ?? (shown?.card.id ? stage.element?.querySelector(`[data-object-id="${CSS.escape(shown.card.id)}"]`) : null);
  // a finger doesn't hover: on touch the card opens with a long press instead (the detail view)
  const touch = lastPointerType() === 'touch';
  if (!shown || touch || detailOpen || !element || !element.isConnected || !element.matches(':hover')) return null;
  const card = shown.card as PermanentView;
  const back = card.secondCardFace;
  const extra = details(card);
  const rules = styled ? (card.rules ?? []).filter((line) => line.trim()) : [];
  const width = large ? LARGE_WIDTH : big ? BIG_WIDTH : ZOOM_WIDTH;
  const height = width * (88 / 63);
  const point = toStagePoint(stage, shown.x, shown.y);
  const pointerRight = point.x > stage.width / 2;
  // the second face sits beside the first: the panel is that much wider
  const span = back && card.transformable ? width * 1.62 + 12 : width;
  const left = large
    ? (pointerRight ? EDGE : stage.width - span - EDGE)
    : pointerRight ? point.x - width - 60 : point.x + 60;
  const top = large
    ? Math.max(EDGE, (stage.height - height) / 2 - 40)
    : Math.max(EDGE, Math.min(stage.height - height - EDGE, point.y - height / 2));

  // the panel goes on the side of the card away from the pointer (toward the middle of the table when large)
  const panelWidth = large ? LARGE_PANEL_WIDTH : PANEL_WIDTH;
  const panelRight = large ? pointerRight : !pointerRight;
  const panelLeft = panelRight
    ? Math.min(left + span + PANEL_GAP, stage.width - panelWidth - 8)
    : Math.max(8, left - panelWidth - PANEL_GAP);

  return (
    <div className={[styles.zoom, large ? styles.large : ''].join(' ')} style={{ left, top }} aria-hidden="true">
      <div className={styles.faces}>
        {isStackAbility(card)
          ? <AbilityCard ability={card} sleeve={shown.sleeve} style={{ width }} />
          : <CardFace card={card} size="large" sleeve={shown.sleeve} style={{ width }} />}
        {back && card.transformable && (
          <CardFace card={back} face={showsBackFace(card) ? 'front' : 'back'} size="normal" sleeve={shown.sleeve} style={{ width: width * 0.62 }} />
        )}
      </div>
      {rules.length > 0 && (
        <ul className={styles.rules}>
          {rules.map((line) => <li key={line}><PromptText text={line} /></li>)}
        </ul>
      )}
      {extra.length > 0 && (
        <ul className={styles.details}>
          {extra.map((line) => <li key={line}>{line}</li>)}
        </ul>
      )}
      {terms.length > 0 && (
        <aside className={styles.mechanics} style={{ left: panelLeft - left, width: panelWidth, maxHeight: stage.height - top - EDGE }}>
          <h3 className={styles.mechanicsTitle}>{t('match.zoom.mechanics')}</h3>
          <dl className={styles.terms}>
            {terms.map((term) => (
              <div key={term.name} className={styles.term}>
                <dt>{term.name}</dt>
                <dd><PromptText text={term.text} /></dd>
              </div>
            ))}
          </dl>
        </aside>
      )}
    </div>
  );
}

/** What the picture can't show: changed stats, counters, damage, who controls it. */
function details(card: PermanentView & CardView): string[] {
  const lines: string[] = [];
  if (card.power !== undefined && card.toughness !== undefined && (card.cardTypes ?? []).includes('CREATURE')) {
    const base = card.original;
    if (base && (base.power !== card.power || base.toughness !== card.toughness)) {
      lines.push(`Now ${card.power}/${card.toughness} (printed ${base.power}/${base.toughness})`);
    }
  }
  if ((card.damage ?? 0) > 0) lines.push(`${card.damage} damage marked`);
  for (const counter of card.counters ?? []) {
    if ((counter.count ?? 0) > 0) lines.push(`${counter.count} × ${counter.name}`);
  }
  if (card.nameController && card.nameOwner && card.nameOwner !== card.nameController) {
    lines.push(`Controlled by ${card.nameController}, owned by ${card.nameOwner}`);
  }

  return lines;
}
