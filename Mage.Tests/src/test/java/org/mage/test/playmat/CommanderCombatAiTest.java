package org.mage.test.playmat;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.game.permanent.Permanent;
import mage.player.ai.score.GameStateEvaluator2;
import mage.player.ai.util.CombatInfo;
import mage.player.ai.util.CombatUtil;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestCommander4Players;

import java.util.List;
import java.util.stream.Collectors;

/**
 * Tests Phase 1 AI upgrades: trample lethal multi-blocking and 4-player Commander evaluation.
 */
public class CommanderCombatAiTest extends CardTestCommander4Players {

    /**
     * When a 6/6 trampler attacks a player at 3 life who controls six 1/1 creatures,
     * CombatUtil must assign at least 4 blockers so the defending player survives instead of
     * stopping after a single blocker and letting 5 lethal trample damage carry through.
     */
    @Test
    public void testCombatUtilMultiBlocksLethalTrampleWithTokens() {
        setLife(playerB, 3);
        addCard(Zone.BATTLEFIELD, playerA, "Colossal Dreadmaw");
        addCard(Zone.BATTLEFIELD, playerB, "Llanowar Elves", 6);

        attack(1, playerA, "Colossal Dreadmaw", playerB);
        runCode("verify AI assigns enough blockers to survive lethal trample", 1, PhaseStep.DECLARE_BLOCKERS, playerB, (info, player, game) -> {
            List<Permanent> attackers = game.getCombat().getAttackers().stream()
                    .map(game::getPermanent)
                    .filter(p -> p != null)
                    .collect(Collectors.toList());
            List<Permanent> blockers = playerB.getAvailableBlockers(game);
            CombatInfo plan = CombatUtil.blockWithGoodTrade2(game, attackers, blockers, playerB.getId());
            int totalBlockers = plan.getCombat().values().stream().mapToInt(List::size).sum();
            Assert.assertTrue(
                    "Expected at least 4 blockers against 6/6 trample at 3 life, got " + totalBlockers,
                    totalBlockers >= 4
            );
        });

        setStopAt(1, PhaseStep.POSTCOMBAT_MAIN);
        execute();
    }

    /**
     * At high life (20 life vs 6/6 trampler), CombatUtil should not throw away multiple 1/1s
     * to chump-block non-lethal trample damage.
     */
    @Test
    public void testCombatUtilDoesNotChumpNonLethalTrample() {
        setLife(playerB, 20);
        addCard(Zone.BATTLEFIELD, playerA, "Colossal Dreadmaw");
        addCard(Zone.BATTLEFIELD, playerB, "Llanowar Elves", 6);

        attack(1, playerA, "Colossal Dreadmaw", playerB);
        runCode("verify AI does not chump non-lethal trample", 1, PhaseStep.DECLARE_BLOCKERS, playerB, (info, player, game) -> {
            List<Permanent> attackers = game.getCombat().getAttackers().stream()
                    .map(game::getPermanent)
                    .filter(p -> p != null)
                    .collect(Collectors.toList());
            List<Permanent> blockers = playerB.getAvailableBlockers(game);
            CombatInfo plan = CombatUtil.blockWithGoodTrade2(game, attackers, blockers, playerB.getId());
            int totalBlockers = plan.getCombat().values().stream().mapToInt(List::size).sum();
            Assert.assertEquals("Should not throw away 1/1s at 20 life against a 6/6 trampler", 0, totalBlockers);
        });

        setStopAt(1, PhaseStep.POSTCOMBAT_MAIN);
        execute();
    }

    /**
     * In a 4-player Commander game, GameStateEvaluator2 must account for threats on ALL opponents,
     * not just the first opponent returned by getOpponents().
     */
    @Test
    public void testMultiOpponentEvaluatorAccountsForAllPodThreats() {
        addCard(Zone.BATTLEFIELD, playerA, "Grizzly Bears");
        addCard(Zone.BATTLEFIELD, playerC, "Colossal Dreadmaw", 3);
        addCard(Zone.BATTLEFIELD, playerD, "Colossal Dreadmaw", 3);

        runCode("evaluator sees threats on C and D", 1, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) -> {
            GameStateEvaluator2.PlayerEvaluateScore score = GameStateEvaluator2.evaluate(playerA.getId(), game);
            Assert.assertTrue(
                    "Player A should evaluate behind when opponents C and D have massive boards, got " + score.getTotalScore(),
                    score.getTotalScore() < 0
            );
            Assert.assertTrue(
                    "Aggregated opponent permanent score should be substantial, got " + score.getOpponentScore(),
                    score.getOpponentScore() > score.getPlayerScore()
            );
        });

        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();
    }
}
