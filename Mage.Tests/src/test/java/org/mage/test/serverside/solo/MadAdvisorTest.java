package org.mage.test.serverside.solo;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.game.advice.Advice;
import mage.player.ai.MadAdvisor;
import org.junit.Assert;
import org.junit.Test;

import java.util.concurrent.atomic.AtomicReference;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * The advise call (solo foundations S003): the mad AI thinks from a player's seat in a copy of the game and says what
 * it would do, and the live game is untouched.
 */
public class MadAdvisorTest extends CardTestPlayerBase {

    @Test
    public void findsLethalBurn() {
        addCard(Zone.BATTLEFIELD, playerA, "Mountain");
        addCard(Zone.HAND, playerA, "Lightning Bolt");
        setLife(playerB, 3);

        // ask while the game runs, on its own thread, as the server does
        AtomicReference<Advice> asked = new AtomicReference<>();
        runCode("advise", 1, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) ->
                asked.set(new MadAdvisor().advise(game.createSimulationForAI(), player.getId(), 4)));
        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();

        Advice advice = asked.get();
        Assert.assertNotNull(advice);
        Assert.assertEquals(advice.text, Advice.AdviceKind.PLAY, advice.kind);
        Assert.assertTrue(advice.text, advice.text.contains("Cast Lightning Bolt targeting PlayerB"));
        Assert.assertEquals(1, advice.steps.size());
        Assert.assertTrue(advice.steps.get(0).targetIds.contains(playerB.getId()));
        Assert.assertNotNull(advice.when);

        // only the copy changed
        assertLife(playerB, 3);
        assertHandCount(playerA, "Lightning Bolt", 1);
    }

    @Test
    public void passesWithNothingToDo() {
        addCard(Zone.BATTLEFIELD, playerA, "Forest");

        AtomicReference<Advice> asked = new AtomicReference<>();
        runCode("advise", 1, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) ->
                asked.set(new MadAdvisor().advise(game.createSimulationForAI(), player.getId(), 4)));
        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();

        Advice advice = asked.get();
        Assert.assertEquals(advice.text, Advice.AdviceKind.PASS, advice.kind);
    }

    @Test
    public void attacksIntoAnEmptyBoard() {
        addCard(Zone.BATTLEFIELD, playerA, "Hill Giant");
        setLife(playerB, 3);

        setStopAt(3, PhaseStep.DECLARE_ATTACKERS);
        execute();

        Advice advice = new MadAdvisor().advise(currentGame.createSimulationForAI(), playerA.getId(), 4);
        Assert.assertEquals(advice.text, Advice.AdviceKind.ATTACK, advice.kind);
        Assert.assertTrue(advice.text, advice.text.contains("Hill Giant attacks PlayerB"));
    }
}
