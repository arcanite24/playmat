package mage.player.ai;

import mage.constants.RangeOfInfluence;
import org.junit.Assert;
import org.junit.Test;

/**
 * Events switch bots to quick decisions for bot-vs-bot matches and back for matches against a human.
 */
public class QuickDecisionsTest {

    @Test
    public void quickDecisionsCapTheSearchAndRestoreTheSkill() {
        ComputerPlayer7 bot = new ComputerPlayer7("Bot", RangeOfInfluence.ONE, 6);
        int depth = bot.maxDepth;
        int nodes = bot.maxNodes;
        int think = bot.maxThinkTimeSecs;

        bot.setQuickDecisions(true);
        Assert.assertTrue(bot.isQuickDecisions());
        Assert.assertTrue(bot.maxDepth <= ComputerPlayer6.QUICK_MAX_DEPTH);
        Assert.assertTrue(bot.maxNodes <= ComputerPlayer6.QUICK_MAX_NODES);
        Assert.assertEquals(ComputerPlayer6.QUICK_MAX_THINK_TIME_SECS, bot.maxThinkTimeSecs);

        // the game works on copies of the player: they must think as quickly
        ComputerPlayer6 copy = bot.copy();
        Assert.assertEquals(bot.maxThinkTimeSecs, copy.maxThinkTimeSecs);
        Assert.assertEquals(bot.maxNodes, copy.maxNodes);

        bot.setQuickDecisions(false);
        Assert.assertFalse(bot.isQuickDecisions());
        Assert.assertEquals(depth, bot.maxDepth);
        Assert.assertEquals(nodes, bot.maxNodes);
        Assert.assertEquals(think, bot.maxThinkTimeSecs);

        // a copy taken while quick can go back to full skill too
        copy.setQuickDecisions(false);
        Assert.assertEquals(think, copy.maxThinkTimeSecs);
    }
}
