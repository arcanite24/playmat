import { useEffect, useState } from 'react';
import { registerMessages } from '../i18n';

/**
 * The free-play solo messages (ghosts, meta opponents, adaptive, the inbox) live in the Career catalog, which stays out
 * of the entry chunk: screens in the entry load it here when they first need it.
 */
let loaded = false;
let loading: Promise<void> | null = null;

export function loadCareerMessages(): Promise<void> {
  loading ??= import('../i18n/en/career').then((module) => {
    registerMessages(module.default);
    loaded = true;
  });
  return loading;
}

/** True once the Career messages are registered; starts loading them when `wanted`. */
export function useCareerMessages(wanted = true): boolean {
  const [ready, setReady] = useState(loaded);
  useEffect(() => {
    if (!wanted || ready) return;
    let live = true;
    void loadCareerMessages().then(() => live && setReady(true));
    return () => {
      live = false;
    };
  }, [wanted, ready]);
  return ready;
}
