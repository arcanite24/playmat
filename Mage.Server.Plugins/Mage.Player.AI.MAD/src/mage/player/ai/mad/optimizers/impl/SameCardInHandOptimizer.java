package mage.player.ai.mad.optimizers.impl;

import mage.abilities.Ability;
import mage.cards.Card;
import mage.constants.Zone;
import mage.game.Game;
import mage.player.ai.mad.optimizers.TreeOptimizer;
import mage.target.Target;

import java.util.HashSet;
import java.util.Iterator;
import java.util.List;
import java.util.Set;
import java.util.UUID;

/**
 * AI: keeps one of the actions that differ only in which copy of a card in hand they use.
 * <p>
 * Two Forests or two Llanowar Elves in hand are the same card, so playing either one leads to the same game. Without
 * this the search plays out each copy's whole subtree, at every depth.
 * Only single-mode abilities of cards in hand, with the same targets and the same mana to pay, count as the same.
 */
public class SameCardInHandOptimizer implements TreeOptimizer {

    @Override
    public void optimize(Game game, List<Ability> actions) {
        Set<String> seen = new HashSet<>();
        // options of one ability are copies sharing its id, so remove by position, not with List.remove(Object)
        for (Iterator<Ability> it = actions.iterator(); it.hasNext(); ) {
            String key = keyOf(game, it.next());
            if (key != null && !seen.add(key)) {
                it.remove();
            }
        }
    }

    private static String keyOf(Game game, Ability ability) {
        if (ability.getModes().size() > 1 || game.getState().getZone(ability.getSourceId()) != Zone.HAND) {
            return null;
        }
        Card card = game.getCard(ability.getSourceId());
        if (card == null) {
            return null;
        }
        StringBuilder key = new StringBuilder()
                .append(ability.getControllerId()).append('|')
                .append(card.getName()).append('|')
                .append(ability.getClass().getName()).append('|')
                .append(ability.getAbilityType()).append('|')
                .append(ability).append('|')
                .append(ability.getManaCostsToPay().getText());
        for (Target target : ability.getTargets()) {
            key.append('|');
            // a divided amount (5 damage among any targets) differs by its split, not only by its targets
            for (UUID targetId : target.getTargets()) {
                key.append(targetId).append('=').append(target.getTargetAmount(targetId)).append(',');
            }
        }
        return key.toString();
    }
}
