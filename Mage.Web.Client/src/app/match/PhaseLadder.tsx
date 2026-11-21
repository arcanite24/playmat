import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import type { CombatStops } from '../../core/game/autoPass';
import type { PhaseStep, SkipPrioritySteps } from '../../protocol/generated/views';
import { useT, type MessageKey } from '../i18n';
import './matchMessages';
import { useSettings } from '../stores/settings';
import { useStage } from './stageContext';
import { useCoarsePointer } from './useLongPress';
import styles from './PhaseLadder.module.css';

interface Rung {
  label: MessageKey;
  steps: PhaseStep[];
  /** the stop setting this rung toggles */
  stop?: keyof SkipPrioritySteps;
  /** a combat stop (kept by the client: the server always offers priority there) */
  combat?: keyof CombatStops;
  /** the name in the touch menu of stops, where the two mains need telling apart */
  menuLabel?: MessageKey;
}

const RUNGS: Rung[] = [
  { label: 'match.phase.upkeep', steps: ['UNTAP', 'UPKEEP'], stop: 'upkeep' },
  { label: 'match.phase.draw', steps: ['DRAW'], stop: 'draw' },
  { label: 'match.phase.main', steps: ['PRECOMBAT_MAIN'], stop: 'main1', menuLabel: 'match.phase.main1' },
  { label: 'match.phase.combat', steps: ['BEGIN_COMBAT'], stop: 'beforeCombat' },
  { label: 'match.phase.attack', steps: ['DECLARE_ATTACKERS'], combat: 'attackers' },
  { label: 'match.phase.block', steps: ['DECLARE_BLOCKERS'], combat: 'blockers' },
  { label: 'match.phase.damage', steps: ['FIRST_COMBAT_DAMAGE', 'COMBAT_DAMAGE', 'END_COMBAT'], stop: 'endOfCombat' },
  { label: 'match.phase.main', steps: ['POSTCOMBAT_MAIN'], stop: 'main2', menuLabel: 'match.phase.main2' },
  { label: 'match.phase.end', steps: ['END_TURN', 'CLEANUP'], stop: 'endOfTurn' },
];

/**
 * Where the turn is, printed down the right edge (across the middle of the table in the tall layout). Dots mark
 * where the game stops for you on this player's turn; click a rung to toggle that stop. On a touch screen the rungs
 * are too small to aim at: the whole ladder is one button that opens the stops as a menu of full-size rows.
 */
export function PhaseLadder({ step, myTurn, turn, started = true, activeName = null }: {
  step: PhaseStep | undefined;
  myTurn: boolean;
  turn: number;
  /** false before the first turn (no active player yet): the turn belongs to nobody */
  started?: boolean;
  /** watching: whose turn it is, by name (a spectator has no turn of their own) */
  activeName?: string | null;
}) {
  const t = useT();
  const settings = useSettings((state) => state.settings);
  const update = useSettings((state) => state.update);
  const side = myTurn ? 'yourTurn' : 'opponentTurn';
  const stops = settings.stops[side] ?? {};
  const combatStops = settings.combatStops[side];
  const fullControl = useSettings((state) => state.fullControl);
  const coarse = useCoarsePointer();
  const tall = useStage().layout === 'tall';

  const toggle = (rung: Rung, on: boolean) => {
    if (rung.stop) update({ stops: { ...settings.stops, [side]: { ...stops, [rung.stop]: on } } });
    else if (rung.combat) update({ combatStops: { ...settings.combatStops, [side]: { ...combatStops, [rung.combat]: on } } });
  };
  const whose = myTurn ? 'yours' : 'theirs';

  return (
    <nav
      className={[styles.ladder, started && myTurn ? styles.mine : styles.theirs].join(' ')}
      aria-label={!started
        ? t('match.turn.aria.starting')
        : activeName ? t('match.turn.aria.player', { turn, name: activeName }) : t(myTurn ? 'match.turn.aria.yours' : 'match.turn.aria.theirs', { turn })}
    >
      <div className={styles.turn}>
        <span className={styles.turnNumber}>{started ? t('match.turn', { turn }) : t('match.turn.starting')}</span>
        {started && <span className={[styles.whose, activeName ? styles.whoseName : ''].join(' ')}>{activeName ?? t(myTurn ? 'match.turn.yours' : 'match.turn.theirs')}</span>}
        {fullControl && <span className={styles.full}>{t('match.ui.fullControl')}</span>}
      </div>
      <ol className={styles.rungs}>
        {RUNGS.map((rung, index) => {
          const current = !!step && rung.steps.includes(step);
          const settable = !!(rung.stop || rung.combat);
          const ownStop = rung.stop ? !!stops[rung.stop] : rung.combat ? !!combatStops?.[rung.combat] : false;
          // under full control the game stops everywhere; the player's own stops come back when it is turned off
          const stopOn = fullControl || ownStop;
          if (coarse) {
            return (
              <li key={index}>
                <span className={[styles.rung, current ? styles.current : ''].join(' ')} aria-current={current ? 'step' : undefined}>
                  <span className={[styles.dot, stopOn ? styles.dotOn : ''].join(' ')} aria-hidden="true" />
                  {t(rung.label)}
                </span>
              </li>
            );
          }
          return (
            <li key={index}>
              <button
                type="button"
                className={[styles.rung, current ? styles.current : ''].join(' ')}
                aria-current={current ? 'step' : undefined}
                aria-pressed={settable ? stopOn : undefined}
                disabled={!settable || fullControl}
                title={fullControl
                  ? t('match.phase.fullControl')
                  : rung.combat
                    ? t(`match.phase.${ownStop ? 'stop' : 'noStop'}.${whose}.attack`, { step: t(rung.label) })
                    : rung.stop ? t(`match.phase.${ownStop ? 'stop' : 'noStop'}.${whose}`, { step: t(rung.label) }) : undefined}
                onClick={() => toggle(rung, !ownStop)}
              >
                <span className={[styles.dot, stopOn ? styles.dotOn : ''].join(' ')} aria-hidden="true" />
                {t(rung.label)}
              </button>
            </li>
          );
        })}
      </ol>
      {coarse && (
        <DropdownMenu.Root>
          <DropdownMenu.Trigger
            className={styles.touchTrigger}
            disabled={fullControl}
            aria-label={fullControl ? t('match.phase.fullControl') : t(`match.phase.stops.${whose}`)}
          />
          <DropdownMenu.Portal>
            <DropdownMenu.Content className={styles.menu} side={tall ? 'top' : 'left'} align="center" sideOffset={8} collisionPadding={8}>
              <DropdownMenu.Label className={styles.menuLabel}>{t(`match.phase.menu.${whose}`)}</DropdownMenu.Label>
              {RUNGS.filter((rung) => rung.stop || rung.combat).map((rung) => {
                const on = rung.stop ? !!stops[rung.stop] : !!combatStops?.[rung.combat!];
                return (
                  <DropdownMenu.CheckboxItem
                    key={rung.stop ?? rung.combat}
                    className={styles.menuItem}
                    checked={on}
                    onCheckedChange={(checked) => toggle(rung, checked)}
                    // several stops are often changed at once: the menu stays open
                    onSelect={(event) => event.preventDefault()}
                  >
                    {rung.combat ? t('match.phase.whenAttack', { step: t(rung.label) }) : t(rung.menuLabel ?? rung.label)}
                    <span className={[styles.dot, on ? styles.dotOn : ''].join(' ')} aria-hidden="true" />
                  </DropdownMenu.CheckboxItem>
                );
              })}
            </DropdownMenu.Content>
          </DropdownMenu.Portal>
        </DropdownMenu.Root>
      )}
    </nav>
  );
}
