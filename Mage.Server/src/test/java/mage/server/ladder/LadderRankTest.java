package mage.server.ladder;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

/** The visible rank's rules (#431). */
public class LadderRankTest {

    private static int step(String rank, int tier, int pips) {
        int r = java.util.Arrays.asList(LadderRank.RANKS).indexOf(rank);
        return r * LadderRank.RANK_STEPS + (LadderRank.TIERS - tier) * LadderRank.PIPS + pips;
    }

    @Test
    void stepsReadAsRankTierAndPips() {
        assertEquals("Bronze 4", LadderRank.label(0));
        assertEquals(0, LadderRank.pips(0));
        assertEquals("Bronze 3", LadderRank.label(4));
        assertEquals("Silver 4", LadderRank.label(16));
        assertEquals("Gold 2", LadderRank.label(step("gold", 2, 3)));
        assertEquals(3, LadderRank.pips(step("gold", 2, 3)));
        assertEquals("Diamond 1", LadderRank.label(LadderRank.MYTHIC_STEP - 1));
        assertEquals("Mythic", LadderRank.label(LadderRank.MYTHIC_STEP));
        assertEquals(0, LadderRank.tier(LadderRank.MYTHIC_STEP));
    }

    @Test
    void aWinIsAPipAndAThirdInARowTwoThroughGold() {
        LadderRank.Move first = LadderRank.after(0, 0, 1);
        assertEquals(1, first.step);
        assertEquals(1, first.streak);
        assertFalse(first.bonus);
        LadderRank.Move third = LadderRank.after(2, 2, 1);
        assertEquals(4, third.step, "a bonus pip carries into the next tier");
        assertTrue(third.bonus);
        LadderRank.Move gold = LadderRank.after(step("gold", 1, 3), 5, 1);
        assertEquals(step("platinum", 4, 1), gold.step, "the last Gold win still has the bonus");
        LadderRank.Move platinum = LadderRank.after(step("platinum", 4, 0), 5, 1);
        assertEquals(1, platinum.delta, "no bonus from Platinum");
    }

    @Test
    void lossesFollowEachRanksFloor() {
        assertEquals(3, LadderRank.after(3, 2, 0).step, "Bronze losses cost nothing");
        assertEquals(0, LadderRank.after(3, 2, 0).streak, "but end the streak");
        assertEquals(step("silver", 3, 1), LadderRank.after(step("silver", 3, 2), 0, 0).step);
        assertEquals(step("silver", 3, 0), LadderRank.after(step("silver", 3, 0), 0, 0).step, "Silver never drops a tier");
        assertEquals(step("gold", 3, 3), LadderRank.after(step("gold", 2, 0), 0, 0).step, "Gold can drop a tier");
        assertEquals(step("gold", 4, 0), LadderRank.after(step("gold", 4, 0), 0, 0).step, "but not the rank");
        assertEquals(step("diamond", 4, 0), LadderRank.after(step("diamond", 4, 0), 0, 0).step);
        assertEquals(LadderRank.MYTHIC_STEP, LadderRank.after(LadderRank.MYTHIC_STEP, 0, 0).step, "Mythic is kept");
        assertEquals(LadderRank.MYTHIC_STEP, LadderRank.after(LadderRank.MYTHIC_STEP, 0, 1).step, "and is the top");
    }

    @Test
    void aDrawEndsTheStreakAndNothingElse() {
        LadderRank.Move draw = LadderRank.after(step("gold", 2, 1), 2, 0.5);
        assertEquals(step("gold", 2, 1), draw.step);
        assertEquals(0, draw.streak);
    }

    @Test
    void podPlacementMovementRewardsFirstAndProtectsSecondThird() {
        LadderRank.Move first = LadderRank.afterPod(step("gold", 3, 0), 0, 1);
        assertEquals(2, first.delta, "1st place in a 4-player pod awards +2 pips");
        LadderRank.Move secondGold = LadderRank.afterPod(step("gold", 3, 0), 0, 2);
        assertEquals(1, secondGold.delta, "2nd place in Bronze–Gold awards +1 pip");
        LadderRank.Move secondPlat = LadderRank.afterPod(step("platinum", 3, 2), 0, 2);
        assertEquals(0, secondPlat.delta, "2nd place in Platinum+ is protected");
        LadderRank.Move thirdGold = LadderRank.afterPod(step("gold", 3, 2), 0, 3);
        assertEquals(0, thirdGold.delta, "3rd place in Bronze–Gold is protected");
        LadderRank.Move thirdPlat = LadderRank.afterPod(step("platinum", 3, 2), 0, 3);
        assertEquals(-1, thirdPlat.delta, "3rd place in Platinum–Diamond loses 1 pip");
        LadderRank.Move fourthBronze = LadderRank.afterPod(step("bronze", 2, 2), 0, 4);
        assertEquals(0, fourthBronze.delta, "4th place in Bronze costs nothing");
        LadderRank.Move fourthSilver = LadderRank.afterPod(step("silver", 2, 2), 0, 4);
        assertEquals(-1, fourthSilver.delta, "4th place in Silver+ loses 1 pip");
    }
}
