import { act, render } from '@testing-library/react';
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
