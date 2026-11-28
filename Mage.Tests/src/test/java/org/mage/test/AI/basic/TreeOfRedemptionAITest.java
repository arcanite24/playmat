package org.mage.test.AI.basic;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBaseWithAIHelps;

/**
 * Tree of Redemption ({T}: Exchange your life total with its toughness): the AI mustn't park its life in the Tree and
 * leave itself at a few life (Playmat #142). Toughness past 10 adds nothing to its score.
 *
 * @author arcanite24
 */
public class TreeOfRedemptionAITest extends CardTestPlayerBaseWithAIHelps {

    @Test
    public void test_LowLife_TakesTheLifeBack() {
        // the AI at 4 life with a 0/13 Tree: swapping gives it 13 life
        addCard(Zone.BATTLEFIELD, playerA, "Tree of Redemption"); // 0/13
        setLife(playerA, 4);

        aiPlayPriority(1, PhaseStep.PRECOMBAT_MAIN, playerA);

        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertLife(playerA, 13);
        assertPowerToughness(playerA, "Tree of Redemption", 0, 4);
    }

    @Test
    public void test_HighLife_KeepsItsLife() {
        // at 50 life the swap would hide 37 life in a wall: keep it
        addCard(Zone.BATTLEFIELD, playerA, "Tree of Redemption"); // 0/13
        setLife(playerA, 50);

        aiPlayPriority(1, PhaseStep.PRECOMBAT_MAIN, playerA);

        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertLife(playerA, 50);
        assertPowerToughness(playerA, "Tree of Redemption", 0, 13);
    }

    @Test
    public void test_BigTreeAtFourLife_TakesTheLifeBack() {
        // as reported: the Tree holds 50 and the AI is down to 4
        addCard(Zone.BATTLEFIELD, playerA, "Tree of Redemption"); // 0/13
        setLife(playerA, 50);
        activateAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "{T}: Exchange");
        runCode("down to 4 life", 3, PhaseStep.UPKEEP, playerA, (info, player, game) -> player.setLife(4, game, null));

        aiPlayStep(3, PhaseStep.PRECOMBAT_MAIN, playerA);

        setStopAt(3, PhaseStep.END_TURN);
        execute();

        assertLife(playerA, 50);
        assertPowerToughness(playerA, "Tree of Redemption", 0, 4);
    }
}
