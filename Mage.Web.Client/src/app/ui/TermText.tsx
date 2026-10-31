import { Fragment, useEffect, useId, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { termSpans, type Term } from '../../core/game/glossary';
import type { CardView } from '../../protocol/generated/views';
import { manaClass } from './mana';
import { cleanText } from './text';
import styles from './TermText.module.css';

/** Mana symbols drawn, the rest as it is (PromptText's drawing, without trimming each piece). */
function Symbols({ text }: { text: string }) {
  return (
    <>
      {text.split(/(\{[^}]+\})/g).filter(Boolean).map((part, index) => {
        const symbol = /^\{([^}]+)\}$/.exec(part)?.[1];
        if (!symbol) return <Fragment key={index}>{part}</Fragment>;
        return <i key={index} className={`ms ms-cost ms-${manaClass(symbol)}`} role="img" aria-label={symbol} style={{ fontSize: '0.85em', margin: '0 0.08em' }} />;
      })}
    </>
  );
}

/** Where an open explanation sits: above its term, or below when there is no room above. */
interface Tip {
  index: number;
  term: Term;
  x: number;
  y: number;
  below: boolean;
}

const OPEN_DELAY = 150;

/**
 * Rules text like PromptText, with each keyword, action and ability word it names marked; pointing at one
 * (or focusing it) explains it, the way Arena does. Expects text already stripped of markup.
 *
 * The explanation is a small tooltip of its own rather than Radix's: Radix's positioning code would load with the
 * first page for a panel only the match shows.
 */
export function TermText({ text, card }: { text: string; card?: Pick<CardView, 'name' | 'displayName'> }) {
  const spans = useMemo(() => termSpans(cleanText(text), card), [text, card]);
  const [tip, setTip] = useState<Tip | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const tipId = useId();
  useEffect(() => () => {
    if (timer.current) clearTimeout(timer.current);
  }, []);

  const open = (index: number, term: Term, element: HTMLElement, delay: number) => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      const box = element.getBoundingClientRect();
      const below = box.top < 120;
      setTip({ index, term, x: box.left + box.width / 2, y: below ? box.bottom + 8 : box.top - 8, below });
    }, delay);
  };
  const close = () => {
    if (timer.current) clearTimeout(timer.current);
    setTip(null);
  };

  return (
    <>
      {spans.map((span, index) => {
        if (!span.term) return <Symbols key={index} text={span.text} />;
        const term = span.term;
        return (
          <span
            key={index}
            className={styles.term}
            tabIndex={0}
            data-term={term.name}
            aria-describedby={tip?.index === index ? tipId : undefined}
            onPointerEnter={(event) => open(index, term, event.currentTarget, OPEN_DELAY)}
            onPointerLeave={close}
            onFocus={(event) => open(index, term, event.currentTarget, 0)}
            onBlur={close}
            onKeyDown={(event) => {
              if (event.key === 'Escape') close();
            }}
          >
            <Symbols text={span.text} />
          </span>
        );
      })}
      {tip && createPortal(
        <span
          id={tipId}
          role="tooltip"
          className={styles.tip}
          data-below={tip.below ? '' : undefined}
          style={{ left: Math.min(Math.max(tip.x, 182), window.innerWidth - 182), top: tip.y }}
        >
          <b className={styles.tipName}>{tip.term.name}</b>
          <span><Symbols text={tip.term.text} /></span>
        </span>,
        document.body,
      )}
    </>
  );
}
