import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { ChevronDown } from 'lucide-react';
import {
  alwaysAnswerCommands, answerLabel, autoAnswerKey, isTriggerOrderPrompt, triggerLabel, triggerOrderCommands,
} from '../../core/game/autoAnswer';
import type { Command } from '../../core/game/interaction';
import type { Prompt } from '../../core/game/prompt';
import { useAutoAnswers } from '../stores/autoAnswers';
import { useT } from '../i18n';
import { Button } from '../ui/Button';
import { labelText } from './uiText';
import './matchMessages';
import styles from './AutoAnswer.module.css';

function send(onCommand: (command: Command) => void, commands: Command[]) {
  // the standing answer goes first, so the server has it before the answer that moves the game on
  for (const command of commands) onCommand(command);
}

/** "Always yes / Always no" for a yes/no question the server lets the player automate. */
export function AlwaysAnswerMenu({ gameId, prompt, onCommand }: { gameId: string; prompt: Prompt; onCommand(command: Command): void }) {
  const t = useT();
  const remember = useAutoAnswers((state) => state.remember);
  const key = autoAnswerKey(prompt);
  if (!key || prompt.kind !== 'ask') return null;
  const choose = (yes: boolean) => {
    remember({ gameId, kind: 'answer', label: answerLabel(key), choice: yes ? 'yes' : 'no' });
    send(onCommand, alwaysAnswerCommands(key, yes));
  };
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <Button variant="quiet" size="md" icon={<ChevronDown size={16} />}>{t('match.auto.menu')}</Button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content className={styles.menu} side="top" align="end" sideOffset={8}>
          <DropdownMenu.Label className={styles.menuLabel}>{t('match.auto.restOfGame')}</DropdownMenu.Label>
          <DropdownMenu.Item className={styles.menuItem} onSelect={() => choose(true)}>{t('match.auto.always', { answer: labelText(t, prompt.yesLabel) })}</DropdownMenu.Item>
          <DropdownMenu.Item className={styles.menuItem} onSelect={() => choose(false)}>{t('match.auto.always', { answer: labelText(t, prompt.noLabel) })}</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}

/** Trigger order shortcuts under the trigger picker: let the game order them, or fix a trigger's place for good. */
export function TriggerOrderOptions({ gameId, prompt, onCommand }: { gameId: string; prompt: Prompt; onCommand(command: Command): void }) {
  const t = useT();
  const remember = useAutoAnswers((state) => state.remember);
  const setAutoOrdering = useAutoAnswers((state) => state.setAutoOrdering);
  if (!isTriggerOrderPrompt(prompt)) return null;
  const triggers = Object.values(prompt.cards ?? {}).filter((card) => card.id && prompt.targets.includes(card.id));
  const place = (id: string, label: string, where: 'first' | 'last') => {
    remember({ gameId, kind: 'trigger', label, choice: where });
    send(onCommand, triggerOrderCommands(id, where));
  };
  return (
    <div className={styles.triggerOptions}>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger asChild>
          <Button variant="quiet" size="md" icon={<ChevronDown size={16} />}>{t('match.auto.menu')}</Button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content className={styles.menu} side="top" align="end" sideOffset={8}>
            <DropdownMenu.Label className={styles.menuLabel}>{t('match.auto.restOfGame')}</DropdownMenu.Label>
            {triggers.map((card, index) => {
              const label = triggerLabel(card);
              return (
                <DropdownMenu.Group key={card.id}>
                  {index > 0 && <DropdownMenu.Separator className={styles.menuRule} />}
                  <DropdownMenu.Label className={styles.menuTrigger}>{label}</DropdownMenu.Label>
                  <DropdownMenu.Item className={styles.menuItem} onSelect={() => place(card.id!, label, 'first')}>{t('match.auto.first')}</DropdownMenu.Item>
                  <DropdownMenu.Item className={styles.menuItem} onSelect={() => place(card.id!, label, 'last')}>{t('match.auto.last')}</DropdownMenu.Item>
                </DropdownMenu.Group>
              );
            })}
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
      <Button
        variant="print"
        size="md"
        // useAutoOrder answers this question and the rest of the batch
        onClick={() => setAutoOrdering(gameId, true)}
      >
        {t('match.auto.order')}
      </Button>
    </div>
  );
}
