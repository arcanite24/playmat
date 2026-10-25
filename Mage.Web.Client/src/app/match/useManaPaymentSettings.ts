import { useEffect } from 'react';
import type { GameSession } from '../../core/game/gameSession';
import { useSettings } from '../stores/settings';

/** User preferences initialize new games; an existing game's mana pool needs player actions too. */
export function useManaPaymentSettings(session: GameSession, active: boolean) {
  const tapLands = useSettings((state) => state.settings.autoPayMana);
  const restricted = useSettings((state) => state.settings.autoPayRestricted);
  const fullControl = useSettings((state) => state.fullControl);
  const automatic = !fullControl && (tapLands || !restricted);

  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    async function sync() {
      await session.respond({ type: 'action', action: restricted ? 'MANA_AUTO_PAYMENT_RESTRICTED_ON' : 'MANA_AUTO_PAYMENT_RESTRICTED_OFF' });
      if (!cancelled) await session.respond({ type: 'action', action: automatic ? 'MANA_AUTO_PAYMENT_ON' : 'MANA_AUTO_PAYMENT_OFF' });
    }
    void sync().catch(() => undefined);
    return () => { cancelled = true; };
  }, [session, active, restricted, automatic]);
}
