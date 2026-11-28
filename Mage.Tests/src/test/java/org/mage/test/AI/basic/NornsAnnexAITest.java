package org.mage.test.AI.basic;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBaseAI;

/**
 * AI must not kill itself paying life for Norn's Annex's {W/P} attack tax,
 * and should prefer paying {W} mana over 2 life when white mana is available (#419).
 */
public class NornsAnnexAITest extends CardTestPlayerBaseAI {

    @Test
    public void test_NornsAnnex_At4LifeTwoAttackers_DoesNotKillItself() {
        // AI (playerA) at 4 life with two 2/2 creatures and no white mana
        setLife(playerA, 4);
        addCard(Zone.BATTLEFIELD, playerA, "Grizzly Bears", 2);

        // Opponent controls Norn's Annex: {W/P} per attacking creature
        addCard(Zone.BATTLEFIELD, playerB, "Norn's Annex", 1);

        setStopAt(1, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        // AI must still be alive (must not pay 4 life to attack with both bears)
        Assert.assertFalse("AI must not kill itself paying life for Norn's Annex", playerA.hasLost());
        Assert.assertTrue("AI must stay above 0 life (was " + playerA.getLife() + ")", playerA.getLife() > 0);
    }

    @Test
    public void test_NornsAnnex_At2LifeNoWhiteMana_DeclinesAttack() {
        setLife(playerA, 2);
        addCard(Zone.BATTLEFIELD, playerA, "Grizzly Bears", 1);
        addCard(Zone.BATTLEFIELD, playerB, "Norn's Annex", 1);

        setStopAt(1, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        Assert.assertFalse("AI must not kill itself at 2 life", playerA.hasLost());
        assertLife(playerA, 2);
        assertLife(playerB, 20);
        assertTapped("Grizzly Bears", false);
    }

    @Test
    public void test_NornsAnnex_PrefersWhiteManaOverLife() {
        setLife(playerA, 4);
        addCard(Zone.BATTLEFIELD, playerA, "Plains", 2);
        addCard(Zone.BATTLEFIELD, playerA, "Grizzly Bears", 2);
        addCard(Zone.BATTLEFIELD, playerB, "Norn's Annex", 1);

        setStopAt(1, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        // AI pays {W}{W} with its two Plains rather than paying 4 lethal life
        Assert.assertFalse("AI must survive by paying white mana", playerA.hasLost());
        assertLife(playerA, 4);
        assertLife(playerB, 16);
        assertTappedCount("Plains", true, 2);
    }
}
