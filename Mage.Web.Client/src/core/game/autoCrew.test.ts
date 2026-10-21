import { describe, expect, it } from 'vitest';
import type { PermanentView } from '../../protocol/generated/views';
import { autoCrewPick, crewCandidate, crewNeed, type CrewCandidate } from './autoCrew';

const creature = (id: string, power: number, kind = id): CrewCandidate => ({ id, power, kind });

describe('crewNeed', () => {
  it('reads the power asked for and chosen so far', () => {
    expect(crewNeed('Select another untapped creature you control (selected power 0 of 3)')).toEqual({ selected: 0, need: 3 });
    expect(crewNeed('Select another untapped creature you control (selected power 4 of 3)')).toEqual({ selected: 4, need: 3 });
    expect(crewNeed('Select target creature')).toBeNull();
  });
});

describe('autoCrewPick', () => {
  it('taps the only creature that can crew, or the only group that reaches the power', () => {
    expect(autoCrewPick(3, [creature('a', 3)])).toEqual(['a']);
    expect(autoCrewPick(3, [creature('a', 3), creature('b', 1), creature('c', 1)])).toEqual(['a']);
    expect(autoCrewPick(3, [creature('a', 2), creature('b', 1)])).toEqual(['a', 'b']);
  });

  it('leaves real choices to the player', () => {
    expect(autoCrewPick(1, [creature('a', 2), creature('b', 2)])).toBeNull();
    expect(autoCrewPick(3, [creature('a', 3), creature('b', 2), creature('c', 1)])).toBeNull();
  });

  it('counts interchangeable creatures as one choice', () => {
    expect(autoCrewPick(1, [creature('t1', 1, 'Soldier'), creature('t2', 1, 'Soldier')])).toEqual(['t1']);
    expect(autoCrewPick(2, [creature('t1', 1, 'Soldier'), creature('t2', 1, 'Soldier'), creature('t3', 1, 'Soldier')])).toEqual(['t1', 't2']);
  });

  it('gives up when nothing reaches the power or creatures have no power', () => {
    expect(autoCrewPick(4, [creature('a', 1), creature('b', 2)])).toBeNull();
    expect(autoCrewPick(1, [creature('a', 0)])).toBeNull();
    expect(autoCrewPick(1, [creature('a', 0), creature('b', 1)])).toEqual(['b']);
  });
});

describe('crewCandidate', () => {
  it('skips creatures that crew by other rules than power', () => {
    const odric = { id: 'o', name: 'Odric', power: '3', toughness: '3', rules: ['Odric crews Vehicles as though its power were 2 greater.'] } as PermanentView;
    expect(crewCandidate(odric)).toBeNull();
    const bear = { id: 'b', name: 'Bear', power: '2', toughness: '2', rules: [] } as unknown as PermanentView;
    expect(crewCandidate(bear)).toMatchObject({ id: 'b', power: 2 });
  });
});
