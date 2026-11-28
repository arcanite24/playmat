import { act, fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { noteBeats } from './beats';
import { PlayerPlate } from './PlayerPlate';
import styles from './PlayerPlate.module.css';

describe('player plate identity', () => {
  it('draws a saved sigil at the match seat and keeps the initial fallback', () => {
    const props = { isMe: true, targetable: false, selected: false, deciding: false, onClick: () => undefined };
    const { container, rerender } = render(<PlayerPlate {...props} player={{ name: 'Player', life: 20, userData: { avatarId: 12 } }} />);
    const avatar = () => container.querySelector(`.${styles.avatar}`)!;
    expect(avatar().querySelector('svg path')?.getAttribute('d')).toBeTruthy();
    expect(avatar().textContent).toBe('');

    rerender(<PlayerPlate {...props} player={{ name: 'Player', life: 20, userData: { avatarId: 10 } }} />);
    expect(avatar().querySelector('svg')).toBeNull();
    expect(avatar().textContent).toBe('P');
  });
});

describe('player plate life', () => {
  it('changes the total when the hit noted for that player lands, not when the view arrives', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance'] });
    try {
      const props = { isMe: false, targetable: false, selected: false, deciding: false, onClick: () => undefined };
      const { container, rerender } = render(<PlayerPlate {...props} player={{ playerId: 'bob', name: 'Bob', life: 20 }} />);
      // the swing that hits Bob connects 400ms into the view
      noteBeats({ turn: 5 }, { impact: 400, land: 0, settle: 600, changes: new Map([['bob', 400]]) });
      const life = () => container.querySelector('[aria-label^="Bob,"]')?.getAttribute('aria-label');
      rerender(<PlayerPlate {...props} player={{ playerId: 'bob', name: 'Bob', life: 14 }} />);
      await act(async () => vi.advanceTimersByTime(300));
      expect(life()).toMatch(/^Bob, 20 life/);
      await act(async () => vi.advanceTimersByTime(150));
      expect(life()).toMatch(/^Bob, 14 life/);
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('player plate hud', () => {
  const props = { targetable: false, selected: false, onClick: () => undefined };

  it('counts how long an AI opponent has been thinking', () => {
    vi.useFakeTimers();
    try {
      const { rerender } = render(<PlayerPlate {...props} isMe={false} deciding player={{ name: 'AI Opponent', life: 20, isHuman: false }} />);
      act(() => void vi.advanceTimersByTime(3100));
      expect(screen.getByRole('status').textContent).toBe('Thinking… 3s');
      rerender(<PlayerPlate {...props} isMe={false} deciding={false} player={{ name: 'AI Opponent', life: 20, isHuman: false }} />);
      expect(screen.queryByRole('status')).toBeNull();
      act(() => void vi.advanceTimersByTime(1000)); // past the gap between two decisions: a new count
      rerender(<PlayerPlate {...props} isMe={false} deciding player={{ name: 'AI Opponent', life: 20, isHuman: false }} />);
      expect(screen.getByRole('status').textContent).toBe('Thinking… 0s');
    } finally {
      vi.useRealTimers();
    }
  });

  it('never on a person, and the mana line only on your own plate', () => {
    render(<PlayerPlate {...props} isMe={false} deciding player={{ name: 'Friend', life: 20, isHuman: true }} />);
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.queryByText(/mana available/)).toBeNull();
    render(<PlayerPlate {...props} isMe deciding={false} player={{ name: 'Me', life: 20, manaPool: { green: 1 }, battlefield: { a: { id: 'a', name: 'Forest', cardTypes: ['LAND'], subTypes: ['FOREST'], controlled: true } } }} />);
    expect(screen.getByText('2 mana available')).toBeTruthy();
  });
});

describe('player plate commander damage toggle', () => {
  it('starts collapsed in compact multiplayer seats and toggles the breakdown on click without targeting the player', () => {
    const onClick = vi.fn();
    render(
      <PlayerPlate
        player={{ playerId: 'ai1', name: 'AI One', life: 32 }}
        isMe={false}
        compact
        targetable
        selected={false}
        deciding={false}
        onClick={onClick}
        commanderDamage={[
          { name: 'Atraxa, Praetors\' Voice', amount: 8 },
          { name: 'Krenko, Mob Boss', amount: 15 },
        ]}
      />,
    );
    expect(screen.queryByRole('group', { name: 'Commander damage taken' })).toBeNull();
    const toggle = screen.getByRole('button', { name: /AI One, highest commander damage 15/i });
    expect(toggle.getAttribute('aria-expanded')).toBe('false');
    fireEvent.click(toggle);
    expect(onClick).not.toHaveBeenCalled();
    expect(toggle.getAttribute('aria-expanded')).toBe('true');
    expect(screen.getByRole('group', { name: 'Commander damage taken' })).toBeTruthy();
    expect(screen.getByText('Atraxa')).toBeTruthy();
    expect(screen.getByText('Krenko')).toBeTruthy();
    fireEvent.click(toggle);
    expect(screen.queryByRole('group', { name: 'Commander damage taken' })).toBeNull();
  });
});
