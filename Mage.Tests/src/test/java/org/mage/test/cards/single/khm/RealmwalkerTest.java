package org.mage.test.cards.single.khm;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.game.Game;
import mage.players.Player;
import mage.view.CardView;
import mage.view.GameView;
import mage.view.PlayerView;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * {@link mage.cards.r.Realmwalker Realmwalker}
 * {2}{G}
 * Creature — Shapeshifter
 * Changeling
 * As Realmwalker enters the battlefield, choose a creature type.
 * You may look at the top card of your library any time.
 * You may cast creature spells of the chosen type from the top of your library.
 */
public class RealmwalkerTest extends CardTestPlayerBase {

    /**
     * The look is sent once and cleared right away, so a client that shows the latest game view only (the web client)
     * lost it: the game view must carry the top card for its controller, and only for them.
     */
    @Test
    public void test_TopCardShownToControllerOnly() {
        skipInitShuffling();
        removeAllCardsFromLibrary(playerA);
        addCard(Zone.LIBRARY, playerA, "Grizzly Bears");
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 3 + 2);
        addCard(Zone.HAND, playerA, "Realmwalker");

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Realmwalker");
        setChoice(playerA, "Bear");
        runCode("top card", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, (info, player, game) -> {
            Assert.assertEquals("Grizzly Bears", topCardSeenBy(game, playerA, playerA));
            Assert.assertNull("opponent must not see it", topCardSeenBy(game, playerB, playerA));
        });
        checkPlayableAbility("cast from the top", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, "Cast Grizzly Bears", true);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();
    }

    private static String topCardSeenBy(Game game, Player viewer, Player owner) {
        GameView gameView = new GameView(game.getState(), game, viewer.getId(), null);
        PlayerView playerView = gameView.getPlayers().stream()
                .filter(view -> view.getPlayerId().equals(owner.getId()))
                .findFirst()
                .orElseThrow(AssertionError::new);
        CardView topCard = playerView.getTopCard();
        return topCard == null ? null : topCard.getName();
    }
}
