package org.mage.test.cards.single.trk;

import mage.cards.Card;
import mage.cards.repository.CardRepository;
import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.game.Game;
import mage.players.Player;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

import java.util.Collections;

/**
 * @author arcanite24
 */
public class TheHolodeckTest extends CardTestPlayerBase {

    // Once during each of your turns, you may play a permanent card you own from outside the game or from your graveyard.
    private static final String HOLODECK = "The Holodeck";

    private static void putInSideboard(Game game, Player player, String cardName) {
        Card card = CardRepository.instance.findCards(cardName).get(0).createCard();
        game.loadCards(Collections.singleton(card), player.getId());
        player.getSideboard().add(card);
        game.getState().setZone(card.getId(), Zone.OUTSIDE);
    }

    @Test
    public void test_CastFromGraveyard_OncePerTurn() {
        addCard(Zone.BATTLEFIELD, playerA, HOLODECK);
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 4);
        addCard(Zone.GRAVEYARD, playerA, "Grizzly Bears");
        addCard(Zone.GRAVEYARD, playerA, "Llanowar Elves");

        checkPlayableAbility("bears playable", 1, PhaseStep.PRECOMBAT_MAIN, playerA, "Cast Grizzly Bears", true);
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Grizzly Bears");
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);
        checkPlayableAbility("used this turn", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, "Cast Llanowar Elves", false);
        // a new turn, a new use
        checkPlayableAbility("next turn", 3, PhaseStep.PRECOMBAT_MAIN, playerA, "Cast Llanowar Elves", true);
        castSpell(3, PhaseStep.PRECOMBAT_MAIN, playerA, "Llanowar Elves");

        setStrictChooseMode(true);
        setStopAt(3, PhaseStep.END_TURN);
        execute();

        assertPermanentCount(playerA, "Grizzly Bears", 1);
        assertPermanentCount(playerA, "Llanowar Elves", 1);
    }

    @Test
    public void test_PlayLandFromGraveyard() {
        addCard(Zone.BATTLEFIELD, playerA, HOLODECK);
        addCard(Zone.GRAVEYARD, playerA, "Forest");

        playLand(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Forest");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertPermanentCount(playerA, "Forest", 1);
        assertGraveyardCount(playerA, "Forest", 0);
    }

    @Test
    public void test_CastFromOutsideTheGame() {
        addCard(Zone.BATTLEFIELD, playerA, HOLODECK);
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 2);
        addCard(Zone.GRAVEYARD, playerA, "Llanowar Elves");

        runCode("sideboard", 1, PhaseStep.UPKEEP, playerA, (info, player, game) -> putInSideboard(game, player, "Grizzly Bears"));
        checkPlayableAbility("bears playable", 1, PhaseStep.PRECOMBAT_MAIN, playerA, "Cast Grizzly Bears", true);
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Grizzly Bears");
        waitStackResolved(1, PhaseStep.PRECOMBAT_MAIN);
        // the one use covers both zones
        checkPlayableAbility("used this turn", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, "Cast Llanowar Elves", false);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertPermanentCount(playerA, "Grizzly Bears", 1);
    }

    @Test
    public void test_NotNonpermanentCards() {
        addCard(Zone.BATTLEFIELD, playerA, HOLODECK);
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 1);
        addCard(Zone.GRAVEYARD, playerA, "Lightning Bolt");

        checkPlayableAbility("no bolt", 1, PhaseStep.PRECOMBAT_MAIN, playerA, "Cast Lightning Bolt", false);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();
    }

    @Test
    public void test_NotOnOpponentsTurn() {
        addCard(Zone.BATTLEFIELD, playerA, HOLODECK);
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 2);
        addCard(Zone.GRAVEYARD, playerA, "Ambush Viper"); // flash

        checkPlayableAbility("not on their turn", 2, PhaseStep.PRECOMBAT_MAIN, playerA, "Cast Ambush Viper", false);
        checkPlayableAbility("on my turn", 3, PhaseStep.UPKEEP, playerA, "Cast Ambush Viper", true);

        setStrictChooseMode(true);
        setStopAt(3, PhaseStep.DRAW);
        execute();
    }
}
