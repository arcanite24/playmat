package org.mage.test.AI.basic;

import mage.abilities.Ability;
import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.game.Game;
import mage.player.ai.SimulatedPlayer2;
import mage.target.TargetAmount;
import mage.target.TargetOptimization;
import mage.target.common.TargetCreaturePermanentAmount;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBaseAI;

import java.util.List;
import java.util.function.Supplier;

/**
 * The AI lists every way to play an ability as a full copy of it: each split of a divided amount, each combination of
 * targets. On a big board those multiply into millions of copies; a server's heap filled up that way, and the
 * search's time limit didn't stop it. The lists are capped, and stop when the search is interrupted.
 */
public class PlayableOptionsLimitAITest extends CardTestPlayerBaseAI {

    private SimulatedPlayer2 simulatedPlayerA(Game sim, Game game) {
        SimulatedPlayer2 simPlayer = new SimulatedPlayer2(sim.getPlayer(playerA.getId()), true);
        simPlayer.restore(game.getPlayer(playerA.getId()).copy());
        sim.getState().getPlayers().put(playerA.getId(), simPlayer);
        return simPlayer;
    }

    private static <T> T interruptedIf(boolean interrupted, Supplier<T> code) {
        if (interrupted) {
            Thread.currentThread().interrupt();
        }
        try {
            return code.get();
        } finally {
            Thread.interrupted();
        }
    }

    private List<Ability> coneOfFlameOptions(Game game, boolean interrupted) {
        Game sim = game.createSimulationForAI();
        SimulatedPlayer2 simPlayer = simulatedPlayerA(sim, game);
        Ability ability = simPlayer.getPlayable(sim, true).stream()
                .filter(playable -> playable.toString().contains("Cone of Flame"))
                .findFirst()
                .orElseThrow(() -> new AssertionError("Cone of Flame can't be cast"));
        return interruptedIf(interrupted, () -> simPlayer.getPlayableOptions(ability, sim));
    }

    private List<? extends TargetAmount> dividedOptions(Game game, int amount, boolean interrupted) {
        // "N damage divided as you choose among any number of target creatures", from one of playerA's permanents
        Ability source = getPermanent("Mountain", playerA).getAbilities().get(0);
        TargetAmount target = new TargetCreaturePermanentAmount(amount);
        return interruptedIf(interrupted, () -> target.getTargetOptions(source, game));
    }

    private void prepareBigBoard() {
        // 1 damage to any target, 2 damage to another target and 3 damage to a third target
        addCard(Zone.HAND, playerA, "Cone of Flame");
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 5);
        addCard(Zone.BATTLEFIELD, playerB, "Grizzly Bears", 20);
    }

    @Test
    public void test_ThreeTargets_OptionsAreCapped() {
        // 18 x 17 x 16 target combinations before the cap
        prepareBigBoard();

        runCode("options", 1, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) -> {
            List<Ability> options = coneOfFlameOptions(game, false);
            Assert.assertEquals("options must stop at the cap", TargetOptimization.AI_MAX_PLAYABLE_OPTIONS, options.size());
            Assert.assertTrue("an interrupted search must stop listing options", coneOfFlameOptions(game, true).isEmpty());
        });

        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();
    }

    @Test
    public void test_DividedAmount_OptionsAreCapped() {
        // 10 divided among 20 creatures: about 20 million splits before the cap
        prepareBigBoard();

        runCode("options", 1, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) -> {
            List<? extends TargetAmount> options = dividedOptions(game, 10, false);
            // the whole amount on any one creature is always among them, whatever the cap cut: at most one more each
            Assert.assertTrue("options must stop at the cap, got " + options.size(),
                    options.size() >= TargetOptimization.AI_MAX_TARGET_OPTIONS && options.size() <= TargetOptimization.AI_MAX_TARGET_OPTIONS + 20);
            long single = options.stream().filter(option -> option.getTargets().size() == 1).count();
            Assert.assertEquals("every creature can take the whole amount", 20, single);
            Assert.assertTrue("an interrupted search must stop listing options", dividedOptions(game, 10, true).isEmpty());
        });

        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();
    }

    @Test
    public void test_DividedAmount_SmallBoardKeepsEveryOption() {
        // under the cap nothing changes: 3 among 3 creatures has 3 + 6 + 1 = 10 splits (one, two or three targets)
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 1);
        addCard(Zone.BATTLEFIELD, playerB, "Grizzly Bears", 3);

        runCode("options", 1, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) -> {
            Assert.assertEquals(10, dividedOptions(game, 3, false).size());
        });

        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();
    }
}
