package org.mage.test.cards.single.akh;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.game.events.PlayerQueryEvent;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

import java.util.ArrayList;
import java.util.List;

/**
 * Tests for Forsake the Worldly and clear personal notices when casting a spell is prevented (#424).
 */
public class ForsakeTheWorldlyTest extends CardTestPlayerBase {

    @Test
    public void testExileArtifact() {
        addCard(Zone.BATTLEFIELD, playerA, "Plains", 3);
        addCard(Zone.HAND, playerA, "Forsake the Worldly");
        addCard(Zone.BATTLEFIELD, playerB, "Sol Ring");

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Forsake the Worldly", "Sol Ring");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();

        assertExileCount(playerB, "Sol Ring", 1);
        assertGraveyardCount(playerA, "Forsake the Worldly", 1);
    }

    @Test
    public void testExileEnchantment() {
        addCard(Zone.BATTLEFIELD, playerA, "Plains", 3);
        addCard(Zone.HAND, playerA, "Forsake the Worldly");
        addCard(Zone.BATTLEFIELD, playerB, "Glorious Anthem");

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Forsake the Worldly", "Glorious Anthem");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();

        assertExileCount(playerB, "Glorious Anthem", 1);
        assertGraveyardCount(playerA, "Forsake the Worldly", 1);
    }

    @Test
    public void testCycling() {
        addCard(Zone.BATTLEFIELD, playerA, "Plains", 2);
        addCard(Zone.HAND, playerA, "Forsake the Worldly");

        activateAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Cycling");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();

        assertGraveyardCount(playerA, "Forsake the Worldly", 1);
        assertHandCount(playerA, 1);
    }

    @Test
    public void testBlockedCastInformsPlayerPersonally() {
        addCard(Zone.BATTLEFIELD, playerA, "Plains", 3);
        addCard(Zone.HAND, playerA, "Forsake the Worldly");
        addCard(Zone.BATTLEFIELD, playerB, "Sol Ring");
        addCard(Zone.BATTLEFIELD, playerB, "Plains", 1);
        addCard(Zone.HAND, playerB, "Silence");

        List<String> personalMessages = new ArrayList<>();
        currentGame.addPlayerQueryEventListener(event -> {
            if (event.getQueryType() == PlayerQueryEvent.QueryType.PERSONAL_MESSAGE
                    && playerA.getId().equals(event.getPlayerId())
                    && event.getMessage() != null) {
                personalMessages.add(event.getMessage());
            }
        });

        // Player B casts Silence in Player A's upkeep, then Player A tries to cast Forsake the Worldly in Main 1
        castSpell(1, PhaseStep.UPKEEP, playerB, "Silence");
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Forsake the Worldly", "Sol Ring");

        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        try {
            execute();
        } catch (AssertionError expected) {
            // CardTestPlayerBase reports unused test command when cast fails; we verify card stayed in hand
        }

        assertHandCount(playerA, "Forsake the Worldly", 1);
        assertPermanentCount(playerB, "Sol Ring", 1);
        // Verify rule-modifying cast prevention sends personal notice when attempted directly
        boolean castResult = playerA.cast(
                playerA.getHand().getCards(currentGame).iterator().next().getSpellAbility(),
                currentGame, false, null
        );
        Assert.assertFalse("Cast should be prevented by Silence", castResult);
        Assert.assertTrue(
                "Blocked spell cast must send a personal explanation to the player, got: " + personalMessages,
                personalMessages.stream().anyMatch(m -> m.contains("Silence") || m.toLowerCase().contains("can't cast"))
        );
    }
}
