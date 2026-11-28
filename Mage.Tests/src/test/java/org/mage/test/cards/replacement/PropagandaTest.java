package org.mage.test.cards.replacement;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.players.Player;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 *
 * @author LevelX2
 */
public class PropagandaTest extends CardTestPlayerBase {

    @Test
    public void test_Propaganda1() {
        setStrictChooseMode(true);

        // Creatures can’t attack you unless their controller pays {2} for each creature they control that’s attacking you.
        addCard(Zone.BATTLEFIELD, playerA, "Propaganda");

        addCard(Zone.BATTLEFIELD, playerB, "Silvercoat Lion");
        addCard(Zone.BATTLEFIELD, playerB, "Plains", 2);

        attack(2, playerB, "Silvercoat Lion");
        setChoice(playerB, true);// Pay {2} to attack?

        setStopAt(2, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertLife(playerA, 18);

        assertTappedCount("Silvercoat Lion", true, 1);
        assertTappedCount("Plains", true, 2);
    }

    @Test
    public void test_Propaganda2() {
        setStrictChooseMode(true);

        // Creatures can’t attack you unless their controller pays {2} for each creature they control that’s attacking you.
        addCard(Zone.BATTLEFIELD, playerA, "Propaganda", 2);

        addCard(Zone.BATTLEFIELD, playerB, "Silvercoat Lion");
        addCard(Zone.BATTLEFIELD, playerB, "Plains", 4);

        attack(2, playerB, "Silvercoat Lion");
        setChoice(playerB, "Propaganda"); // Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.");
        setChoice(playerB, true);// Pay {2} to attack?
        setChoice(playerB, true);// Pay {2} to attack?

        setStopAt(2, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertLife(playerA, 18);

        assertTappedCount("Silvercoat Lion", true, 1);
        assertTappedCount("Plains", true, 4);
    }

    /**
     * Paying the tax happens while attackers are declared, when the playable list is normally left empty. The clients
     * only let the player tap what that list shows, so it must still show the mana sources during the payment.
     */
    @Test
    public void test_ManaSourcesPlayableWhilePayingTax() {
        addCard(Zone.BATTLEFIELD, playerA, "Propaganda");
        addCard(Zone.BATTLEFIELD, playerB, "Silvercoat Lion");
        addCard(Zone.BATTLEFIELD, playerB, "Plains", 2);

        setStopAt(2, PhaseStep.DECLARE_ATTACKERS);
        execute();

        currentGame.getPhase().getStep().resumeBeginStep(currentGame, playerB.getId());
        Player player = currentGame.getPlayer(playerB.getId());
        Assert.assertTrue("no playable list while declaring attackers",
                player.getPlayableObjects(currentGame, Zone.ALL).getObjects().isEmpty());

        player.setPayManaMode(true);
        try {
            long plains = currentGame.getBattlefield().getAllActivePermanents(playerB.getId()).stream()
                    .filter(permanent -> permanent.getName().equals("Plains"))
                    .filter(permanent -> player.getPlayableObjects(currentGame, Zone.ALL).containsObject(permanent.getId()))
                    .count();
            Assert.assertEquals("both Plains can pay the tax", 2, plains);
        } finally {
            player.setPayManaMode(false);
        }
    }

}
