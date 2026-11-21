import type { Interaction, PromptButton } from '../../core/game/interaction';
import type { MessageKey, Translate } from '../i18n';

/**
 * Button labels the server sends word for word, translated by exact match. Anything else the server writes (a card's
 * own choice, the prompt text) stays in English: it is built with card names inside.
 */
const SERVER_LABELS: Record<string, MessageKey> = {
  Yes: 'match.ui.yes',
  No: 'match.ui.no',
  Done: 'match.ui.done',
  Cancel: 'match.ui.cancel',
  Mulligan: 'match.ui.mulligan',
  Keep: 'match.ui.keep',
  'Special payment': 'match.ui.specialPayment',
};

/** A label the client wrote (core/game's UiText), or one of the server's known labels, in the player's language. */
export function labelText(t: Translate, label: string, text?: PromptButton['text']): string {
  if (text) return t(text.key as MessageKey, text.vars);
  const known = SERVER_LABELS[label];
  return known ? t(known) : label;
}

export function buttonText(t: Translate, button: PromptButton): string {
  return labelText(t, button.label, button.text);
}

export function headlineText(t: Translate, interaction: Pick<Interaction, 'headline' | 'headlineText'>): string {
  return interaction.headlineText ? t(interaction.headlineText.key as MessageKey, interaction.headlineText.vars) : interaction.headline;
}
