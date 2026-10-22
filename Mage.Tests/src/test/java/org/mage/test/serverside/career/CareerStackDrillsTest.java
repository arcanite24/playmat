package org.mage.test.serverside.career;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * The stack drills' answers work in the engine (S021). "Battle cry first": Syr Faren's trigger reads its power when it
 * resolves, so the battle cry has to resolve before it for Signal Pest to hit for three.
 */
public class CareerStackDrillsTest extends CardTestPlayerBase {

    private void attackWithBoth(String putFirst) {
        addCard(Zone.BATTLEFIELD, playerA, "Syr Faren, the Hengehammer", 1);
        addCard(Zone.BATTLEFIELD, playerA, "Signal Pest", 1);
        addCard(Zone.BATTLEFIELD, playerB, "Wall of Wood", 2);

        attack(1, playerA, "Syr Faren, the Hengehammer");
        attack(1, playerA, "Signal Pest");
        // the first trigger chosen goes on the stack first, so it resolves last
        setChoice(playerA, putFirst);
        addTarget(playerA, "Signal Pest");
        block(1, playerB, "Wall of Wood", "Syr Faren, the Hengehammer");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.POSTCOMBAT_MAIN);
        execute();
    }

    @Test
    public void battleCryFirstIsLethal() {
        attackWithBoth("Syr Faren");
        assertLife(playerB, 20 - 3);
    }

    @Test
    public void theOtherOrderFallsShort() {
        attackWithBoth("Signal Pest");
        assertLife(playerB, 20 - 2);
    }
}
