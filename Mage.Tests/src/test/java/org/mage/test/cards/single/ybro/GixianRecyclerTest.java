package org.mage.test.cards.single.ybro;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * Gixian Recycler {2}, Artifact Creature 3/1
 * When Gixian Recycler dies or is put into your graveyard from your hand or library, conjure a card named Gixian
 * Recycler into your graveyard.
 * Unearth {1}{B}
 *
 * @author playmat
 */
public class GixianRecyclerTest extends CardTestPlayerBase {

    private static final String recycler = "Gixian Recycler";
    private static final String murder = "Murder"; // {1}{B}{B} instant: destroy target creature
    private static final String glimpse = "Glimpse the Unthinkable"; // {U}{B} sorcery: target player mills ten cards
    private static final String counterspell = "Counterspell";

    @Test
    public void testDies() {
        addCard(Zone.BATTLEFIELD, playerA, recycler);
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 3);
        addCard(Zone.HAND, playerA, murder);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, murder, recycler);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertGraveyardCount(playerA, recycler, 2);
    }

    @Test
    public void testMilled() {
        removeAllCardsFromLibrary(playerA);
        addCard(Zone.LIBRARY, playerA, recycler);
        addCard(Zone.BATTLEFIELD, playerA, "Underground Sea");
        addCard(Zone.BATTLEFIELD, playerA, "Island");
        addCard(Zone.HAND, playerA, glimpse);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, glimpse, playerA);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertGraveyardCount(playerA, recycler, 2);
    }

    @Test
    public void testCounteredDoesNotTrigger() {
        addCard(Zone.BATTLEFIELD, playerA, "Wastes", 2);
        addCard(Zone.HAND, playerA, recycler);
        addCard(Zone.BATTLEFIELD, playerB, "Island", 2);
        addCard(Zone.HAND, playerB, counterspell);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, recycler);
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerB, counterspell, recycler, recycler);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertGraveyardCount(playerA, recycler, 1);
    }

    @Test
    public void testUnearthedIsExiledWithoutConjuring() {
        addCard(Zone.GRAVEYARD, playerA, recycler);
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 5);
        addCard(Zone.HAND, playerA, murder);

        activateAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Unearth");
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, murder, recycler);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertExileCount(playerA, recycler, 1);
        assertGraveyardCount(playerA, recycler, 0);
    }
}
