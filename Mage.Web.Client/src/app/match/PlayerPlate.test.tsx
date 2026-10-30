import { act, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
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
