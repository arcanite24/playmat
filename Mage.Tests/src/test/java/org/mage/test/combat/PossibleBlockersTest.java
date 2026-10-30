package org.mage.test.combat;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.filter.common.FilterCreatureForCombatBlock;
import mage.filter.predicate.permanent.ControllerIdPredicate;
import mage.game.Game;
import mage.player.human.HumanPlayer;
import mage.player.human.PlayerResponse;
import mage.players.PlayerImpl;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

import java.util.List;
import java.util.UUID;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

public class PossibleBlockersTest extends CardTestPlayerBase {

    private static class BlockerChoices extends HumanPlayer {
        BlockerChoices(PlayerImpl player) {
            super(player, new PlayerResponse());
        }

        List<UUID> choices(UUID defenderId, Game game) {
            FilterCreatureForCombatBlock filter = new FilterCreatureForCombatBlock();
            filter.add(new ControllerIdPredicate(defenderId));
            return getPossibleBlockers(filter, game);
        }
    }

    private List<UUID> choices() {
        return new BlockerChoices(playerA.getComputerPlayer())
                .choices(playerA.getId(), currentGame);
    }

    private void assertOffered(List<UUID> choices, String name, boolean offered) {
        UUID id = getPermanent(name, playerA.getId()).getId();
        if (offered) {
            assertTrue(name + " should be offered as a blocker", choices.contains(id));
        } else {
            assertFalse(name + " cannot block any attacker", choices.contains(id));
        }
    }

    @Test
    public void flyingOnlyOffersFlyingAndReach() {
        addCard(Zone.BATTLEFIELD, playerA, "Grizzly Bears");
        addCard(Zone.BATTLEFIELD, playerA, "Giant Spider");
        addCard(Zone.BATTLEFIELD, playerA, "Wind Drake");
        addCard(Zone.BATTLEFIELD, playerB, "Air Elemental");

        attack(2, playerB, "Air Elemental");
        setStopAt(2, PhaseStep.DECLARE_BLOCKERS);
        execute();

        List<UUID> choices = choices();
        assertOffered(choices, "Grizzly Bears", false);
        assertOffered(choices, "Giant Spider", true);
        assertOffered(choices, "Wind Drake", true);
    }

    @Test
    public void groundCreatureCanBlockAnotherAttacker() {
        addCard(Zone.BATTLEFIELD, playerA, "Grizzly Bears");
        addCard(Zone.BATTLEFIELD, playerB, "Air Elemental");
        addCard(Zone.BATTLEFIELD, playerB, "Hill Giant");

        attack(2, playerB, "Air Elemental");
        attack(2, playerB, "Hill Giant");
        setStopAt(2, PhaseStep.DECLARE_BLOCKERS);
        execute();

        assertOffered(choices(), "Grizzly Bears", true);
    }

    @Test
    public void unblockableAttackerOffersNoBlockers() {
        addCard(Zone.BATTLEFIELD, playerA, "Grizzly Bears");
        addCard(Zone.BATTLEFIELD, playerA, "Wind Drake");
        addCard(Zone.BATTLEFIELD, playerB, "Invisible Stalker");

        attack(2, playerB, "Invisible Stalker");
        setStopAt(2, PhaseStep.DECLARE_BLOCKERS);
        execute();

        assertTrue("No creature can block Invisible Stalker", choices().isEmpty());
    }

    @Test
    public void protectionExcludesBlockersOfThatColor() {
        addCard(Zone.BATTLEFIELD, playerA, "Grizzly Bears");
        addCard(Zone.BATTLEFIELD, playerA, "Hill Giant");
        addCard(Zone.BATTLEFIELD, playerB, "Mirran Crusader");

        attack(2, playerB, "Mirran Crusader");
        setStopAt(2, PhaseStep.DECLARE_BLOCKERS);
        execute();

        List<UUID> choices = choices();
        assertOffered(choices, "Grizzly Bears", false);
        assertOffered(choices, "Hill Giant", true);
    }
}
