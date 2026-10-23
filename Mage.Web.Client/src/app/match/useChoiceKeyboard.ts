import { useEffect, useRef } from 'react';
import { ignoreMatchKey, numberKeyIndex, promptKey } from './keyboard';
import { useMatchUi } from './matchUi';

const ENABLED = ':not(:disabled):not([aria-disabled="true"])';
const FOCUSABLE = `button${ENABLED}, input${ENABLED}, select${ENABLED}, textarea${ENABLED}, [tabindex="0"]`;

/** Each choice owns its keys and focus, including when another dialog opens above it. */
export function useChoiceKeyboard(active = true) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const root = ref.current;
    if (!active || !root) return;
    const previous = document.activeElement;
    root.focus();
    function onKey(event: KeyboardEvent) {
      if (useMatchUi.getState().detail) return;
      const layers = document.querySelectorAll('[role="dialog"], [role="alertdialog"], [role="menu"]');
      if (layers[layers.length - 1] !== root || event.defaultPrevented || event.isComposing) return;
      const controls = Array.from(root!.querySelectorAll<HTMLElement>(FOCUSABLE));
      if (event.key === 'Tab' && controls.length) {
        const index = controls.indexOf(document.activeElement as HTMLElement);
        if (index < 0 || (event.shiftKey ? index === 0 : index === controls.length - 1)) {
          event.preventDefault();
          controls[event.shiftKey ? controls.length - 1 : 0].focus();
        }
        return;
      }
      if (event.repeat && (event.key === 'Enter' || event.key === ' ') && event.target instanceof HTMLElement && event.target.closest('button')) {
        event.preventDefault();
        return;
      }
      if (ignoreMatchKey(event)) return;
      const options = Array.from(root!.querySelectorAll<HTMLButtonElement>(`[data-choice]${ENABLED}`));
      const action = promptKey(event);
      let target: HTMLButtonElement | null | undefined;
      if (action && typeof action === 'object') {
        event.preventDefault();
        target = root!.querySelectorAll<HTMLButtonElement>(`[data-secondary]${ENABLED}`)[action.secondary];
      } else if (action === 'cancel') {
        target = root!.querySelector<HTMLButtonElement>(`[data-cancel]${ENABLED}`);
      } else if (!event.altKey && !event.ctrlKey && !event.metaKey && !event.shiftKey) {
        const index = numberKeyIndex(event);
        if (index !== null) target = options[index];
        else if (['ArrowDown', 'ArrowRight', 'ArrowUp', 'ArrowLeft', 'Home', 'End'].includes(event.key) && options.length) {
          const current = options.indexOf(document.activeElement as HTMLButtonElement);
          const backwards = event.key === 'ArrowUp' || event.key === 'ArrowLeft';
          const next = event.key === 'Home' ? 0 : event.key === 'End' ? options.length - 1
            : current < 0 ? (backwards ? options.length - 1 : 0) : (current + (backwards ? -1 : 1) + options.length) % options.length;
          event.preventDefault();
          options[next].focus();
          return;
        }
      }
      if (target) {
        event.preventDefault();
        target.focus();
        target.click();
      }
    }
    window.addEventListener('keydown', onKey, true);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      if (previous instanceof HTMLElement && previous.isConnected && (root.contains(document.activeElement) || document.activeElement === document.body)) previous.focus();
    };
  }, [active]);
  return ref;
}
