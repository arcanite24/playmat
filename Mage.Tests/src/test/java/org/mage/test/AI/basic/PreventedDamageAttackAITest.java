package org.mage.test.AI.basic;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBaseAI;

/**
 * AI must not attack with a creature whose combat damage would all be prevented (#199).
 */
public class PreventedDamageAttackAITest extends CardTestPlayerBaseAI {

    @Test
    public void test_DovinPreventsDamage_NoAttack() {
        addCard(Zone.BATTLEFIELD, playerA, "Grizzly Bears", 1);

        // -1: Until your next turn, prevent all damage that would be dealt to and dealt by target permanent an
        // opponent controls.
        addCard(Zone.BATTLEFIELD, playerB, "Dovin, Hand of Control", 1);
        activateAbility(2, PhaseStep.PRECOMBAT_MAIN, playerB, "-1:", "Grizzly Bears");

        setStopAt(3, PhaseStep.END_COMBAT);
        execute();

        // the Bears can't deal damage: attacking would only tap them
        assertTapped("Grizzly Bears", false);
    }

    @Test
    public void test_NoPrevention_Attacks() {
        addCard(Zone.BATTLEFIELD, playerA, "Grizzly Bears", 1);
        addCard(Zone.BATTLEFIELD, playerB, "Dovin, Hand of Control", 1);

        setStopAt(3, PhaseStep.END_COMBAT);
        execute();

        // the Bears go for the planeswalker or the player
        assertTapped("Grizzly Bears", true);
    }
}
