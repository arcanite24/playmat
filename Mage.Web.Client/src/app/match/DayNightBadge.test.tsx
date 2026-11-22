// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, test } from 'vitest';
import { explainCard } from '../../core/game/glossary';
import type { CardView } from '../../protocol/generated/views';
import { DayNightBadge } from './DayNightBadge';

afterEach(cleanup);

describe('DayNightBadge (#407)', () => {
  test('renders nothing when neither day nor night is active', () => {
    const { container } = render(<DayNightBadge daytime={undefined} />);
    expect(container.firstChild).toBeNull();
  });

  test('shows Day / Night state, explains transition rules on click, and announces transitions', () => {
    const { rerender } = render(<DayNightBadge daytime={true} />);
    const dayButton = screen.getByRole('button', { name: /currently Day/i });
    expect(dayButton.textContent).toContain('Day');

    fireEvent.click(dayButton);
    expect(screen.getByRole('tooltip').textContent).toContain('no spells');
    expect(screen.getByRole('tooltip').textContent).toContain('2 or more spells');

    rerender(<DayNightBadge daytime={false} />);
    expect(screen.getByRole('button', { name: /currently Night/i }).textContent).toContain('Night');
    expect(screen.getByText('It is now Night.')).toBeTruthy();
  });

  test('glossary explains Day and Night on cards like Vadrik that do not have Daybound keyword', () => {
    const vadrik = {
      name: 'Vadrik, Astral Archmage',
      rules: [
        "If it's neither day nor night, it becomes day as Vadrik, Astral Archmage enters.",
        'Whenever day becomes night or night becomes day, put a +1/+1 counter on Vadrik.',
      ],
    } as CardView;
    const explained = explainCard(vadrik);
    const term = explained.terms.find((item) => item.name === 'Day and Night');
    expect(term).toBeDefined();
    expect(term?.text).toContain('no spells');
    expect(term?.text).toContain('two or more spells');
  });
});
