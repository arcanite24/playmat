import { render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';
import { MatPrint } from './MatPrint';

// stand-ins that say which picture the mat would show
vi.mock('../career/Art', () => ({ Art: ({ id }: { id: string }) => <div data-testid="painting">{id}</div> }));
vi.mock('../ui/MatPrint', () => ({ MatPrint: ({ card }: { card: { name?: string } | null }) => <div data-testid="print">{card?.name ?? 'none'}</div> }));

const cloth = { id: 'mat-tavern', career: true } as never;
const card = { name: 'The Tabernacle at Pendrell Vale' } as never;

describe('MatPrint', () => {
  test('an equipped Career playmat shows its painting over the deck print', () => {
    render(<MatPrint cloth={cloth} card={card} enabled />);
    expect(screen.getByTestId('painting').textContent).toBe('mat-tavern');
  });

  test('a card picked for the mat wins over the painting (#146)', () => {
    render(<MatPrint cloth={cloth} card={card} enabled chosenCard />);
    expect(screen.queryByTestId('painting')).toBeNull();
    expect(screen.getByTestId('print').textContent).toBe('The Tabernacle at Pendrell Vale');
  });

  test('with no card picked yet, the painting stays', () => {
    render(<MatPrint cloth={cloth} card={null} enabled chosenCard />);
    expect(screen.getByTestId('painting')).toBeTruthy();
  });
});
