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

    @Test
    public void test_VincentTransformsToGalianBeast_DiesAndReturns_StillTriggers() {
        addCard(Zone.COMMAND, playerA, "Vincent Valentine"); // {2}{B}{B} 2/2
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 4);
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 4);
        addCard(Zone.HAND, playerA, "Lightning Bolt", 3);

        addCard(Zone.COMMAND, playerB, "Daxos of Meletis"); // {1}{W}{U} 2/2
        addCard(Zone.BATTLEFIELD, playerB, "Plains", 6);
        addCard(Zone.BATTLEFIELD, playerB, "Island", 6);

        // Turn 1: Cast Vincent Valentine
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Vincent Valentine");

        // Turn 2: Opponent casts commander, it dies #1 -> Vincent gets 2 counters
        castSpell(2, PhaseStep.PRECOMBAT_MAIN, playerB, "Daxos of Meletis");
        castSpell(2, PhaseStep.BEGIN_COMBAT, playerA, "Lightning Bolt", "Daxos of Meletis");
        setChoice(playerB, true); // Daxos to command zone

        // Turn 3: Vincent attacks, transforms into Galian Beast (3/2 + 2 counters = 5/4)
        attack(3, playerA, "Vincent Valentine");
        setChoice(playerA, true); // transform into Galian Beast
        // Kill Galian Beast in postcombat main with two Lightning Bolts (6 damage >= 4 toughness)
        castSpell(3, PhaseStep.POSTCOMBAT_MAIN, playerA, "Lightning Bolt", "Galian Beast");
        waitStackResolved(3, PhaseStep.POSTCOMBAT_MAIN);
        castSpell(3, PhaseStep.POSTCOMBAT_MAIN, playerA, "Lightning Bolt", "Galian Beast");
        setChoice(playerA, false); // Leave in graveyard so Galian Beast trigger returns it tapped as Vincent Valentine!

        // Turn 4: Opponent casts commander again, we kill it in combat or with Murder -> Vincent should trigger!
        addCard(Zone.HAND, playerA, "Murder");
        castSpell(4, PhaseStep.PRECOMBAT_MAIN, playerB, "Daxos of Meletis");
        castSpell(4, PhaseStep.BEGIN_COMBAT, playerA, "Murder", "Daxos of Meletis");
        setChoice(playerB, true); // Daxos to command zone

        setStrictChooseMode(true);
        setStopAt(4, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertPermanentCount(playerA, "Vincent Valentine", 1);
        assertCommandZoneCount(playerB, "Daxos of Meletis", 1);
        assertCounterCount(playerA, "Vincent Valentine", CounterType.P1P1, 2);
    }

    @Test
    public void test_VincentRecastFromCommandZone_StillTriggers() {
        addCard(Zone.COMMAND, playerA, "Vincent Valentine"); // {2}{B}{B} 2/2
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 10);
        addCard(Zone.HAND, playerA, "Murder", 3);

        addCard(Zone.COMMAND, playerB, "Daxos of Meletis"); // {1}{W}{U} 2/2
        addCard(Zone.BATTLEFIELD, playerB, "Plains", 6);
        addCard(Zone.BATTLEFIELD, playerB, "Island", 6);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Vincent Valentine");

        // Daxos dies #1
        castSpell(2, PhaseStep.PRECOMBAT_MAIN, playerB, "Daxos of Meletis");
        castSpell(2, PhaseStep.BEGIN_COMBAT, playerA, "Murder", "Daxos of Meletis");
        setChoice(playerB, true); // Daxos to command zone

        // Vincent dies -> moves to command zone -> recast on turn 3
        castSpell(2, PhaseStep.END_TURN, playerA, "Murder", "Vincent Valentine");
        setChoice(playerA, true); // Vincent to command zone
        castSpell(3, PhaseStep.PRECOMBAT_MAIN, playerA, "Vincent Valentine");

        // Daxos cast again and dies #2 -> Vincent must trigger!
        castSpell(4, PhaseStep.PRECOMBAT_MAIN, playerB, "Daxos of Meletis");
        castSpell(4, PhaseStep.BEGIN_COMBAT, playerA, "Murder", "Daxos of Meletis");
        setChoice(playerB, true); // Daxos to command zone

        setStrictChooseMode(true);
        setStopAt(4, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertPermanentCount(playerA, "Vincent Valentine", 1);
        assertCommandZoneCount(playerB, "Daxos of Meletis", 1);
        assertCounterCount(playerA, "Vincent Valentine", CounterType.P1P1, 2);
    }

    @Test
    public void test_OpponentMdfcCommanderDiesThreeTimes() {
        addCard(Zone.BATTLEFIELD, playerA, "Vincent Valentine");
        addCard(Zone.HAND, playerA, "Murder", 3);
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 9);

        addCard(Zone.COMMAND, playerB, "Esika, God of the Tree"); // {1}{G}{G} 1/4 MDFC
        addCard(Zone.BATTLEFIELD, playerB, "Forest", 12);

        castSpell(2, PhaseStep.PRECOMBAT_MAIN, playerB, "Esika, God of the Tree");
        castSpell(2, PhaseStep.BEGIN_COMBAT, playerA, "Murder", "Esika, God of the Tree");
        setChoice(playerB, true); // move to command zone

        castSpell(2, PhaseStep.POSTCOMBAT_MAIN, playerB, "Esika, God of the Tree");
        castSpell(2, PhaseStep.END_TURN, playerA, "Murder", "Esika, God of the Tree");
        setChoice(playerB, true); // move to command zone

        castSpell(4, PhaseStep.PRECOMBAT_MAIN, playerB, "Esika, God of the Tree");
        castSpell(4, PhaseStep.BEGIN_COMBAT, playerA, "Murder", "Esika, God of the Tree");
        setChoice(playerB, true); // move to command zone

        setStrictChooseMode(true);
        setStopAt(4, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertCommandZoneCount(playerB, "Esika, God of the Tree", 1);
        assertCounterCount(playerA, "Vincent Valentine", CounterType.P1P1, 1 * 3);
    }
}
