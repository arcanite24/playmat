// @vitest-environment happy-dom
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';
import type { CardView } from '../../protocol/generated/views';
import { CardInspect, InspectButton } from './CardInspect';

vi.mock('../ui/CardFace', () => ({ CardFace: ({ card }: { card: CardView }) => <span data-testid="face">{card.name}</span> }));

const card = {
  id: 'c1',
  name: 'Trufflesnout',
  cardTypes: ['CREATURE'],
  subTypes: ['Boar'],
  power: '2',
  toughness: '2',
  rules: ['When Trufflesnout enters, choose one &mdash;<br>&bull Put a +1/+1 counter on it.<br>&bull You gain 4 life.'],
} as unknown as CardView;

describe('CardInspect (#250)', () => {
  test('the magnifier opens the card up close, with its rules as text', () => {
    const onInspect = vi.fn();
    render(<InspectButton card={card} label="Look at Trufflesnout up close" onInspect={onInspect} />);
    fireEvent.click(screen.getByRole('button', { name: 'Look at Trufflesnout up close' }));
    expect(onInspect).toHaveBeenCalledWith(card);
  });

  test('shows the card, its type line and its rules', () => {
    const onClose = vi.fn();
    render(<CardInspect card={card} onClose={onClose} />);
    expect(screen.getByRole('dialog', { name: 'Trufflesnout' })).toBeTruthy();
    expect(screen.getByText('Creature — Boar')).toBeTruthy();
    expect(screen.getByText(/choose one — • Put a \+1\/\+1 counter on it\. • You gain 4 life\./)).toBeTruthy();
  });
});
