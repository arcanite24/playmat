package org.mage.test.serverside.solo;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.server.career.CareerSetup;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

import java.io.ByteArrayOutputStream;
import java.io.ObjectOutputStream;
import java.util.ArrayList;
import java.util.Collections;

/**
 * Twists that come in during a Career game (S008): at a turn, at a life threshold, when a card is cast. Each fires
 * once, for the player holding it. Neither test player is human, so both hold the same twists, each watching itself.
 */
public class TimedTwistsTest extends CardTestPlayerBase {

    private static CareerSetup.TimedTwist twist(String when) {
        CareerSetup.TimedTwist twist = new CareerSetup.TimedTwist();
        twist.when = when;
        return twist;
    }

    private static CareerSetup setup() {
        CareerSetup.CareerTwists twists = new CareerSetup.CareerTwists();
        twists.timed = new ArrayList<>();
        CareerSetup.TimedTwist atTurn = twist(CareerSetup.TimedTwist.TURN);
        atTurn.turn = 3;
        atTurn.gainLife = 5;
        atTurn.message = "Reinforcements arrive.";
        twists.timed.add(atTurn);
        CareerSetup.TimedTwist atLife = twist(CareerSetup.TimedTwist.LIFE);
        atLife.life = 15;
        atLife.who = "opponent";
        atLife.then = new CareerSetup.CareerTwists();
        atLife.then.permanents = Collections.singletonList("Grizzly Bears");
        twists.timed.add(atLife);
        CareerSetup.TimedTwist onCast = twist(CareerSetup.TimedTwist.CAST);
        onCast.card = "Lightning Bolt";
        onCast.who = "opponent";
        onCast.gainLife = 2;
        twists.timed.add(onCast);
        // a broken one is skipped, not fatal
        twists.timed.add(twist("someday"));
        return CareerSetup.ofTwists(twists);
    }

    @Test
    public void twistsFireOnceEachAtTheirMoment() {
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 2);
        addCard(Zone.HAND, playerA, "Lightning Bolt", 2);

        runCode("twists in", 1, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) -> setup().apply(game, null));
        castSpell(1, PhaseStep.POSTCOMBAT_MAIN, playerA, "Lightning Bolt", playerB);
        waitStackResolved(1, PhaseStep.POSTCOMBAT_MAIN);
        castSpell(1, PhaseStep.POSTCOMBAT_MAIN, playerA, "Lightning Bolt", playerB);
        runCode("saves with the game", 3, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) -> {
            try (ObjectOutputStream out = new ObjectOutputStream(new ByteArrayOutputStream())) {
                // the test players themselves don't serialize; the command zone holds the twists
                Assert.assertFalse(game.getState().getCommand().isEmpty());
                out.writeObject(game.getState().getCommand());
                out.writeObject(game.getState().getTriggers());
            } catch (Exception e) {
                throw new AssertionError("the twists must serialize with the game", e);
            }
        });

        setStrictChooseMode(true);
        setStopAt(3, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        // A: +2 for the first bolt only, +5 at turn 3
        assertLife(playerA, 20 + 2 + 5);
        // B: two bolts take it to 14, under its threshold: bears; then +5 at turn 3
        assertLife(playerB, 20 - 3 - 3 + 5);
        assertPermanentCount(playerB, "Grizzly Bears", 1);
        assertPermanentCount(playerA, "Grizzly Bears", 0);
    }

    @Test
    public void malformedTwistsSayWhy() {
        Assert.assertNotNull(twist(null).problem());
        Assert.assertNotNull(twist(CareerSetup.TimedTwist.TURN).problem());
        CareerSetup.TimedTwist life = twist(CareerSetup.TimedTwist.LIFE);
        life.life = 5;
        Assert.assertNotNull("nothing happens", life.problem());
        life.gainLife = 3;
        Assert.assertNull(life.problem());
        life.who = "everyone";
        Assert.assertNotNull(life.problem());
    }
}
