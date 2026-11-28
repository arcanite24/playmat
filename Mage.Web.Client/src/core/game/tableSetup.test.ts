import { describe, expect, it } from 'vitest';
import { BUFFER_TIMES, DEFAULT_ADVANCED, describeAdvanced, FREE_MULLIGANS_ALL, matchOptions, TIME_LIMITS } from './tableSetup';

describe('matchOptions', () => {
  const base = { name: 'Friday', gameType: 'Two Player Duel', deckType: 'Constructed - Modern', winsNeeded: 2, playerTypes: ['HUMAN', 'HUMAN'] };

  it('sends the server defaults when nothing changed', () => {
    const options = matchOptions(base);
    expect(options).toMatchObject({ mulliganType: 'GAME_DEFAULT', matchTimeLimit: 'NONE', skillLevel: 'CASUAL', rated: false, spectatorsAllowed: true });
    expect(options.customStartLifeEnabled).toBeUndefined();
    expect(options.attackOption).toBeUndefined();
    expect(options.edhPowerLevel).toBeUndefined();
    expect(options.password).toBeUndefined();
  });

  it('adds multiplayer and commander settings only where they apply', () => {
    const options = matchOptions({
      ...base,
      gameType: 'Commander Free For All',
      deckType: 'Variant Magic - Commander',
      playerTypes: ['HUMAN', 'HUMAN', 'COMPUTER_MAD'],
      advanced: { attackOption: 'LEFT', range: 'ONE', edhPowerLevel: 7, startingLife: 30, handSize: 8 },
    });
    expect(options).toMatchObject({
      attackOption: 'LEFT', range: 'ONE', edhPowerLevel: 7,
      customStartLifeEnabled: true, customStartLife: 30, customStartHandSizeEnabled: true, customStartHandSize: 8,
    });
  });
});

describe('option lists', () => {
  it('spell the server enum names', () => {
    expect(TIME_LIMITS.map((item) => item.value)).toContain('MIN___5');
    expect(TIME_LIMITS.map((item) => item.value)).toContain('MIN_120');
    expect(TIME_LIMITS.map((item) => item.value)).toContain('MIN__45');
    expect(BUFFER_TIMES.map((item) => item.value)).toEqual(['NONE', 'SEC__01', 'SEC__02', 'SEC__03', 'SEC__05', 'SEC__10', 'SEC__15', 'SEC__20', 'SEC__25', 'SEC__30']);
  });

  it('describes only what differs from the defaults', () => {
    expect(describeAdvanced(DEFAULT_ADVANCED)).toEqual([]);
    expect(describeAdvanced({ ...DEFAULT_ADVANCED, matchTimeLimit: 'MIN__20', rated: true, freeMulligans: 1 })).toEqual(['1 free mulligan', '20 minutes', 'Rated']);
    expect(describeAdvanced({ ...DEFAULT_ADVANCED, freeMulligans: FREE_MULLIGANS_ALL })).toEqual(['Free mulligans']);
    expect(matchOptions({ name: 't', gameType: 'g', deckType: 'Constructed - Freeform', winsNeeded: 1, playerTypes: ['HUMAN', 'COMPUTER_MAD'], advanced: { freeMulligans: FREE_MULLIGANS_ALL } }).freeMulligans).toBe(7);
  });
});
