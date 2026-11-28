package org.mage.test.cards.single.blc;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * @author arcanite24
 */
public class ZinniaValleysVoiceTest extends CardTestPlayerBase {

    /**
     * {@link mage.cards.z.ZinniaValleysVoice Zinnia, Valley's Voice} {U}{R}{W}
     * Legendary Creature — Bird Bard 1/3
     * Flying
     * Zinnia, Valley's Voice gets +X/+0, where X is the number of other creatures you control with base power 1.
     * Creature spells you cast have offspring {2}.
     */
    private static final String zinnia = "Zinnia, Valley's Voice";

    @Test
    public void test_OffspringPaid() {
        addCard(Zone.BATTLEFIELD, playerA, zinnia);
        addCard(Zone.BATTLEFIELD, playerA, "Plains", 4);
        addCard(Zone.HAND, playerA, "Grizzly Bears");
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 1);

        setChoice(playerA, true); // pay offspring
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Grizzly Bears");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertPermanentCount(playerA, "Grizzly Bears", 2);
        assertTokenCount(playerA, "Grizzly Bears", 1);
        // the 1/1 token has base power 1
        assertPowerToughness(playerA, zinnia, 1 + 1, 3);
    }

    @Test
    public void test_OffspringNotPaid() {
        addCard(Zone.BATTLEFIELD, playerA, zinnia);
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 4);
        addCard(Zone.HAND, playerA, "Grizzly Bears");

        setChoice(playerA, false); // don't pay offspring
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Grizzly Bears");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertPermanentCount(playerA, "Grizzly Bears", 1);
        assertTokenCount(playerA, "Grizzly Bears", 0);
        assertPowerToughness(playerA, zinnia, 1, 3);
    }

    @Test
    public void test_PrintedOffspringCopiesOnce() {
        addCard(Zone.BATTLEFIELD, playerA, zinnia);
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 3);
        addCard(Zone.HAND, playerA, "Iridescent Vinelasher"); // {B} 1/2, offspring {2}

        setChoice(playerA, true); // pay its own offspring (Zinnia grants no second one)
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Iridescent Vinelasher");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertPermanentCount(playerA, "Iridescent Vinelasher", 2);
        assertTokenCount(playerA, "Iridescent Vinelasher", 1);
    }

    @Test
    public void test_NoCopyAfterZinniaLeaves() {
        addCard(Zone.BATTLEFIELD, playerA, zinnia);
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 4);
        addCard(Zone.HAND, playerA, "Grizzly Bears");
        addCard(Zone.HAND, playerB, "Lightning Bolt");
        addCard(Zone.BATTLEFIELD, playerB, "Mountain");

        setChoice(playerA, true); // pay offspring
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Grizzly Bears");
        // Zinnia dies with the creature spell still on the stack
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerB, "Lightning Bolt", zinnia, "Grizzly Bears");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertGraveyardCount(playerA, zinnia, 1);
        assertPermanentCount(playerA, "Grizzly Bears", 1);
        assertTokenCount(playerA, "Grizzly Bears", 0);
    }

    @Test
    public void test_BoostCountsOtherBasePowerOne() {
        addCard(Zone.BATTLEFIELD, playerA, zinnia);
        addCard(Zone.BATTLEFIELD, playerA, "Memnite", 2); // 1/1
        addCard(Zone.BATTLEFIELD, playerA, "Grizzly Bears"); // 2/2, not counted
        addCard(Zone.BATTLEFIELD, playerB, "Memnite"); // opponent's, not counted

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.PRECOMBAT_MAIN);
        execute();

        assertPowerToughness(playerA, zinnia, 1 + 2, 3);
    }
}
