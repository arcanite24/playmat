package org.mage.test.AI.basic;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBaseAI;

/**
 * AI must never counter its own spells (#224).
 */
public class CounterOwnSpellAITest extends CardTestPlayerBaseAI {

    @Test
    public void test_ForceOfWill_NotOnOwnSpell() {
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 2);
        addCard(Zone.HAND, playerA, "Grizzly Bears", 1);
        addCard(Zone.HAND, playerA, "Force of Will", 2);

        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertPermanentCount(playerA, "Grizzly Bears", 1);
        assertHandCount(playerA, "Force of Will", 2);
        assertLife(playerA, 20);
    }

    @Test
    public void test_Counterspell_OnOpponentCounter() {
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 2);
        addCard(Zone.BATTLEFIELD, playerA, "Island", 4);
        addCard(Zone.HAND, playerA, "Grizzly Bears", 1);
        addCard(Zone.HAND, playerA, "Counterspell", 1);

        addCard(Zone.BATTLEFIELD, playerB, "Island", 2);
        addCard(Zone.HAND, playerB, "Counterspell", 1);
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerB, "Counterspell", "Grizzly Bears", "Grizzly Bears");

        setStopAt(1, PhaseStep.END_TURN);
        execute();

        // the AI fights for its Bears, never counters them
        assertGraveyardCount(playerB, "Counterspell", 1);
        assertPermanentCount(playerA, "Grizzly Bears", 1);
    }
}
