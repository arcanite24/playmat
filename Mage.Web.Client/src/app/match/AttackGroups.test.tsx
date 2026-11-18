import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { AttackGroup } from '../../core/game/attackGroups';
import { AttackGroups } from './AttackGroups';

const ids = Array.from({ length: 14 }, (_, i) => `t${i}`);
const group: AttackGroup = {
  key: 'Soldier|1|1|token', name: 'Soldier', power: '1', toughness: '1', isToken: true,
  lead: { id: 't0', name: 'Soldier' }, ready: ids, attacking: [],
};

function setup() {
  const onWant = vi.fn();
  render(<AttackGroups groups={[group]} wanted={new Map()} busy={false} remaining={0} onWant={onWant} onStop={() => undefined} />);
  const [, number] = screen.getAllByLabelText(/How many Soldier 1\/1 attack/);
  return { onWant, number: number as HTMLInputElement };
}

describe('the attack-with panel', () => {
  it('asks for a typed count once it is entered, not digit by digit', () => {
    const { onWant, number } = setup();
    fireEvent.change(number, { target: { value: '1' } });
    fireEvent.change(number, { target: { value: '12' } });
    expect(onWant).not.toHaveBeenCalled();
    expect(number.value).toBe('12');
    fireEvent.keyDown(number, { key: 'Enter' });
    expect(onWant).toHaveBeenCalledTimes(1);
    expect(onWant).toHaveBeenCalledWith(group, 12);
  });

  it('enters a typed count when the field is left, and caps it at the group', () => {
    const { onWant, number } = setup();
    fireEvent.change(number, { target: { value: '40' } });
    fireEvent.blur(number);
    expect(onWant).toHaveBeenCalledWith(group, 14);
  });

  it('drops a typed count on Escape', () => {
    const { onWant, number } = setup();
    fireEvent.change(number, { target: { value: '7' } });
    fireEvent.keyDown(number, { key: 'Escape' });
    expect(number.value).toBe('0');
    fireEvent.blur(number);
    expect(onWant).not.toHaveBeenCalled();
  });
});
