import { describe, expect, test } from 'vitest';
import { opponentName } from './opponentName';

describe('opponentName', () => {
  test('the face found for the game wins', () => {
    expect(opponentName('Rival 2', 'limited:86e535e4')).toBe('Rival 2');
  });
  test('a roster id the roster no longer lists still reads as a name', () => {
    expect(opponentName(undefined, 'kraul-scout')).toBe('kraul-scout');
  });
  test("a mode's run id is never printed (#145)", () => {
    expect(opponentName(undefined, 'limited:86e535e4-6448-4f07-896c-8a32906fa700')).toBeNull();
    expect(opponentName('', 'gauntlet:week-44')).toBeNull();
  });
});
