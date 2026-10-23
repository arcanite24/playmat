import { describe, expect, it } from 'vitest';
import type { Translate } from '../i18n';
import core from '../i18n/en/core';
import es from '../i18n/es';
import { createTranslator } from '../i18n/translate';
import { DEFAULT_AI_OPTIONS, type AiOptions } from '../stores/play';
import { aiDeckLabel, describeAi, offeredAiTypes, seatableAiType } from './aiSetup';

describe('AI types a server offers', () => {
  it('lists every type until the server has said, then only the ones it has', () => {
    expect(offeredAiTypes(undefined).map((item) => item.value)).toEqual(['COMPUTER_MAD', 'COMPUTER_MONTE_CARLO']);
    expect(offeredAiTypes(['HUMAN', 'COMPUTER_MAD']).map((item) => item.value)).toEqual(['COMPUTER_MAD']);
  });

  it('seats the standard AI when the chosen one is not offered', () => {
    expect(seatableAiType('COMPUTER_MONTE_CARLO', ['HUMAN', 'COMPUTER_MAD'])).toBe('COMPUTER_MAD');
    expect(seatableAiType('COMPUTER_MONTE_CARLO', undefined)).toBe('COMPUTER_MONTE_CARLO');
    expect(seatableAiType('COMPUTER_MONTE_CARLO', ['COMPUTER_MONTE_CARLO'])).toBe('COMPUTER_MONTE_CARLO');
  });
});

describe("the AI's deck in the setup summary", () => {
  const t = ((key: string, vars?: Record<string, unknown>) => `${key}${vars ? JSON.stringify(vars) : ''}`) as Translate;
  const options = (opponentDeckId: string | null, opponentLabel: string | null = null): AiOptions => ({ ...DEFAULT_AI_OPTIONS, opponentDeckId, opponentLabel });

  it.each([
    { locale: 'en', messages: core, label: "a deck from the format's pool" },
    { locale: 'es', messages: es, label: 'un mazo del formato' },
  ])('describes an unnamed format deck as a pool pick in $locale', ({ locale, messages, label }) => {
    const translate = createTranslator({ locale, messages, fallback: core });
    expect(describeAi(translate, { ...DEFAULT_AI_OPTIONS, rules: 'deck' }, null).split(' · ')).toContain(label);
  });

  it('names roster and format decks, ghosts and the mirror', () => {
    expect(aiDeckLabel(t, options('deck-1'), 'Mono Red', true)).toBe('Mono Red');
    expect(aiDeckLabel(t, options(null), null, true)).toBeNull();
    expect(aiDeckLabel(t, options('meta:modern-affinity.dck'), 'Affinity', true)).toBe('Affinity');
    expect(aiDeckLabel(t, options('mirror'), null, true)).toBe('solo.mirror.summary');
    expect(aiDeckLabel(t, options('ghost:ABC234', 'Elves'), null, true)).toBe('solo.ghost.summary{"name":"Elves"}');
  });

  it('waits for the solo messages before naming a ghost or the mirror', () => {
    expect(aiDeckLabel(t, options('mirror'), null, false)).toBeNull();
    expect(aiDeckLabel(t, options('ghost:ABC234'), null, false)).toBeNull();
  });
});
