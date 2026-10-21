import type { MessageKey, Translate } from '../i18n';
import type { AiOptions, AiType } from '../stores/play';

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

/** One line describing the AI setup, for the Play zone. */
export function describeAi(t: Translate, options: AiOptions, deckName: string | null): string {
  const difficulty = t(difficultyOf(options.skill).label);
  const parts = [
    options.opponents > 1 ? t('ai.summary.many', { count: options.opponents, difficulty }) : t('ai.summary.one', { difficulty }),
    deckName ?? t(options.rules === 'deck' ? 'ai.summary.metaDeck' : 'ai.summary.randomDeck'),
    t(options.winsNeeded === 2 ? 'ai.summary.bestOf3' : 'ai.summary.bestOf1'),
  ];
  if (options.rules === 'deck') parts.push(t('ai.summary.deckFormat'));
  if (options.startingLife !== null) parts.push(t('ai.summary.life', { count: options.startingLife }));
  return parts.join(' · ');
}
