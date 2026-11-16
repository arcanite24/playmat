package org.mage.test.cards.single.afr;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * @author arcanite24
 */
public class DragonsFireTest extends CardTestPlayerBase {

    // As an additional cost to cast this spell, you may reveal a Dragon card from your hand or choose a Dragon you
    // control.
    // Dragon's Fire deals 3 damage to target creature or planeswalker. If you revealed a Dragon card or chose a Dragon
    // as you cast this spell, Dragon's Fire deals damage equal to the power of that card or creature instead.
    private static final String fire = "Dragon's Fire";

    /**
     * #384: the reveal is optional, so with no Dragon at all the spell is cast and deals 3
     */
    @Test
    public void test_NoDragon() {
        addCard(Zone.HAND, playerA, fire);
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 2);
        addCard(Zone.BATTLEFIELD, playerB, "Hill Giant"); // 3/3

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, fire, "Hill Giant");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertGraveyardCount(playerA, fire, 1);
        assertGraveyardCount(playerB, "Hill Giant", 1);
    }

    /**
     * a Dragon to reveal, but the player declines: 3 damage
     */
    @Test
    public void test_DragonNotRevealed() {
        addCard(Zone.HAND, playerA, fire);
        addCard(Zone.HAND, playerA, "Shivan Dragon"); // 5/5
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 2);
        addCard(Zone.BATTLEFIELD, playerB, "Hill Giant"); // 3/3

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, fire, "Hill Giant");
        setChoice(playerA, false); // Reveal a Dragon card from your hand?

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertGraveyardCount(playerA, fire, 1);
        assertGraveyardCount(playerB, "Hill Giant", 1);
    }

    @Test
    public void test_DragonRevealed() {
        addCard(Zone.HAND, playerA, fire);
        addCard(Zone.HAND, playerA, "Shivan Dragon"); // 5/5
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 2);
        addCard(Zone.BATTLEFIELD, playerB, "Craw Wurm"); // 6/4

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, fire, "Craw Wurm");
        setChoice(playerA, true); // Reveal a Dragon card from your hand?
        setChoice(playerA, "Shivan Dragon");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertGraveyardCount(playerB, "Craw Wurm", 1);
    }
}
