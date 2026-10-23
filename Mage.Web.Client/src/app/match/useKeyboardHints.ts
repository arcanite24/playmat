import { useEffect, useState } from 'react';
import { isEditableEventTarget } from '../ui/keys';

/** Alt reveals the shortcuts; losing focus must never leave the hints stuck on. */
export function useKeyboardHints() {
  const [shown, setShown] = useState(false);
  useEffect(() => {
    const update = (event: KeyboardEvent) => {
      const editable = isEditableEventTarget(event.target);
      if (event.key === 'Alt' && !event.ctrlKey && !event.metaKey && !event.shiftKey && !editable) event.preventDefault();
      setShown(event.altKey && !event.ctrlKey && !event.metaKey && !editable);
    };
    const clear = () => setShown(false);
    window.addEventListener('keydown', update);
    window.addEventListener('keyup', update);
    window.addEventListener('blur', clear);
    document.addEventListener('visibilitychange', clear);
    return () => {
      window.removeEventListener('keydown', update);
      window.removeEventListener('keyup', update);
      window.removeEventListener('blur', clear);
      document.removeEventListener('visibilitychange', clear);
    };
  }, []);
  return shown;
}
