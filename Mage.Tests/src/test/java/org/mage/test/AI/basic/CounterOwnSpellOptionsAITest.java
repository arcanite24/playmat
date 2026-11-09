package org.mage.test.AI.basic;

import mage.MageObject;
import mage.abilities.Ability;
import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.stack.StackObject;
import mage.player.ai.SimulatedPlayer2;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

import java.util.List;
import java.util.UUID;

/**
 * AI search options must never counter the AI's own spells (#224). PlayerA is scripted here, the options are asked
 * for directly.
 */
public class CounterOwnSpellOptionsAITest extends CardTestPlayerBase {

    /**
     * The search's options: with only its own spell on the stack, the AI's counterspells aren't options at all; with
     * an opponent's spell, they are.
     */
    @Test
    public void test_SearchOptions_NeverTargetOwnSpell() {
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 2);
        addCard(Zone.BATTLEFIELD, playerA, "Island", 2);
        addCard(Zone.HAND, playerA, "Grizzly Bears", 1);
        addCard(Zone.HAND, playerA, "Counterspell", 1);
        // You may pay 1 life and exile a blue card from your hand rather than pay this spell's mana cost.
        // Counter target spell.
        addCard(Zone.HAND, playerA, "Force of Will", 2);

        addCard(Zone.BATTLEFIELD, playerB, "Forest", 2);
        addCard(Zone.HAND, playerB, "Grizzly Bears", 1);

        // own spell on the stack: no counterspell option at all
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Grizzly Bears");
        runCode("own spell only", 1, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) -> {
            Assert.assertEquals(1, game.getStack().size());
            assertNoOwnStackTargets(game, player.getId(), searchOptions(game, player.getId()), 0);
        });

        // opponent's spell on the stack: counterspell options stay
        castSpell(2, PhaseStep.PRECOMBAT_MAIN, playerB, "Grizzly Bears");
        runCode("opponent's spell", 2, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) -> {
            Assert.assertEquals(1, game.getStack().size());
            assertNoOwnStackTargets(game, player.getId(), searchOptions(game, player.getId()), 1);
        });

        setStopAt(2, PhaseStep.BEGIN_COMBAT);
        setStrictChooseMode(true);
        execute();
    }

    private static List<Ability> searchOptions(Game game, UUID playerId) {
        return new SimulatedPlayer2(game.getPlayer(playerId), true).simulatePriority(game);
    }

    private static void assertNoOwnStackTargets(Game game, UUID playerId, List<Ability> options, int minCounters) {
        int counters = 0;
        for (Ability option : options) {
            MageObject source = game.getObject(option.getSourceId());
            if (source == null || !(source.getName().equals("Counterspell") || source.getName().equals("Force of Will"))) {
                continue;
            }
            for (UUID targetId : option.getAllSelectedTargets().stream()
                    .flatMap(target -> target.getTargets().stream())
                    .toArray(UUID[]::new)) {
                StackObject stackObject = game.getStack().getStackObject(targetId);
                Assert.assertFalse("AI must not counter its own spell: " + option,
                        stackObject != null && playerId.equals(stackObject.getControllerId()));
            }
            counters++;
        }
        Assert.assertTrue("counterspell options: " + counters + " in " + options, counters >= minCounters);
    }

}
