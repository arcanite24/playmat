package org.mage.test.AI.basic;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.SimulationInterruptedException;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBaseAI;

/**
 * The AI's searches share a few threads. A search past its time limit is interrupted, and a simulation that didn't
 * check that (one stuck resolving a copy effect) held a thread for hours: every other AI on the server waited for a
 * thread, gave up and passed, so none of them played a land. Simulations now stop on the interrupt.
 */
public class SimulationInterruptAITest extends CardTestPlayerBaseAI {

    @Test
    public void test_InterruptedSimulationStops() {
        addCard(Zone.BATTLEFIELD, playerA, "Glorious Anthem");
        addCard(Zone.BATTLEFIELD, playerA, "Grizzly Bears");

        runCode("interrupt", 1, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) -> {
            Game sim = game.createSimulationForAI();
            Assert.assertFalse(sim.getState().getContinuousEffects().getLayeredEffects(sim).isEmpty());

            Thread.currentThread().interrupt();
            try {
                sim.getState().getContinuousEffects().getLayeredEffects(sim);
                Assert.fail("an interrupted simulation must stop");
            } catch (SimulationInterruptedException e) {
                // expected
            } finally {
                Thread.interrupted();
            }

            // the real game is never stopped this way
            Thread.currentThread().interrupt();
            try {
                Assert.assertFalse(game.getState().getContinuousEffects().getLayeredEffects(game).isEmpty());
            } finally {
                Thread.interrupted();
            }
        });

        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        setStrictChooseMode(true);
        execute();

        assertPowerToughness(playerA, "Grizzly Bears", 3, 3);
    }
}
