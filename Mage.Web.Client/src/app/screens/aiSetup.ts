import type { MessageKey, Translate } from '../i18n';
import type { AiOptions, AiType } from '../stores/play';
import { opponentSource } from '../solo/freePlay';

/** The server AI's skill (1-10) behind each difficulty. */
export const DIFFICULTIES: { label: MessageKey; skill: number; detail: MessageKey }[] = [
  { label: 'ai.difficulty.easy', skill: 1, detail: 'ai.difficulty.easy.detail' },
  { label: 'ai.difficulty.normal', skill: 4, detail: 'ai.difficulty.normal.detail' },
  { label: 'ai.difficulty.hard', skill: 7, detail: 'ai.difficulty.hard.detail' },
  { label: 'ai.difficulty.expert', skill: 10, detail: 'ai.difficulty.expert.detail' },
];

export const AI_TYPES: { value: AiType; label: MessageKey; detail: MessageKey }[] = [
  { value: 'COMPUTER_MAD', label: 'ai.type.standard', detail: 'ai.type.standard.detail' },
  { value: 'COMPUTER_MONTE_CARLO', label: 'ai.type.monteCarlo', detail: 'ai.type.monteCarlo.detail' },
];

/** The AI types this server offers (a server can leave the Monte Carlo AI out); all of them until it has said. */
export function offeredAiTypes(playerTypes: readonly string[] | undefined) {
  return playerTypes ? AI_TYPES.filter((item) => playerTypes.includes(item.value)) : AI_TYPES;
}

/** The AI type to seat: the chosen one if the server offers it, else the standard AI. */
export function seatableAiType(aiType: AiType, playerTypes: readonly string[] | undefined): AiType {
  return !playerTypes || playerTypes.includes(aiType) ? aiType : 'COMPUTER_MAD';
}

export function difficultyOf(skill: number) {
  return DIFFICULTIES.reduce((best, item) => (Math.abs(item.skill - skill) < Math.abs(best.skill - skill) ? item : best));
}

/**
 * The AI's deck in the setup summary: a roster deck by its name, a meta or ghost deck by the name kept with it, the
 * mirror as such. Null for a random starter, and for the solo sources until their messages (`soloReady`) are in.
 */
export function aiDeckLabel(t: Translate, options: AiOptions, rosterName: string | null, soloReady: boolean): string | null {
  const source = opponentSource(options.opponentDeckId);
  switch (source.kind) {
    case 'roster': return rosterName;
    case 'meta': return options.opponentLabel;
    case 'mirror': return soloReady ? t('solo.mirror.summary') : null;
    case 'ghost': return soloReady ? t('solo.ghost.summary', { name: options.opponentLabel ?? source.code }) : null;
  }
}

/** One line describing the AI setup, for the Play zone; `adaptiveLabel` stands in for the difficulty when adaptive. */
export function describeAi(t: Translate, options: AiOptions, deckName: string | null, adaptiveLabel?: string): string {
  const difficulty = options.adaptive && adaptiveLabel ? adaptiveLabel : t(difficultyOf(options.skill).label);
  const parts = [
    options.opponents > 1 ? t('ai.summary.many', { count: options.opponents, difficulty }) : t('ai.summary.one', { difficulty }),
    deckName ?? t('ai.summary.randomDeck'),
    t(options.winsNeeded === 2 ? 'ai.summary.bestOf3' : 'ai.summary.bestOf1'),
  ];
  if (options.rules === 'deck') parts.push(t('ai.summary.deckFormat'));
  if (options.startingLife !== null) parts.push(t('ai.summary.life', { count: options.startingLife }));
  return parts.join(' · ');
}
