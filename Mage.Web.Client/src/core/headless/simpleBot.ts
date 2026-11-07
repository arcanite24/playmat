import type { GameView } from '../../protocol/generated/views';
import type { Command, Interaction } from '../game/interaction';

/**
 * A tiny rule-based player for automated tests: keeps its hand, plays lands and spells,
 * attacks with everything, never blocks and answers every other prompt with the first legal option.
 * It only uses the interaction model, so it exercises the same path as a human clicking the board.
 */
export function chooseBotCommand(
  view: GameView | null,
  interaction: Interaction,
  /** spells that could not be paid this turn; the bot won't retry them */
  skip: ReadonlySet<string> = new Set(),
): Command | null {
  const prompt = interaction.prompt;
  if (!prompt) return null;

  switch (interaction.mode) {
    case 'mulligan':
      return { type: 'boolean', value: false }; // keep
    case 'priority': {
      const hand = view?.myHand ?? {};
      const playable = [...interaction.clickable.keys()];
      // lands first, then spells from hand; never activate abilities of permanents (keeps games moving)
      const land = playable.find((id) => hand[id]?.cardTypes?.includes('LAND'));
      if (land) return { type: 'uuid', id: land };
      const spell = playable.find((id) => hand[id] && !hand[id].cardTypes?.includes('LAND') && !skip.has(id));
      if (spell) return { type: 'uuid', id: spell };
      return interaction.mainButton?.command ?? { type: 'boolean', value: false };
    }
    case 'declareAttackers': {
      const allAttack = interaction.secondaryButtons.find((button) => button.tone === 'attack');
      if (allAttack && interaction.selected.size === 0) return allAttack.command;
      return interaction.mainButton?.command ?? null;
    }
    case 'declareBlockers':
      return { type: 'boolean', value: true }; // no blocks
    case 'assignDamage':
      return interaction.mainButton?.command ?? null; // the lethal-first split, as the app's auto-assign sends
    case 'target':
    case 'pickCards': {
      if (prompt.kind !== 'target') return null;
      const unchosen = prompt.targets.find((id) => !interaction.selected.has(id));
      // prefer opponents and their things: a crude stand-in for "target the enemy"
      const opponentTarget = prompt.targets.find((id) =>
        !interaction.selected.has(id) && view?.players?.some((p) => p.playerId === id && p.playerId !== view.myPlayerId));
      if (interaction.selected.size === 0 && (opponentTarget || unchosen)) {
        return { type: 'uuid', id: opponentTarget ?? unchosen! };
      }
      // nothing left to choose (an optional target with no legal one): the app's Cancel
      return interaction.mainButton?.command ?? (unchosen ? { type: 'uuid', id: unchosen } : interaction.secondaryButtons[0]?.command ?? null);
    }
    case 'payMana': {
      // skip sources that already failed to pay this cost (the server can list a source it then ignores)
      const source = [...interaction.clickable.keys()].find((id) => !skip.has(id));
      if (source) return { type: 'uuid', id: source };
      return interaction.secondaryButtons.find((button) => button.label === 'Cancel')?.command ?? null;
    }
    case 'question':
      // the server's "mana will be lost, pass anyway?" confirms a pass the bot already chose; saying no loops forever
      if (/pass anyway/i.test(interaction.headline)) return { type: 'boolean', value: true };
      return { type: 'boolean', value: false }; // decline optional extras like kicker

    case 'panel':
      switch (prompt.kind) {
        case 'chooseAbility':
          return prompt.choices[0] ? { type: 'uuid', id: prompt.choices[0].id } : null;
        case 'choosePile':
          return { type: 'boolean', value: true };
        case 'chooseChoice': {
          const key = Object.keys(prompt.choice.keyChoices ?? {})[0];
          const value = prompt.choice.choices?.[0];
          return { type: 'string', value: key ?? value ?? '' };
        }
        case 'amount':
          return { type: 'integer', value: prompt.min };
        case 'multiAmount':
          return { type: 'string', value: prompt.items.map((item) => String(item.min ?? 0)).join(' ') };
        default:
          return null;
      }
    case 'waiting':
    default:
      return null;
  }
}
