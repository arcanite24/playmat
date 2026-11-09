package org.mage.test.AI.basic;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.counters.CounterType;
import mage.game.permanent.Permanent;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBaseAI;

/**
 * AI must stop an unbounded loop instead of repeating it forever (#231).
 */
public class RepeatedActionsCapAITest extends CardTestPlayerBaseAI {

    /**
     * The game in #231: Blade of the Bloodchief on Basking Broodscale. The AI equips the Blade to an Eldrazi Spawn and
     * pays the equip cost by sacrificing that same Spawn: the Spawn dies, the Broodscale gets a counter and makes a new
     * Spawn, the equip fizzles, and the AI is back where it started with one more counter and its equip still on the
     * stack. Every loop scores a little better and the stack never empties, so the turn never ended.
     */
    @Test
    public void test_BroodscaleBladeLoop_Stops() {
        // Whenever one or more +1/+1 counters are put on Basking Broodscale, you may create a 0/1 Eldrazi Spawn
        // {1}{G}: Adapt 1.
        addCard(Zone.BATTLEFIELD, playerA, "Basking Broodscale", 1);
        // Whenever a creature dies, put a +1/+1 counter on equipped creature. Equip {1}
        addCard(Zone.BATTLEFIELD, playerA, "Blade of the Bloodchief", 1);
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 3);

        // set the combo up: Blade on Broodscale. The AI adapts with the two lands left (its first Spawn), then has only
        // the loop to play
        activateAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Equip {1}", "Basking Broodscale");

        setStopAt(2, PhaseStep.UPKEEP);
        execute();

        // the turn ended, after a bounded number of loops
        Permanent broodscale = getPermanent("Basking Broodscale", playerA);
        int counters = broodscale.getCounters(currentGame).getCount(CounterType.P1P1);
        Assert.assertTrue("AI must stop the loop, counters: " + counters, counters < 50);
    }
}
