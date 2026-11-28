package org.mage.test.view;

import mage.cards.Card;
import mage.cards.CardsImpl;
import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.view.CardView;
import mage.view.RevealedView;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * A revealed card is shown with the zone it was revealed from, not the zone it is in now: opponents must not follow
 * a card revealed from a hand into the library.
 *
 * @author arcanite24
 */
public class RevealedViewZoneTest extends CardTestPlayerBase {

    @Test
    public void test_RevealedFromHandStaysHandAfterGoingToLibrary() {
        addCard(Zone.HAND, playerB, "Grizzly Bears");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.PRECOMBAT_MAIN);
        execute();

        Card bears = playerB.getHand().getCards(currentGame).stream()
                .filter(card -> card.getName().equals("Grizzly Bears"))
                .findFirst().orElse(null);
        Assert.assertNotNull(bears);
        playerB.revealCards("test", new CardsImpl(bears), currentGame);
        Assert.assertEquals(Zone.HAND, currentGame.getState().getRevealed().getRevealedFrom(bears.getId()));

        // put back on the library, as Brainstorm would: the reveal still says hand
        playerB.putCardsOnTopOfLibrary(bears, currentGame, null, false);
        Assert.assertEquals(Zone.LIBRARY, currentGame.getState().getZone(bears.getId()));
        boolean shownOnce = false;
        for (String name : currentGame.getState().getRevealed().keySet()) {
            RevealedView view = new RevealedView(name, currentGame.getState().getRevealed().get(name), currentGame);
            CardView shown = view.getCards().get(bears.getId());
            if (shown != null) {
                Assert.assertEquals(Zone.HAND, shown.getZone());
                shownOnce = true;
            }
        }
        Assert.assertTrue("the reveal should still list the card", shownOnce);
    }
}
