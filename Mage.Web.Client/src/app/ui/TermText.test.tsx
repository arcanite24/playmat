import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { TermText } from './TermText';

describe('TermText', () => {
  afterEach(() => vi.useRealTimers());

  it('explains a marked keyword when it is focused, and hides it again on Escape', () => {
    vi.useFakeTimers();
    render(<TermText text="Flying, lifelink" />);
    const flying = screen.getByText('Flying');
    expect(screen.queryByRole('tooltip')).toBeNull();

    fireEvent.focus(flying);
    act(() => vi.advanceTimersByTime(0));
    const tip = screen.getByRole('tooltip');
    expect(tip.textContent).toMatch(/^Flying/);
    expect(flying.getAttribute('aria-describedby')).toBe(tip.id);

    fireEvent.keyDown(flying, { key: 'Escape' });
    expect(screen.queryByRole('tooltip')).toBeNull();
  });

  it('waits a moment before explaining a keyword under the pointer', () => {
    vi.useFakeTimers();
    render(<TermText text="Flying" />);
    const flying = screen.getByText('Flying');
    fireEvent.pointerEnter(flying);
    expect(screen.queryByRole('tooltip')).toBeNull();
    act(() => vi.advanceTimersByTime(150));
    expect(screen.getByRole('tooltip')).toBeTruthy();
    fireEvent.pointerLeave(flying);
    expect(screen.queryByRole('tooltip')).toBeNull();
  });
});
