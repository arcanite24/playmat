package org.mage.test.cards.single.ybro;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * Fallaji Antiquarian {3}{R}, Creature — Human Artificer 3/3
 * Haste
 * When Fallaji Antiquarian enters, conjure a duplicate of another target nontoken creature or artifact you control
 * into your graveyard. The duplicate perpetually gains unearth {1}{R}.
 *
 * @author playmat
 */
public class FallajiAntiquarianTest extends CardTestPlayerBase {

    private static final String antiquarian = "Fallaji Antiquarian";
    private static final String bears = "Grizzly Bears";
    private static final String solRing = "Sol Ring";
    private static final String raiseDead = "Raise Dead";
    private static final String swords = "Swords to Plowshares";
    private static final String murder = "Murder";

    @Test
    public void testConjureCreatureDuplicateAndUnearth() {
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 6);
        addCard(Zone.BATTLEFIELD, playerA, bears);
        addCard(Zone.HAND, playerA, antiquarian);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, antiquarian);
        addTarget(playerA, bears);
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);

        // Duplicate is in graveyard and can be unearthed for {1}{R}
        checkGraveyardCount("duplicate in graveyard", 1, PhaseStep.PRECOMBAT_MAIN, playerA, bears, 1);
        activateAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Unearth");
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);

        // Fallaji Antiquarian has haste, and both original and unearthed duplicate are on the battlefield
        checkPermanentCount("two bears on battlefield", 1, PhaseStep.BEGIN_COMBAT, playerA, bears, 2);
        attack(1, playerA, antiquarian);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.CLEANUP);
        execute();

        // Unearthed duplicate exiled at end step; original remains on battlefield
        assertPermanentCount(playerA, antiquarian, 1);
        assertPermanentCount(playerA, bears, 1);
        assertExileCount(playerA, bears, 1);
        assertGraveyardCount(playerA, bears, 0);
        assertLife(playerB, 17);
    }

    @Test
    public void testConjureArtifactDuplicateAndUnearth() {
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 6);
        addCard(Zone.BATTLEFIELD, playerA, solRing);
        addCard(Zone.HAND, playerA, antiquarian);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, antiquarian);
        addTarget(playerA, solRing);
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);

        checkGraveyardCount("artifact duplicate in graveyard", 1, PhaseStep.PRECOMBAT_MAIN, playerA, solRing, 1);
        activateAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Unearth");
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);

        checkPermanentCount("two sol rings on battlefield", 1, PhaseStep.BEGIN_COMBAT, playerA, solRing, 2);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.CLEANUP);
        execute();

        assertPermanentCount(playerA, solRing, 1);
        assertExileCount(playerA, solRing, 1);
    }

    @Test
    public void testUnearthIsPerpetualAcrossZoneChanges() {
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 6);
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 4);
        addCard(Zone.BATTLEFIELD, playerA, "Plains", 4);
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 4);
        addCard(Zone.BATTLEFIELD, playerA, bears);
        addCard(Zone.HAND, playerA, antiquarian);
        addCard(Zone.HAND, playerA, raiseDead);
        addCard(Zone.HAND, playerA, swords);
        addCard(Zone.HAND, playerA, murder);

        // Conjure duplicate of Grizzly Bears into graveyard on turn 1
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, antiquarian);
        addTarget(playerA, bears);
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);

        // Exile original Grizzly Bears so only the conjured duplicate remains
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, swords, bears);
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);

        // On turn 3 (fresh untapped lands): return duplicate from graveyard to hand, cast it, destroy it back to graveyard
        castSpell(3, PhaseStep.PRECOMBAT_MAIN, playerA, raiseDead, bears);
        waitStackResolved(3, PhaseStep.PRECOMBAT_MAIN);
        castSpell(3, PhaseStep.PRECOMBAT_MAIN, playerA, bears);
        waitStackResolved(3, PhaseStep.PRECOMBAT_MAIN);
        castSpell(3, PhaseStep.POSTCOMBAT_MAIN, playerA, murder, bears);
        waitStackResolved(3, PhaseStep.POSTCOMBAT_MAIN);

        // Even after leaving the graveyard -> hand -> battlefield -> graveyard, it STILL has perpetual unearth {1}{R}!
        activateAbility(3, PhaseStep.POSTCOMBAT_MAIN, playerA, "Unearth");
        waitStackResolved(3, PhaseStep.POSTCOMBAT_MAIN);
        checkPermanentCount("unearthed after zone changes", 3, PhaseStep.POSTCOMBAT_MAIN, playerA, bears, 1);

        setStrictChooseMode(true);
        setStopAt(3, PhaseStep.CLEANUP);
        execute();

        // Exiled at end step after unearth (plus the originally exiled Grizzly Bears)
        assertExileCount(playerA, bears, 2);
        assertGraveyardCount(playerA, bears, 0);
    }

    @Test
    public void testCannotTargetSelfOrToken() {
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 6);
        addCard(Zone.HAND, playerA, "Raise the Alarm"); // creates two 1/1 Soldier tokens
        addCard(Zone.BATTLEFIELD, playerA, "Plains", 2);
        addCard(Zone.HAND, playerA, antiquarian);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Raise the Alarm");
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);

        // No legal nontoken target other than itself -> enters with no trigger target
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, antiquarian);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertPermanentCount(playerA, antiquarian, 1);
        assertGraveyardCount(playerA, "Raise the Alarm", 1);
        assertGraveyardCount(playerA, 1); // no conjured duplicate
    }
}
