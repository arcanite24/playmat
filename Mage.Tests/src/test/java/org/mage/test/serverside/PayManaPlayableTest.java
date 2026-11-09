package org.mage.test.serverside;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.game.permanent.Permanent;
import mage.view.GameView;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * A player asked to pay mana while another player holds priority (Propaganda's or Archon of Absolution's attack tax
 * during declare attackers) must be shown their own mana sources as playable, or a client that only lets the player
 * click what the server lists can't pay at all.
 *
 * @author arcanite24
 */
public class PayManaPlayableTest extends CardTestPlayerBase {

    @Test
    public void test_PayingPlayerWithoutPrioritySeesOwnManaSources() {
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 2);

        // playerB holds priority on playerA's turn
        runCode("check playable", 1, PhaseStep.PRECOMBAT_MAIN, playerB, (info, player, game) -> {
            Permanent forest = game.getBattlefield().getAllActivePermanents(playerA.getId()).stream()
                    .filter(permanent -> permanent.getName().equals("Forest"))
                    .findFirst()
                    .orElse(null);
            Assert.assertNotNull(forest);
            Assert.assertEquals(playerB.getId(), game.getPriorityPlayerId());

            GameView idle = getGameView(playerA);
            Assert.assertFalse("without priority or a payment, nothing is playable",
                    idle.getCanPlayObjects() != null && idle.getCanPlayObjects().containsObject(forest.getId()));

            playerA.setPayManaMode(true);
            try {
                GameView paying = getGameView(playerA);
                Assert.assertNotNull(paying.getCanPlayObjects());
                Assert.assertTrue("a player paying mana must see their lands as playable",
                        paying.getCanPlayObjects().containsObject(forest.getId()));

                GameView other = getGameView(playerB);
                Assert.assertFalse("the priority player is not shown the payer's lands",
                        other.getCanPlayObjects() != null && other.getCanPlayObjects().containsObject(forest.getId()));
            } finally {
                playerA.setPayManaMode(false);
            }
        });

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();
    }
}
