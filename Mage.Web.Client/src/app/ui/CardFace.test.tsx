import { act, fireEvent, render } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { CardFace } from './CardFace';

const PICTURE = 'https://cards.scryfall.io/normal/front/a/b/ab.jpg?1';

vi.mock('../../core/images/useCardImage', () => ({ useCardImage: () => PICTURE }));

describe('card picture that fails to load', () => {
  afterEach(() => vi.useRealTimers());

  it('tries again after a wait instead of staying blank', () => {
    vi.useFakeTimers();
    const { container } = render(<CardFace card={{ name: 'Island', expansionSetCode: 'M21', cardNumber: '264' }} />);
    const first = container.querySelector('img');
    expect(first?.getAttribute('src')).toBe(PICTURE);

    fireEvent.error(first!);
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('Island');

    act(() => vi.advanceTimersByTime(2_000));
    const second = container.querySelector('img');
    expect(second?.getAttribute('src')).toBe(PICTURE);
    expect(second).not.toBe(first);
  });

  it('gives up after the last retry and keeps the text frame', () => {
    vi.useFakeTimers();
    const { container } = render(<CardFace card={{ name: 'Island', expansionSetCode: 'M21', cardNumber: '264' }} />);
    for (let attempt = 0; attempt < 6; attempt++) {
      fireEvent.error(container.querySelector('img')!);
      act(() => vi.advanceTimersByTime(300_000));
    }
    fireEvent.error(container.querySelector('img')!);
    act(() => vi.advanceTimersByTime(600_000));
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('Island');
  });
});
