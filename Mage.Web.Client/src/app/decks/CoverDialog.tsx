import { Images, Sparkles } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useCardImage } from '../../core/images/useCardImage';
import type { DeckCardInfo, DeckCardLists, DeckCoverCard } from '../../core/decks/types';
import { useT } from '../i18n';
import { Dialog } from '../ui/Dialog';
import { useCardInfo } from './cardInfo';
import { chooseCover, coverFor, entryKey, type PrintingRef } from './deckModel';
import { PrintingPicker } from './PrintingPicker';
import styles from './CoverDialog.module.css';

/**
 * Choose the deck's cover (#167): any card in the deck, in the deck's printing or another, or back to an automatic
 * cover. `onChange` gets the deck with its new cover.
 */
export function CoverDialog({ open, onOpenChange, deck, onChange }: {
  open: boolean;
  onOpenChange(open: boolean): void;
  deck: DeckCardLists | null;
  onChange(deck: DeckCardLists): void;
}) {
  const t = useT();
  return (
    <Dialog open={open && !!deck} onOpenChange={onOpenChange} title={t('decks.cover.title')} description={t('decks.cover.lead')} width="xl">
      {open && deck && <CoverChoices deck={deck} onChange={(next) => {
        onChange(next);
        onOpenChange(false);
      }} />}
    </Dialog>
  );
}

function CoverChoices({ deck, onChange }: { deck: DeckCardLists; onChange(deck: DeckCardLists): void }) {
  const t = useT();
  const entries = useMemo(() => [...deck.cards, ...deck.sideboard], [deck.cards, deck.sideboard]);
  const info = useCardInfo(entries);
  const [printingsOf, setPrintingsOf] = useState<DeckCardInfo | null>(null);
  // one choice per card name: the deck's printing (the first, when it plays several)
  const cards = useMemo(() => {
    const seen = new Set<string>();
    return entries
      .filter((entry) => entry.setCode && entry.cardNumber)
      .filter((entry) => {
        const name = entry.cardName.toLowerCase();
        if (seen.has(name)) return false;
        seen.add(name);
        return true;
      })
      .sort((a, b) => a.cardName.localeCompare(b.cardName));
  }, [entries]);
  const { cover, pinned } = coverFor(deck, info);
  const coverName = cover?.name?.toLowerCase();
  const pick = (printing: PrintingRef | null) => onChange(chooseCover(deck, printing, info));

  return (
    <>
      <div className={styles.grid}>
        <button type="button" className={[styles.option, !pinned ? styles.on : ''].join(' ')} aria-pressed={!pinned} onClick={() => pick(null)}>
          <span className={styles.auto} aria-hidden="true"><Sparkles size={28} /></span>
          <span className={styles.caption}>
            <b>{t('decks.cover.auto')}</b>
            <small>{t('decks.cover.autoDetail')}</small>
          </span>
        </button>
        {cards.map((entry) => {
          const on = pinned && coverName === entry.cardName.toLowerCase();
          // the chosen cover's own printing, when it isn't the deck's
          const shown: DeckCoverCard = on && cover ? cover : { setCode: entry.setCode!, cardNumber: entry.cardNumber!, name: entry.cardName };
          return (
            <div key={entryKey(entry)} className={[styles.option, on ? styles.on : ''].join(' ')}>
              <button
                type="button"
                className={styles.pick}
                aria-pressed={on}
                onClick={() => pick({ cardName: entry.cardName, setCode: entry.setCode!, cardNumber: entry.cardNumber! })}
              >
                <CoverArt cover={shown} />
                <span className={styles.caption}>
                  <b>{entry.cardName}</b>
                  <small>{shown.setCode} #{shown.cardNumber}</small>
                </span>
              </button>
              <button
                type="button"
                className={styles.printings}
                title={t('decks.cover.otherPrinting')}
                aria-label={t('decks.cover.otherPrintingOf', { name: entry.cardName })}
                onClick={() => setPrintingsOf(entry)}
              >
                <Images size={15} aria-hidden="true" />
              </button>
            </div>
          );
        })}
      </div>
      {cards.length === 0 && <p className={styles.note}>{t('decks.cover.empty')}</p>}
      <PrintingPicker
        name={printingsOf?.cardName ?? null}
        current={pinned && cover && coverName === printingsOf?.cardName.toLowerCase() ? `${cover.setCode}:${cover.cardNumber}` : printingsOf ? entryKey(printingsOf) : null}
        description={t('decks.cover.printingLead')}
        onClose={() => setPrintingsOf(null)}
        onPick={(printing) => pick(printing)}
      />
    </>
  );
}

function CoverArt({ cover }: { cover: DeckCoverCard }) {
  const art = useCardImage(cover, 'front', 'art_crop');
  return (
    <span className={styles.art} style={typeof art === 'string' ? { backgroundImage: `url("${art}")` } : undefined} aria-hidden="true" />
  );
}
