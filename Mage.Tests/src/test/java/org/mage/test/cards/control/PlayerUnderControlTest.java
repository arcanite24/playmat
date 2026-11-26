package org.mage.test.cards.control;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import com.google.gson.JsonObject;
import mage.cards.Card;
import mage.game.permanent.Permanent;
import mage.server.websocket.rpc.JsonCodec;
import mage.view.CardView;
import mage.view.GameView;
import mage.view.SimpleCardView;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * Test Framework do not support under control commands, so check only game related info and data
 *
 * @author JayDi85
 */
public class PlayerUnderControlTest extends CardTestPlayerBase {

    @Test
    public void test_ClientSideDataMustBeHidden() {
        // possible bug: after control ends - player still able to view opponent's hands

        // When you cast Emrakul, you gain control of target opponent during that player's next turn.
        // After that turn, that player takes an extra turn.
        addCard(Zone.HAND, playerA, "Emrakul, the Promised End"); // {13}
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 13);
        //
        addCard(Zone.HAND, playerB, "Lightning Bolt");
        addCard(Zone.BATTLEFIELD, playerB, "Mountain", 1);

        // prepare control effect
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Emrakul, the Promised End");
        addTarget(playerA, playerB);
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);

        checkUnderControl("turn 1 - A, B normal", 1, PhaseStep.PRECOMBAT_MAIN, false);
        checkUnderControl("turn 2 - B under A", 2, PhaseStep.PRECOMBAT_MAIN, true);
        checkUnderControl("turn 3 - A, B normal", 3, PhaseStep.PRECOMBAT_MAIN, false);

        setStrictChooseMode(true);
        setStopAt(3, PhaseStep.END_TURN);
        execute();
    }

    @Test
    public void test_ControllerSeesAndPlaysControlledHand() {
        // the controller casts from the controlled player's hand and taps their lands, so their view needs that hand
        // as whole cards (a client renders and casts from it) and the hand's spells and the lands among its playables
        addCard(Zone.HAND, playerA, "Emrakul, the Promised End"); // {13}
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 13);
        //
        addCard(Zone.HAND, playerB, "Raise the Alarm"); // {1}{W}
        addCard(Zone.BATTLEFIELD, playerB, "Plains", 1);
        addCard(Zone.BATTLEFIELD, playerB, "Godless Shrine", 1);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Emrakul, the Promised End");
        addTarget(playerA, playerB);
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);

        // playerB holds priority on their own turn, under playerA's control
        runCode("controller's view", 2, PhaseStep.PRECOMBAT_MAIN, playerB, (info, player, game) -> {
            Assert.assertEquals(playerA.getId(), playerB.getTurnControlledBy());
            Card alarm = playerB.getHand().getCards(game).stream()
                    .filter(card -> card.getName().equals("Raise the Alarm"))
                    .findFirst().orElse(null);
            Permanent shrine = game.getBattlefield().getAllActivePermanents(playerB.getId()).stream()
                    .filter(permanent -> permanent.getName().equals("Godless Shrine"))
                    .findFirst().orElse(null);
            Assert.assertNotNull(alarm);
            Assert.assertNotNull(shrine);

            GameView viewA = getGameView(playerA);
            SimpleCardView inHand = viewA.getOpponentHands().get(playerB.getName()).get(alarm.getId());
            Assert.assertTrue("the controlled hand is sent as whole cards", inHand instanceof CardView);
            Assert.assertEquals("Raise the Alarm", ((CardView) inHand).getName());

            Assert.assertNotNull(viewA.getCanPlayObjects());
            Assert.assertTrue("the controller can cast from the controlled hand",
                    viewA.getCanPlayObjects().containsObject(alarm.getId()));
            Assert.assertTrue("the controller can tap the controlled player's dual land",
                    viewA.getCanPlayObjects().containsObject(shrine.getId()));

            // the web client gets the whole card, not only its picture's reference
            JsonObject json = JsonCodec.GSON.toJsonTree(viewA).getAsJsonObject()
                    .getAsJsonObject("opponentHands").getAsJsonObject(playerB.getName())
                    .getAsJsonObject(alarm.getId().toString());
            Assert.assertEquals("Raise the Alarm", json.get("name").getAsString());
            Assert.assertTrue("rules reach the client", json.has("rules"));
            Assert.assertTrue("the mana cost reaches the client", json.has("manaCostLeftStr") || json.has("manaCostStr"));

            // the controlled player's own view keeps their hand as their own
            Assert.assertTrue(getGameView(playerB).getOpponentHands().isEmpty());
        });

        setStrictChooseMode(true);
        setStopAt(2, PhaseStep.END_TURN);
        execute();
    }

    private void checkUnderControl(String info, int turnNum, PhaseStep step, boolean mustHaveControl) {
        runCode(info, turnNum, step, playerA, (info1, player, game) -> {
            GameView viewA = getGameView(playerA);
            GameView viewB = getGameView(playerB);
            if (mustHaveControl) {
                Assert.assertTrue(info, playerA.isGameUnderControl());
                Assert.assertFalse(info, playerB.isGameUnderControl());

                Assert.assertTrue(info, playerA.getPlayersUnderYourControl().contains(playerB.getId()));
                Assert.assertTrue(info, playerB.getPlayersUnderYourControl().isEmpty());

                Assert.assertTrue(info, playerA.getTurnControllers().isEmpty());
                Assert.assertTrue(info, playerB.getTurnControllers().contains(playerA.getId()));

                Assert.assertEquals(info, playerA.getTurnControlledBy(), playerA.getId());
                Assert.assertEquals(info, playerB.getTurnControlledBy(), playerA.getId());

                // client side
                Assert.assertFalse(info, viewA.getOpponentHands().isEmpty());
                Assert.assertTrue(info, viewB.getOpponentHands().isEmpty());
            } else {
                // A,B normal
                Assert.assertTrue(info, playerA.isGameUnderControl());
                Assert.assertTrue(info, playerB.isGameUnderControl());

                Assert.assertTrue(info, playerA.getPlayersUnderYourControl().isEmpty());
                Assert.assertTrue(info, playerB.getPlayersUnderYourControl().isEmpty());

                Assert.assertTrue(info, playerA.getTurnControllers().isEmpty());
                Assert.assertTrue(info, playerB.getTurnControllers().isEmpty());

                Assert.assertEquals(info, playerA.getTurnControlledBy(), playerA.getId());
                Assert.assertEquals(info, playerB.getTurnControlledBy(), playerB.getId());

                // client side
                Assert.assertTrue(info, viewA.getOpponentHands().isEmpty());
                Assert.assertTrue(info, viewB.getOpponentHands().isEmpty());
            }
        });
    }
}
