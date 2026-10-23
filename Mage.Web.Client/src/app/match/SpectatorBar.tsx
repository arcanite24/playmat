import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { Check, Eye, RadioTower } from 'lucide-react';
import { BROADCAST_DELAYS, delayLabel, isBroadcastDelay } from '../../core/game/broadcastDelay';
import type { GameSession, GameSessionState } from '../../core/game/gameSession';
import { useSettings } from '../stores/settings';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { useWatchers } from './useWatchers';
import styles from './SpectatorBar.module.css';

/** The eye and the number of people watching; their names on hover. */
export function WatcherCount({ watchers, className }: { watchers: string[]; className?: string }) {
  if (watchers.length === 0) return null;
  const label = `${watchers.length} watching: ${watchers.join(', ')}`;
  return (
    <span className={[styles.count, className ?? ''].join(' ')} title={label} aria-label={label} role="status">
      <Eye size={22} aria-hidden="true" />
      {watchers.length}
    </span>
  );
}

/** Watching or replaying: whose seat to see the table from, who else is watching, and the way out. */
export function SpectatorBar({ session, state, onLeave }: { session: GameSession; state: GameSessionState; onLeave(): void }) {
  const players = state.view?.players ?? [];
  const watchers = useWatchers(state.gameId, state.mode === 'watch' && !state.gameOver);
  const seat = state.playerId ?? players[0]?.playerId ?? '';
  const delaySeconds = Math.round(state.broadcastDelayMs / 1000);
  return (
    <div className={styles.bar}>
      <p className={styles.mode}>{state.mode === 'watch' ? 'Watching' : 'Replay'}</p>
      {delaySeconds > 0 && (
        <span className={styles.delay} role="status">Delayed {delayLabel(delaySeconds)}</span>
      )}
      {players.length > 1 && (
        <label className={styles.seat}>
          <span>Seat</span>
          <select value={seat} onChange={(event) => session.setPerspective(event.target.value || null)}>
            {players.map((player) => <option key={player.playerId} value={player.playerId}>{player.name}</option>)}
          </select>
        </label>
      )}
      <WatcherCount watchers={watchers} />
      <BroadcastMenu session={session} watching={state.mode === 'watch'} live={state.mode === 'watch' && !state.gameOver} />
      <Button variant="print" onClick={onLeave}>Leave</Button>
    </div>
  );
}

/** Options for streaming what one watches: a delay against ghosting, hidden hands, a card zoom viewers can read. */
function BroadcastMenu({ session, watching, live }: { session: GameSession; watching: boolean; live: boolean }) {
  const saved = useSettings((settings) => settings.settings.broadcastDelay);
  const hideHands = useSettings((settings) => settings.settings.hideHands);
  const largeZoom = useSettings((settings) => settings.settings.largeZoom);
  const update = useSettings((settings) => settings.update);
  const delay = isBroadcastDelay(saved) ? saved : 0;

  const askForHands = () => {
    session.requestToSeeHands()
      .then((asked) => {
        if (asked > 0) notify('Asked to see the hands', 'The computer shows its hand at once; players are asked first.');
      })
      .catch((error) => notify("Couldn't ask to see the hands", String(error?.message ?? error), 'error'));
  };

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <Button variant="print" icon={<RadioTower size={20} />}>Broadcast</Button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content className={styles.menu} side="top" align="end" sideOffset={10}>
          {live && (
            <>
              <DropdownMenu.Label className={styles.menuLabel}>Delay</DropdownMenu.Label>
              <DropdownMenu.RadioGroup value={String(delay)} onValueChange={(value) => update({ broadcastDelay: isBroadcastDelay(Number(value)) ? (Number(value) as typeof delay) : 0 })}>
                {BROADCAST_DELAYS.map((seconds) => (
                  <DropdownMenu.RadioItem key={seconds} value={String(seconds)} className={styles.menuItem} onSelect={(event) => event.preventDefault()}>
                    {delayLabel(seconds)}
                    <DropdownMenu.ItemIndicator className={styles.check}><Check size={16} aria-hidden="true" /></DropdownMenu.ItemIndicator>
                  </DropdownMenu.RadioItem>
                ))}
              </DropdownMenu.RadioGroup>
              <p className={styles.menuNote}>The game shows this far behind, so a stream can't give the players away.</p>
              <DropdownMenu.Separator className={styles.menuRule} />
            </>
          )}
          {watching && (
            <DropdownMenu.CheckboxItem className={styles.menuItem} checked={hideHands} onCheckedChange={(on) => update({ hideHands: on })} onSelect={(event) => event.preventDefault()}>
              Hide hands
              <kbd>{hideHands ? 'On' : 'Off'}</kbd>
            </DropdownMenu.CheckboxItem>
          )}
          <DropdownMenu.CheckboxItem className={styles.menuItem} checked={largeZoom} onCheckedChange={(on) => update({ largeZoom: on })} onSelect={(event) => event.preventDefault()}>
            Large card zoom
            <kbd>{largeZoom ? 'On' : 'Off'}</kbd>
          </DropdownMenu.CheckboxItem>
          {live && (
            <DropdownMenu.Item className={styles.menuItem} onSelect={askForHands}>Ask to see the hands</DropdownMenu.Item>
          )}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
