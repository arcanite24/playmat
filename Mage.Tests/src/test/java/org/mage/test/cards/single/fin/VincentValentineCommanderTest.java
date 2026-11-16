package org.mage.test.cards.single.fin;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.counters.CounterType;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestCommander4Players;

/**
 * #295 in a multiplayer Commander game: Vincent Valentine is a commander, cast from the command zone, and an
 * opponent's commander dies three times.
 *
 * @author arcanite24
 */
public class VincentValentineCommanderTest extends CardTestCommander4Players {

    @Test
    public void test_VincentAsCommander_OpponentCommanderDiesThreeTimes() {
        // Player order: A -> D -> C -> B
        // Whenever a creature an opponent controls dies, put a number of +1/+1 counters on Vincent Valentine equal to
        // that creature's power.
        addCard(Zone.COMMAND, playerA, "Vincent Valentine"); // {2}{B}{B} 2/2
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 4);
        addCard(Zone.HAND, playerA, "Lightning Bolt", 3);
        // two Bolts on turn 4, even with Vincent's {2} paid from Mountains
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 4);

        addCard(Zone.COMMAND, playerB, "Daxos of Meletis"); // {1}{W}{U} 2/2
        addCard(Zone.BATTLEFIELD, playerB, "Plains", 6);
        addCard(Zone.BATTLEFIELD, playerB, "Island", 6);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Vincent Valentine");

        castSpell(4, PhaseStep.PRECOMBAT_MAIN, playerB, "Daxos of Meletis");
        castSpell(4, PhaseStep.BEGIN_COMBAT, playerA, "Lightning Bolt", "Daxos of Meletis");
        setChoice(playerB, true); // move to command zone
        castSpell(4, PhaseStep.POSTCOMBAT_MAIN, playerB, "Daxos of Meletis");
        castSpell(4, PhaseStep.END_TURN, playerA, "Lightning Bolt", "Daxos of Meletis");
        setChoice(playerB, true); // move to command zone
        castSpell(8, PhaseStep.PRECOMBAT_MAIN, playerB, "Daxos of Meletis");
        castSpell(8, PhaseStep.BEGIN_COMBAT, playerA, "Lightning Bolt", "Daxos of Meletis");
        setChoice(playerB, true); // move to command zone

        setStrictChooseMode(true);
        setStopAt(8, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertGraveyardCount(playerA, "Lightning Bolt", 3);
        assertCommandZoneCount(playerB, "Daxos of Meletis", 1);
        assertCounterCount(playerA, "Vincent Valentine", CounterType.P1P1, 2 * 3);
    }
}
