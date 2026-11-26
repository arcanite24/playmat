package org.mage.test.playmat;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestCommander4Players;

public class FeedbackBugsTest extends CardTestCommander4Players {

    /**
     * Player B steals Player A's Roil Elemental with Modify Memory, then plays a land and targets Player C's creature.
     * Roil Elemental's Landfall (GainControlTargetEffect with Duration.WhileControlled) must work and keep control.
     */
    @Test
    public void testRoilElementalStolenByModifyMemoryLandfallWorks() {
        addCard(Zone.BATTLEFIELD, playerA, "Roil Elemental");
        addCard(Zone.BATTLEFIELD, playerB, "Grizzly Bears");
        addCard(Zone.BATTLEFIELD, playerB, "Island", 5);
        addCard(Zone.HAND, playerB, "Modify Memory");
        addCard(Zone.HAND, playerB, "Forest");
        addCard(Zone.BATTLEFIELD, playerC, "Hill Giant");

        // Turn 4 is Player B's turn (order: A=1, D=2, C=3, B=4)
        castSpell(4, PhaseStep.PRECOMBAT_MAIN, playerB, "Modify Memory", "Roil Elemental^Grizzly Bears", true);
        playLand(4, PhaseStep.PRECOMBAT_MAIN, playerB, "Forest");
        addTarget(playerB, "Hill Giant");
        setChoice(playerB, true); // yes to optional "you may gain control"

        setStopAt(4, PhaseStep.BEGIN_COMBAT);
        execute();

        assertPermanentCount(playerB, "Roil Elemental", 1);
        assertPermanentCount(playerA, "Grizzly Bears", 1);
        assertPermanentCount(playerB, "Hill Giant", 1);
    }

    /**
     * When the player who stole Roil Elemental later loses control of it, creatures stolen via its Landfall
     * (Duration.WhileControlled) must return to their previous controllers.
     */
    @Test
    public void testRoilElementalStolenBackReleasesLandfallControl() {
        addCard(Zone.BATTLEFIELD, playerA, "Roil Elemental");
        addCard(Zone.BATTLEFIELD, playerA, "Island", 4);
        addCard(Zone.HAND, playerA, "Control Magic");
        addCard(Zone.BATTLEFIELD, playerB, "Grizzly Bears");
        addCard(Zone.BATTLEFIELD, playerB, "Island", 5);
        addCard(Zone.HAND, playerB, "Modify Memory");
        addCard(Zone.HAND, playerB, "Forest");
        addCard(Zone.BATTLEFIELD, playerC, "Hill Giant");

        // Turn 4 (B): B steals Roil Elemental, plays Forest, steals Hill Giant
        castSpell(4, PhaseStep.PRECOMBAT_MAIN, playerB, "Modify Memory", "Roil Elemental^Grizzly Bears", true);
        playLand(4, PhaseStep.PRECOMBAT_MAIN, playerB, "Forest");
        addTarget(playerB, "Hill Giant");
        setChoice(playerB, true);

        // Turn 5 (A): A steals Roil Elemental back with Control Magic -> Hill Giant returns to C
        castSpell(5, PhaseStep.PRECOMBAT_MAIN, playerA, "Control Magic", "Roil Elemental");

        setStopAt(5, PhaseStep.BEGIN_COMBAT);
        execute();

        assertPermanentCount(playerA, "Roil Elemental", 1);
        assertPermanentCount(playerC, "Hill Giant", 1);
        assertPermanentCount(playerB, "Hill Giant", 0);
    }

    /**
     * Activating Alchemist's Refuge in 4-player Commander lets the player cast sorcery-speed spells
     * (from hand and from the command zone) on an opponent's turn, even in response to a spell on the stack,
     * and across multiple turns.
     */
    @Test
    public void testAlchemistsRefugeCommanderAndMultipleTurns() {
        addCard(Zone.BATTLEFIELD, playerB, "Alchemist's Refuge");
        addCard(Zone.BATTLEFIELD, playerB, "Forest", 2);
        addCard(Zone.BATTLEFIELD, playerB, "Island", 2);
        addCard(Zone.BATTLEFIELD, playerB, "Plains", 4);
        addCard(Zone.HAND, playerB, "Grizzly Bears");
        addCard(Zone.HAND, playerB, "Divination");
        addCard(Zone.COMMAND, playerB, "Isamaru, Hound of Konda");

        // Turn 1 is Player A's turn: Player B activates Alchemist's Refuge and casts Grizzly Bears + Commander
        activateAbility(1, PhaseStep.PRECOMBAT_MAIN, playerB, "{G}{U}, {T}:");
        runCode("check playable objects on opponent's turn after Refuge", 1, PhaseStep.BEGIN_COMBAT, playerB, (info, player, game) -> {
            mage.players.PlayableObjectsList playable = player.getPlayableObjects(game, Zone.ALL);
            boolean bearsPlayable = player.getHand().getCards(game).stream()
                    .filter(c -> c.getName().equals("Grizzly Bears"))
                    .anyMatch(c -> playable.getObjects().containsKey(c.getId()));
            org.junit.Assert.assertTrue("Grizzly Bears should be in getPlayableObjects after Alchemist's Refuge", bearsPlayable);
        });
        castSpell(1, PhaseStep.BEGIN_COMBAT, playerB, "Grizzly Bears");
        castSpell(1, PhaseStep.POSTCOMBAT_MAIN, playerB, "Isamaru, Hound of Konda");

        // Turn 5 is A's turn again after B untaps on Turn 4.
        activateAbility(5, PhaseStep.PRECOMBAT_MAIN, playerB, "{G}{U}, {T}:");
        castSpell(5, PhaseStep.BEGIN_COMBAT, playerB, "Divination");

        setStopAt(5, PhaseStep.END_TURN);
        execute();

        assertPermanentCount(playerB, "Grizzly Bears", 1);
        assertPermanentCount(playerB, "Isamaru, Hound of Konda", 1);
        assertGraveyardCount(playerB, "Divination", 1);
    }

    @Test
    public void testAlchemistsRefugeNotPlayableWithoutMana() {
        addCard(Zone.BATTLEFIELD, playerA, "Alchemist's Refuge");
        checkPlayableAbility("cannot activate Refuge with 0 mana", 1, PhaseStep.PRECOMBAT_MAIN, playerA, "{G}{U}, {T}:", false);
        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();
    }
}
