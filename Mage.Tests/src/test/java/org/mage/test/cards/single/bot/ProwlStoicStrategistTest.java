package org.mage.test.cards.single.bot;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.counters.CounterType;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * @author arcanite24
 */
public class ProwlStoicStrategistTest extends CardTestPlayerBase {

    /*
     * Prowl, Stoic Strategist {3}{W}
     * Legendary Artifact Creature — Robot
     * More Than Meets the Eye {2}{W}
     * Whenever Prowl attacks, exile up to one other target tapped creature or Vehicle.
     * For as long as that card remains exiled, its owner may play it.
     * Whenever a player plays a card exiled with Prowl, you may draw a card and convert Prowl.
     * 3/3
     *
     * Prowl, Pursuit Vehicle
     * Legendary Artifact — Vehicle
     * Living metal
     * Whenever another creature or Vehicle enters the battlefield under your control, put a +1/+1 counter on Prowl.
     * If this is the second time this ability has resolved this turn, convert Prowl.
     * 2/3
     */
    private static final String prowl = "Prowl, Stoic Strategist";
    private static final String prowlVehicle = "Prowl, Pursuit Vehicle";
    private static final String bears = "Grizzly Bears";
    private static final String memnite = "Memnite"; // {0} 1/1 artifact creature

    @Test
    public void testExileOwnerCastsDrawAndConvert() {
        addCard(Zone.BATTLEFIELD, playerA, prowl);
        addCard(Zone.BATTLEFIELD, playerB, bears, 1, true);
        addCard(Zone.BATTLEFIELD, playerB, "Forest", 2);

        attack(1, playerA, prowl, playerB);
        addTarget(playerA, bears);

        checkExileCount("bears exiled", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, bears, 1);

        // owner plays the exiled card
        castSpell(2, PhaseStep.PRECOMBAT_MAIN, playerB, bears);
        setChoice(playerA, true); // may draw a card and convert

        setStrictChooseMode(true);
        setStopAt(2, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertLife(playerB, 20 - 3);
        assertExileCount(playerB, bears, 0);
        assertPermanentCount(playerB, bears, 1);
        assertHandCount(playerA, 1);
        assertPermanentCount(playerA, prowl, 0);
        assertPermanentCount(playerA, prowlVehicle, 1);
    }

    @Test
    public void testExiledCardNotPlayableByController() {
        addCard(Zone.BATTLEFIELD, playerA, prowl);
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 2);
        addCard(Zone.BATTLEFIELD, playerB, bears, 1, true);

        attack(1, playerA, prowl, playerB);
        addTarget(playerA, bears);

        checkPlayableAbility("controller can't cast", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, "Cast " + bears, false);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertExileCount(playerB, bears, 1);
        assertPermanentCount(playerA, prowl, 1);
    }

    @Test
    public void testBackFaceCountersAndConvertOnSecondResolution() {
        addCard(Zone.BATTLEFIELD, playerA, "Plains", 3);
        addCard(Zone.HAND, playerA, prowl);
        addCard(Zone.HAND, playerA, memnite, 3);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, prowlVehicle + " using More than Meets the Eye");
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);
        checkPermanentCount("converted cast", 1, PhaseStep.PRECOMBAT_MAIN, playerA, prowlVehicle, 1);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, memnite);
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);
        checkPermanentCount("first resolution, no convert", 1, PhaseStep.PRECOMBAT_MAIN, playerA, prowlVehicle, 1);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, memnite);
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);

        // front face has no enters trigger
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, memnite);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertPermanentCount(playerA, memnite, 3);
        assertPermanentCount(playerA, prowlVehicle, 0);
        assertPermanentCount(playerA, prowl, 1);
        assertCounterCount(playerA, prowl, CounterType.P1P1, 2);
        assertPowerToughness(playerA, prowl, 3 + 2, 3 + 2);
    }
}
