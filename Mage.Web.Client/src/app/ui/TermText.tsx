import * as Tooltip from '@radix-ui/react-tooltip';
import { Fragment, useMemo } from 'react';
import { termSpans } from '../../core/game/glossary';
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

/**
 * Rules text like PromptText, with each keyword, action and ability word it names marked; pointing at one
 * (or focusing it) explains it, the way Arena does. Expects text already stripped of markup.
 */
export function TermText({ text, card }: { text: string; card?: Pick<CardView, 'name' | 'displayName'> }) {
  const spans = useMemo(() => termSpans(cleanText(text), card), [text, card]);
  return (
    <>
      {spans.map((span, index) => {
        if (!span.term) return <Symbols key={index} text={span.text} />;
        return (
          <Tooltip.Root key={index} delayDuration={150}>
            <Tooltip.Trigger asChild>
              <span className={styles.term} tabIndex={0} data-term={span.term.name}><Symbols text={span.text} /></span>
            </Tooltip.Trigger>
            <Tooltip.Portal>
              <Tooltip.Content className={styles.tip} side="top" sideOffset={6} collisionPadding={12}>
                <b className={styles.tipName}>{span.term.name}</b>
                <span><Symbols text={span.term.text} /></span>
                <Tooltip.Arrow className={styles.arrow} />
              </Tooltip.Content>
            </Tooltip.Portal>
          </Tooltip.Root>
        );
      })}
    </>
  );
}
