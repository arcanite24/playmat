import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
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
