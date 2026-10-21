package org.mage.test.cards.single.msh;

import mage.abilities.keyword.HexproofAbility;
import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.counters.CounterType;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * @author arcanite24
 */
public class KidLokiTest extends CardTestPlayerBase {

    /*
     * Kid Loki {U}
     * Legendary Creature — God Hero Villain
     * Each creature you control that you've put one or more +1/+1 counters on this turn has hexproof.
     * Whenever you draw your second card each turn, put a +1/+1 counter on Kid Loki.
     * 1/1
     */
    private static final String loki = "Kid Loki";
    private static final String divination = "Divination"; // {2}{U} sorcery: draw two cards
    private static final String battlegrowth = "Battlegrowth"; // {G} instant: put a +1/+1 counter on target creature
    private static final String bears = "Grizzly Bears";
    private static final String lion = "Silvercoat Lion";
    private static final String spikeFeeder = "Spike Feeder"; // enters with two +1/+1 counters

    @Test
    public void testCounterOnSecondDraw() {
        addCard(Zone.BATTLEFIELD, playerA, loki);
        addCard(Zone.BATTLEFIELD, playerA, "Island", 3);
        addCard(Zone.HAND, playerA, divination);

        // turn 3: draw step is the first card, Divination draws the second and third
        castSpell(3, PhaseStep.PRECOMBAT_MAIN, playerA, divination);

        setStrictChooseMode(true);
        setStopAt(3, PhaseStep.END_TURN);
        execute();

        assertCounterCount(playerA, loki, CounterType.P1P1, 1);
        assertPowerToughness(playerA, loki, 2, 2);
    }

    @Test
    public void testHexproofForCreatureYouPutCounterOn() {
        addCard(Zone.BATTLEFIELD, playerA, loki);
        addCard(Zone.BATTLEFIELD, playerA, bears);
        addCard(Zone.BATTLEFIELD, playerA, lion);
        addCard(Zone.BATTLEFIELD, playerA, "Forest");
        addCard(Zone.HAND, playerA, battlegrowth);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, battlegrowth, bears);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertCounterCount(playerA, bears, CounterType.P1P1, 1);
        assertAbility(playerA, bears, HexproofAbility.getInstance(), true);
        assertAbility(playerA, lion, HexproofAbility.getInstance(), false);
        assertAbility(playerA, loki, HexproofAbility.getInstance(), false);
    }

    @Test
    public void testHexproofEndsNextTurn() {
        addCard(Zone.BATTLEFIELD, playerA, loki);
        addCard(Zone.BATTLEFIELD, playerA, bears);
        addCard(Zone.BATTLEFIELD, playerA, "Forest");
        addCard(Zone.HAND, playerA, battlegrowth);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, battlegrowth, bears);

        setStrictChooseMode(true);
        setStopAt(2, PhaseStep.PRECOMBAT_MAIN);
        execute();

        assertCounterCount(playerA, bears, CounterType.P1P1, 1);
        assertAbility(playerA, bears, HexproofAbility.getInstance(), false);
    }

    @Test
    public void testOpponentCounterDoesNotGiveHexproof() {
        addCard(Zone.BATTLEFIELD, playerA, loki);
        addCard(Zone.BATTLEFIELD, playerA, bears);
        addCard(Zone.BATTLEFIELD, playerB, "Forest");
        addCard(Zone.HAND, playerB, battlegrowth);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerB, battlegrowth, bears);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertCounterCount(playerA, bears, CounterType.P1P1, 1);
        assertAbility(playerA, bears, HexproofAbility.getInstance(), false);
    }

    @Test
    public void testEntersWithCountersHasHexproof() {
        addCard(Zone.BATTLEFIELD, playerA, loki);
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 3);
        addCard(Zone.HAND, playerA, spikeFeeder);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, spikeFeeder);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertCounterCount(playerA, spikeFeeder, CounterType.P1P1, 2);
        assertAbility(playerA, spikeFeeder, HexproofAbility.getInstance(), true);
    }
}
