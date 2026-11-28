package org.mage.test.playmat;

import mage.abilities.Ability;
import mage.abilities.TriggeredAbility;
import mage.cards.Card;
import mage.cards.repository.CardRepository;
import mage.filter.StaticFilters;
import mage.target.TargetPermanent;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * An optional ("you may") trigger chooses its targets before its "may" is asked, so the human player's target prompt
 * is marked ({@code optionalTrigger}) and the web client says the ability can still be declined on resolution.
 * Reported with Qasali Slingers.
 *
 * @author arcanite24
 */
public class OptionalTriggerTargetTest extends CardTestPlayerBase {

    private static TriggeredAbility targetedTrigger(String name) {
        Card card = CardRepository.instance.findCards(name).get(0).createCard();
        for (Ability ability : card.getAbilities()) {
            if (ability instanceof TriggeredAbility && !ability.getTargets().isEmpty()) {
                return (TriggeredAbility) ability;
            }
        }
        throw new AssertionError("no targeted trigger on " + name);
    }

    @Test
    public void test_QasaliSlingers_TargetOfOptionalTrigger() {
        TriggeredAbility ability = targetedTrigger("Qasali Slingers");
        Assert.assertTrue(ability.isOptional());
        Assert.assertTrue(TriggeredAbility.isOptionalTriggerTarget(ability, ability.getTargets().get(0)));
        // a copy (as PlayerImpl.triggerAbility puts it on the stack) marks its own targets
        Ability copy = ability.copy();
        Assert.assertTrue(TriggeredAbility.isOptionalTriggerTarget(copy, copy.getTargets().get(0)));
        // a target chosen by something else while it resolves isn't one of the trigger's
        Assert.assertFalse(TriggeredAbility.isOptionalTriggerTarget(ability,
                new TargetPermanent(StaticFilters.FILTER_PERMANENT_ARTIFACT_OR_ENCHANTMENT)));
    }

    @Test
    public void test_AcidicSlime_MandatoryTriggerNotMarked() {
        TriggeredAbility ability = targetedTrigger("Acidic Slime");
        Assert.assertFalse(ability.isOptional());
        Assert.assertFalse(TriggeredAbility.isOptionalTriggerTarget(ability, ability.getTargets().get(0)));
    }
}
