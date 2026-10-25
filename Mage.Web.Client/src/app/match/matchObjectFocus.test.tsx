import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, test, vi } from 'vitest';
import type { Interaction } from '../../core/game/interaction';
import type { PermanentView } from '../../protocol/generated/views';
import { ActionCluster } from './ActionCluster';
import { Hand } from './Hand';
import { PermanentStack } from './PermanentStack';
import styles from './PermanentStack.module.css';
import { PlayerPlate } from './PlayerPlate';

const bear = { id: 'bear', name: 'Grizzly Bears', cardTypes: ['CREATURE'] } as PermanentView;
const main: Interaction = {
  mode: 'declareAttackers', prompt: null, headline: '', clickable: new Map(), selected: new Set(),
  mainButton: { label: 'Attack', command: { type: 'boolean', value: true }, tone: 'attack', shortcut: 'Space' },
  secondaryButtons: [],
};

function object(kind: string, onClick: () => void) {
  if (kind === 'hand') return <Hand cards={[bear]} clickable={new Set(['bear'])} selected={new Set()} sleeve="default" onPlay={onClick} playLine={600} libraryOrigin="library:me" />;
  if (kind === 'battlefield') return <PermanentStack group={{ key: 'bear', lead: bear, members: [bear], attachments: [] }} width={100} sleeve="default" clickable={new Set(['bear'])} selected={new Set()} attacking={new Set()} blocking={new Set()} forward={1} onClick={onClick} />;
  return <PlayerPlate player={{ playerId: 'opponent', name: 'Opponent', life: 20 }} isMe={false} targetable selected={false} deciding={false} onClick={onClick} />;
}

describe('match object focus', () => {
  test('available lands have the action border', () => {
    const land = { id: 'forest', name: 'Forest', cardTypes: ['LAND'] } as PermanentView;
    const { container } = render(<PermanentStack group={{ key: 'forest', lead: land, members: [land], attachments: [] }} width={100} sleeve="default" clickable={new Set(['forest'])} selected={new Set()} attacking={new Set()} blocking={new Set()} forward={1} onClick={vi.fn()} />);
    expect(container.querySelector('[data-object-id="forest"]')?.classList.contains(styles.clickable)).toBe(true);
  });

  test.each(['hand', 'battlefield', 'player'])('clicking a %s object leaves Space for the main action; Tab still activates it', async (kind) => {
    const user = userEvent.setup();
    const activate = vi.fn();
    const onCommand = vi.fn();
    const { container } = render(<>
      {object(kind, activate)}
      <ActionCluster interaction={main} awaiting={false} status={null} canAct special={false} holdingPriority={false} onCommand={onCommand} />
    </>);
    const target = container.querySelector<HTMLElement>('[data-object-id]')!;
    await user.click(target);
    expect(activate).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(document.body);
    await user.keyboard(' ');
    expect(onCommand).toHaveBeenCalledTimes(1);
    expect(onCommand).toHaveBeenCalledWith(main.mainButton!.command);
    expect(activate).toHaveBeenCalledTimes(1);
    await user.tab();
    expect(document.activeElement).toBe(target);
    await user.keyboard(' {Enter}');
    expect(activate).toHaveBeenCalledTimes(3);
    expect(onCommand).toHaveBeenCalledTimes(1);
    // A later mouse click also clears focus left by keyboard navigation.
    await user.click(target.firstElementChild!);
    expect(document.activeElement).toBe(document.body);
    await user.keyboard(' ');
    expect(activate).toHaveBeenCalledTimes(4);
    expect(onCommand).toHaveBeenCalledTimes(2);
  });

  test('mana buttons inside a player plate keep native focus and activation', async () => {
    const user = userEvent.setup();
    const onPay = vi.fn();
    const onClick = vi.fn();
    render(<PlayerPlate player={{ playerId: 'me', name: 'Me', life: 20, manaPool: { green: 1 } }} isMe targetable selected={false} deciding payable={new Set(['pool:GREEN'])} onPay={onPay} onClick={onClick} />);
    const mana = screen.getByRole('button', { name: 'Pay with green mana from your pool, 1 floating' });
    await user.click(mana.querySelector('b')!);
    expect(document.activeElement).toBe(mana);
    expect(onPay).toHaveBeenCalledWith('pool:GREEN');
    expect(onClick).not.toHaveBeenCalled();
  });
});
