import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { ChevronUp } from 'lucide-react';
import { useEffect, useMemo, type ReactNode } from 'react';
import type { Command, Interaction, PromptButton } from '../../core/game/interaction';
import type { CardView, PlayerAction } from '../../protocol/generated/views';
import { useT } from '../i18n';
import { isEditableEventTarget } from '../ui/keys';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { PromptText } from '../ui/PromptText';
import './matchMessages';
import { useMatchUi } from './matchUi';
import { ignoreMatchKey, matchKeysBlocked, promptKey } from './keyboard';
import { useKeyboardHints } from './useKeyboardHints';
import styles from './ActionCluster.module.css';

export interface ActionClusterProps {
  interaction: Interaction;
  awaiting: boolean;
  status: string | null;
  canAct: boolean;
  /** server offers a special action (e.g. turning a face-down creature up) */
  special: boolean;
  holdingPriority: boolean;
  /** the server kept an undo point for the player's last mana taps (mana floating, nothing cast since) */
  undoable?: boolean;
  /** the client is answering for the player (nothing to do): show that instead of a button to press */
  autoPassing?: boolean;
  /** more ways to answer, beside the secondary buttons (e.g. "Always…") */
  extra?: ReactNode;
  /** Arena's full control: stop at every step, never pass for the player */
  fullControl?: boolean;
  onFullControl?(on: boolean): void;
  /** the last answer got no reaction from the server in time */
  stalled?: boolean;
  onResend?(): void;
  onResync?(): void;
  /** the player conceded and the server hasn't ended the game yet; waitingOn: who holds priority meanwhile */
  conceding?: { waitingOn: string | null } | null;
  /** the cards the question is about (an effect looked at them just before asking), shown with it */
  cards?: CardView[] | null;
  onCommand(command: Command): void;
}

const PASS_AHEAD: { label: string; action: PlayerAction; key: string }[] = [
  { label: 'Pass to end of turn', action: 'PASS_PRIORITY_UNTIL_TURN_END_STEP', key: 'F4' },
  { label: 'Pass to next turn', action: 'PASS_PRIORITY_UNTIL_NEXT_TURN', key: 'F5' },
  { label: 'Pass to my next turn', action: 'PASS_PRIORITY_UNTIL_MY_NEXT_TURN', key: 'F9' },
  { label: 'Resolve the stack', action: 'PASS_PRIORITY_UNTIL_STACK_RESOLVED', key: 'F10' },
];

const UNDO: PromptButton = { label: 'Undo tap', command: { type: 'action', action: 'UNDO' }, tone: 'secondary', shortcut: 'Ctrl+Z' };

/** The decision corner: what the game is asking, and the one big button that answers it. */
export function ActionCluster({
  interaction, awaiting, status, canAct, special, holdingPriority, undoable = false, autoPassing = false, fullControl = false, onFullControl, extra,
  stalled = false, onResend, onResync, conceding = null, cards = null, onCommand,
}: ActionClusterProps) {
  const t = useT();
  const openViewer = useMatchUi((state) => state.openViewer);
  const main = interaction.mainButton;
  const primaryShortcut = interaction.mode !== 'question' || main?.shortcut === 'Space';
  const hints = useKeyboardHints();
  // Never undo while paying: Cancel rolls back the half-cast spell and its mana taps (#55).
  const undo = undoable && interaction.mode === 'priority' && canAct ? UNDO : null;
  const secondary = useMemo(() => {
    const buttons = [...interaction.secondaryButtons];
    if (special && interaction.mode === 'priority') {
      buttons.unshift({ label: 'Special action', command: { type: 'string', value: 'special' }, tone: 'secondary' });
    }
    if (undo) buttons.unshift(undo);
    return buttons;
  }, [interaction.secondaryButtons, interaction.mode, special, undo]);
  const cancel = secondary.find((button) => button.label === 'Cancel');

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (ignoreMatchKey(event) || matchKeysBlocked() || !canAct) return;
      const key = promptKey(event);
      const focusedControl = event.target instanceof HTMLElement && event.target.closest('button, [role="button"], [role="listitem"], a');
      let command: Command | undefined;
      if (key === 'primary' && primaryShortcut && !focusedControl && !awaiting && !autoPassing) {
        command = main?.command;
      } else if (key && typeof key === 'object' && interaction.mode !== 'waiting') {
        const button = secondary[key.secondary];
        if (!awaiting || (interaction.mode === 'payMana' && button === cancel)) command = button?.command;
      } else if (key === 'cancel' && (!awaiting || interaction.mode === 'payMana')) {
        command = cancel?.command;
      } else if (undo && !awaiting && (event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === 'z') {
        command = undo.command;
      } else if (!event.altKey && !event.ctrlKey && !event.metaKey && !event.shiftKey && (interaction.mode === 'priority' || interaction.mode === 'waiting')) {
        const ahead = PASS_AHEAD.find((item) => item.key === event.key);
        if (ahead) command = { type: 'action', action: ahead.action };
      }
      // Reserved Alt shortcuts must not fall through to a focused button's native activation.
      if (command || (key && typeof key === 'object')) {
        event.preventDefault();
        event.stopPropagation();
        if (command) onCommand(command);
      }
    }
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [main, primaryShortcut, secondary, cancel, undo, awaiting, autoPassing, canAct, interaction.mode, onCommand]);

  // Ctrl+Shift on its own, as on Arena, locks full control on or off (released without any other key in between)
  useEffect(() => {
    if (!canAct || !onFullControl) return;
    let armed = false;
    const isModifier = (event: KeyboardEvent) => event.key === 'Control' || event.key === 'Shift';
    function onDown(event: KeyboardEvent) {
      armed = !ignoreMatchKey(event) && !matchKeysBlocked() && isModifier(event) && event.ctrlKey && event.shiftKey && !event.altKey && !event.metaKey;
    }
    function onUp(event: KeyboardEvent) {
      if (!armed || !isModifier(event)) return;
      armed = false;
      if (!isEditableEventTarget(event.target) && !matchKeysBlocked()) onFullControl!(!fullControl);
    }
    const disarm = () => { armed = false; };
    window.addEventListener('keydown', onDown);
    window.addEventListener('keyup', onUp);
    window.addEventListener('blur', disarm);
    return () => {
      window.removeEventListener('keydown', onDown);
      window.removeEventListener('keyup', onUp);
      window.removeEventListener('blur', disarm);
    };
  }, [canAct, fullControl, onFullControl]);

  // the answered prompt stays up while the server works (see the session's hold), so nothing blinks between decisions
  const deciding = interaction.mode !== 'waiting';
  // every decision is asked here, next to its buttons, so the question never covers the cards it is about
  const headline = deciding ? interaction.headline : status ?? interaction.headline;
  const passAhead = canAct && (interaction.mode === 'priority' || interaction.mode === 'waiting');

  return (
    <section className={styles.cluster} aria-label="Your decision" aria-live="polite">
      {headline && <p className={[styles.headline, deciding ? styles.deciding : ''].join(' ')}><PromptText text={headline} /></p>}
      {fullControl && canAct && (
        <button type="button" className={styles.fullControl} onClick={() => onFullControl?.(false)} title="Turn full control off (Ctrl+Shift)">
          Full control
        </button>
      )}
      {conceding && (
        <p className={styles.conceding} role="status">
          {conceding.waitingOn ? t('match.conceding.waiting', { name: conceding.waitingOn }) : t('match.conceding')}
        </p>
      )}
      {cards && cards.length > 0 && deciding && (
        <button
          type="button"
          className={styles.cards}
          onClick={() => openViewer({ title: t('match.ask.cards', { count: cards.length }), cards })}
          aria-label={`${t('match.ask.cards', { count: cards.length })}. ${t('match.ask.open')}`}
        >
          {cards.map((card, index) => (
            <span key={card.id ?? index} className={styles.cardThumb}><CardFace card={card} size="small" /></span>
          ))}
        </button>
      )}
      {stalled && canAct && !conceding && (
        <div className={styles.stalled} role="alert">
          <p>The server hasn't answered.</p>
          <div className={styles.stalledButtons}>
            {onResend && <Button variant="quiet" size="md" onClick={onResend}>Send again</Button>}
            {onResync && <Button variant="print" size="md" onClick={onResync}>Refresh board</Button>}
          </div>
        </div>
      )}
      {(secondary.length > 0 || extra) && deciding && (
        <div className={styles.secondary}>
          {extra}
          {secondary.map((button, index) => (
            <Button
              key={button.label}
              variant={button.tone === 'danger' ? 'danger' : button.tone === 'attack' ? 'print' : 'print'}
              size="md"
              aria-keyshortcuts={index < 10 ? `Alt+${(index + 1) % 10}${index === 0 ? ' Alt+Enter' : ''}` : undefined}
              title={button.shortcut && button.shortcut !== 'Space' ? `${button.label} (${button.shortcut})` : undefined}
              onKeyDown={(event) => {
                if (event.repeat && (event.key === 'Enter' || event.key === ' ')) event.preventDefault();
              }}
              onClick={() => onCommand(button.command)}
              className={button.tone === 'attack' ? styles.attack : undefined}
            >
              {button.label}
              {hints && index < 10 && <kbd aria-hidden="true">Alt+{(index + 1) % 10}</kbd>}
            </Button>
          ))}
        </div>
      )}
      <div className={styles.mainRow}>
        {passAhead && (
          <DropdownMenu.Root>
            <DropdownMenu.Trigger className={styles.ahead} aria-label="Pass ahead">
              <ChevronUp size={20} />
            </DropdownMenu.Trigger>
            <DropdownMenu.Portal>
              <DropdownMenu.Content className={styles.menu} side="top" align="end" sideOffset={8}>
                {PASS_AHEAD.map((item) => (
                  <DropdownMenu.Item key={item.action} className={styles.menuItem} onSelect={() => onCommand({ type: 'action', action: item.action })}>
                    {item.label}<kbd>{item.key}</kbd>
                  </DropdownMenu.Item>
                ))}
                <DropdownMenu.Separator className={styles.menuRule} />
                <DropdownMenu.CheckboxItem
                  className={styles.menuItem}
                  checked={holdingPriority}
                  onCheckedChange={(on) => onCommand({ type: 'action', action: on ? 'HOLD_PRIORITY' : 'UNHOLD_PRIORITY' })}
                >
                  Hold priority after casting
                  <kbd>{holdingPriority ? 'On' : 'Off'}</kbd>
                </DropdownMenu.CheckboxItem>
                {onFullControl && (
                  <DropdownMenu.CheckboxItem
                    className={styles.menuItem}
                    checked={fullControl}
                    onCheckedChange={(on) => onFullControl(on)}
                  >
                    Full control: stop at every step
                    <kbd>Ctrl+Shift</kbd>
                  </DropdownMenu.CheckboxItem>
                )}
              </DropdownMenu.Content>
            </DropdownMenu.Portal>
          </DropdownMenu.Root>
        )}
        {autoPassing ? (
          <div className={[styles.waiting, styles.passing].join(' ')} role="status">Passing</div>
        ) : main && deciding ? (
          <Button
            variant="decision"
            size="xl"
            className={[styles.main, awaiting ? styles.sending : ''].join(' ')}
            // while the last answer is on its way a click can't be taken (one answer per question): show it
            aria-keyshortcuts={primaryShortcut ? 'Space Enter' : undefined}
            aria-disabled={awaiting || undefined}
            onKeyDown={(event) => {
              if (event.repeat && (event.key === 'Enter' || event.key === ' ')) event.preventDefault();
            }}
            onClick={() => onCommand(main.command)}
          >
            {main.label}
            {hints && primaryShortcut && <kbd aria-hidden="true">{t('match.shortcuts.primaryKeys')}</kbd>}
          </Button>
        ) : deciding ? null : (
          <div className={styles.waiting}>{awaiting ? 'Sending…' : 'Waiting'}</div>
        )}
      </div>
    </section>
  );
}
