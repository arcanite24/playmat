package org.mage.test.cards.single.ydmu;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * Diminished Returner {1}{B}{B}, Creature 4/3
 * Diminished Returner enters tapped.
 * {B}{B}: Diminished Returner perpetually gets -1/-1, then return it to the battlefield. Activate only if Diminished
 * Returner is in your graveyard and its toughness is 2 or greater.
 *
 * @author playmat
 */
public class DiminishedReturnerTest extends CardTestPlayerBase {

    private static final String returner = "Diminished Returner";
    private static final String murder = "Murder"; // {1}{B}{B} instant: destroy target creature
    private static final String unsummon = "Unsummon"; // {U} instant: return target creature to its owner's hand

    @Test
    public void testReturnsSmallerEachTime() {
        addCard(Zone.BATTLEFIELD, playerA, returner);
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 13);
        addCard(Zone.HAND, playerA, murder, 3);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, murder, returner);
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);
        activateAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "{B}{B}:");
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);
        checkPT("first return", 1, PhaseStep.PRECOMBAT_MAIN, playerA, returner, 3, 2);

        castSpell(1, PhaseStep.POSTCOMBAT_MAIN, playerA, murder, returner);
        waitStackResolved(1, PhaseStep.POSTCOMBAT_MAIN);
        activateAbility(1, PhaseStep.POSTCOMBAT_MAIN, playerA, "{B}{B}:");
        waitStackResolved(1, PhaseStep.POSTCOMBAT_MAIN);

        // the -1/-1 stays with the card: now 2/1, and with toughness 1 in the graveyard it can't come back again
        castSpell(1, PhaseStep.END_TURN, playerA, murder, returner);
        waitStackResolved(1, PhaseStep.END_TURN);
        checkPlayableAbility("toughness 1", 1, PhaseStep.END_TURN, playerA, "{B}{B}:", false);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.CLEANUP);
        execute();

        assertGraveyardCount(playerA, returner, 1);
        assertGraveyardCount(playerA, murder, 3);
    }

    @Test
    public void testEntersTappedAndKeepsTheChangeInHand() {
        addCard(Zone.BATTLEFIELD, playerA, returner);
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 8);
        addCard(Zone.BATTLEFIELD, playerA, "Island", 2); // two: auto-pay may spend one on Murder's generic
        addCard(Zone.HAND, playerA, murder);
        addCard(Zone.HAND, playerA, unsummon);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, murder, returner);
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);
        activateAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "{B}{B}:");
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);
        checkPermanentTapped("returns tapped", 1, PhaseStep.PRECOMBAT_MAIN, playerA, returner, true, 1);

        // back to hand and cast again: still 3/2
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, unsummon, returner);
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);
        castSpell(1, PhaseStep.POSTCOMBAT_MAIN, playerA, returner);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertPowerToughness(playerA, returner, 3, 2);
        assertTapped(returner, true);
    }

    @Test
    public void testNotFromBattlefield() {
        addCard(Zone.BATTLEFIELD, playerA, returner);
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 2);

        checkPlayableAbility("on the battlefield", 1, PhaseStep.PRECOMBAT_MAIN, playerA, "{B}{B}:", false);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertPowerToughness(playerA, returner, 4, 3);
    }
}
