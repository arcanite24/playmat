package org.mage.test.cards.single.fin;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.counters.CounterType;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestCommanderDuelBase;

/**
 * @author arcanite24
 */
public class VincentValentineTest extends CardTestCommanderDuelBase {

    /**
     * #295: an opponent's commander killed three times, put into the command zone and cast again each time, must
     * trigger Vincent Valentine each time it dies.
     */
    @Test
    public void test_OpponentCommanderDiesThreeTimes() {
        // Whenever a creature an opponent controls dies, put a number of +1/+1 counters on Vincent Valentine equal to
        // that creature's power.
        addCard(Zone.BATTLEFIELD, playerA, "Vincent Valentine"); // 2/2
        addCard(Zone.HAND, playerA, "Lightning Bolt", 3);
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 3);

        addCard(Zone.COMMAND, playerB, "Daxos of Meletis"); // {1}{W}{U} 2/2
        addCard(Zone.BATTLEFIELD, playerB, "Plains", 6);
        addCard(Zone.BATTLEFIELD, playerB, "Island", 6);

        // first: cast for {3}
        castSpell(2, PhaseStep.PRECOMBAT_MAIN, playerB, "Daxos of Meletis");
        castSpell(2, PhaseStep.BEGIN_COMBAT, playerA, "Lightning Bolt", "Daxos of Meletis");
        setChoice(playerB, true); // move to command zone
        // second: {5}
        castSpell(2, PhaseStep.POSTCOMBAT_MAIN, playerB, "Daxos of Meletis");
        castSpell(2, PhaseStep.END_TURN, playerA, "Lightning Bolt", "Daxos of Meletis");
        setChoice(playerB, true); // move to command zone
        // third: {7}
        castSpell(4, PhaseStep.PRECOMBAT_MAIN, playerB, "Daxos of Meletis");
        castSpell(4, PhaseStep.BEGIN_COMBAT, playerA, "Lightning Bolt", "Daxos of Meletis");
        setChoice(playerB, true); // move to command zone

        setStrictChooseMode(true);
        setStopAt(4, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertGraveyardCount(playerA, "Lightning Bolt", 3);
        assertCommandZoneCount(playerB, "Daxos of Meletis", 1);
        assertCounterCount(playerA, "Vincent Valentine", CounterType.P1P1, 2 * 3);
    }
}
