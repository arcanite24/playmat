import { Fragment, useEffect, useLayoutEffect, useRef } from 'react';
import type { ManaChoice } from '../../core/game/manaChoices';
import { useT } from '../i18n';
import { Button } from '../ui/Button';
import { ManaCost } from '../ui/ManaCost';
import { PromptText } from '../ui/PromptText';
import { isEditableEventTarget } from '../ui/keys';
import { cleanText } from '../ui/text';
import { useMatchUi } from './matchUi';
import { useStage } from './stageContext';
import styles from './ManaPicker.module.css';

export function ManaPicker({ title, choices, sourceId, subMessage, onPick, onCancel, cancelLabel }: {
  title: string;
  choices: ManaChoice[];
  sourceId?: string | null;
  subMessage?: string;
  onPick(index: number): void;
  onCancel?: () => void;
  cancelLabel?: string;
}) {
  const t = useT();
  const stage = useStage();
  const panel = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const element = panel.current;
    if (!element) return;
    const place = () => {
      const source = sourceId && stage.element?.querySelector(`[data-object-id="${CSS.escape(sourceId)}"]`);
      const origin = stage.element?.getBoundingClientRect();
      if (!source || !origin) {
        element.style.removeProperty('left');
        element.style.removeProperty('top');
        return;
      }
      const card = source.getBoundingClientRect();
      const width = element.offsetWidth;
      const height = element.offsetHeight;
      const left = (card.left + card.width / 2 - origin.left) / stage.scale - width / 2;
      const above = (card.top - origin.top) / stage.scale - height - 16;
      const top = above >= 16 ? above : (card.bottom - origin.top) / stage.scale + 16;
      element.style.left = `${Math.max(16, Math.min(stage.width - width - 16, left))}px`;
      element.style.top = `${Math.max(16, Math.min(stage.height - height - 16, top))}px`;
    };
    place();
    const observer = new ResizeObserver(place);
    observer.observe(element);
    return () => observer.disconnect();
  }, [stage, sourceId]);

  useEffect(() => {
    const previous = document.activeElement;
    panel.current?.querySelector<HTMLButtonElement>('button')?.focus();
    return () => {
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || event.repeat || useMatchUi.getState().detail) return;
      if (isEditableEventTarget(event.target)) return;
      if (/^[1-9]$/.test(event.key) && Number(event.key) <= choices.length) {
        event.preventDefault();
        onPick(Number(event.key) - 1);
      } else if (event.key === 'Escape' && onCancel) {
        event.preventDefault();
        onCancel();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [choices.length, onPick, onCancel]);

  return (
    <div className={styles.scrim}>
      <div ref={panel} className={styles.picker} role="dialog" aria-modal="true" aria-label={cleanText(title)} onKeyDown={(event) => {
        if (useMatchUi.getState().detail) return;
        const buttons = Array.from(panel.current?.querySelectorAll<HTMLButtonElement>('button') ?? []);
        const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
        const forward = event.key === 'ArrowRight' || event.key === 'ArrowDown';
        const backward = event.key === 'ArrowLeft' || event.key === 'ArrowUp';
        if (forward || backward || event.key === 'Tab') {
          event.preventDefault();
          event.stopPropagation();
          const step = backward || (event.key === 'Tab' && event.shiftKey) ? -1 : 1;
          buttons[(index + step + buttons.length) % buttons.length]?.focus();
        }
      }}>
        <h2 className={styles.title}><PromptText text={title} /></h2>
        {subMessage && <p className={styles.detail}><PromptText text={subMessage} /></p>}
        <div className={styles.choices}>
          {choices.map((choice, index) => (
            <button key={index} type="button" className={styles.choice} aria-label={choice.label} title={choice.label} onClick={() => onPick(index)}>
              {choice.outputs.length > 0 && <span className={styles.symbols} aria-hidden="true">
                {choice.outputs.map((output, group) => (
                  <Fragment key={group}>
                    {group > 0 && <span className={styles.or}>{t('match.mana.or')}</span>}
                    <ManaCost cost={output} size="lg" />
                  </Fragment>
                ))}
              </span>}
              {choice.detail && <span className={styles.detail}><PromptText text={choice.detail} /></span>}
              {index < 9 && <kbd className={styles.key} aria-hidden="true">{index + 1}</kbd>}
            </button>
          ))}
        </div>
        <footer className={styles.footer}>
          <span>{t('match.mana.keys')}</span>
          {onCancel && <Button variant="quiet" onClick={onCancel}>{cancelLabel ?? t('match.ability.cancel')}</Button>}
        </footer>
      </div>
    </div>
  );
}
