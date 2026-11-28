package org.mage.test.cards.single.c14;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.player.TestPlayer;
import org.mage.test.serverside.base.CardTestCommanderDuelBase;

/**
 * {@link mage.cards.c.CommandersSphere Commander's Sphere}
 * {3} Artifact
 * {T}: Add one mana of any color in your commander's color identity.
 * Sacrifice Commander's Sphere: Draw a card.
 * <p>
 * Reported: https://github.com/arcanite24/playmat/issues/230 (couldn't pay for Dissolve with two Islands and the
 * Sphere in response to Krosan Grip, and clicking the Sphere offered no choice of abilities).
 */
public class CommandersSphereTest extends CardTestCommanderDuelBase {

    @Test
    public void test_PaysForAnInstantWithLands() {
        addCard(Zone.COMMAND, playerA, "Kestia, the Cultivator", 1); // {1}{G}{W}{U}, blue in the color identity
        addCard(Zone.BATTLEFIELD, playerA, "Commander's Sphere", 1);
        addCard(Zone.BATTLEFIELD, playerA, "Island", 2);
        addCard(Zone.HAND, playerA, "Dissolve", 1); // {1}{U}{U} counter target spell, scry 1

        addCard(Zone.BATTLEFIELD, playerB, "Mountain", 1);
        addCard(Zone.HAND, playerB, "Lightning Bolt", 1);

        castSpell(2, PhaseStep.PRECOMBAT_MAIN, playerB, "Lightning Bolt", playerA);
        checkPlayableAbility("Dissolve castable with the Sphere", 2, PhaseStep.PRECOMBAT_MAIN, playerA, "Cast Dissolve", true);
        castSpell(2, PhaseStep.PRECOMBAT_MAIN, playerA, "Dissolve", "Lightning Bolt", "Lightning Bolt");
        setChoice(playerA, "Blue"); // Commander's Sphere
        addTarget(playerA, TestPlayer.TARGET_SKIP); // scry 1: keep on top

        setStrictChooseMode(true);
        setStopAt(2, PhaseStep.END_TURN);
        execute();

        assertLife(playerA, 40);
        assertGraveyardCount(playerB, "Lightning Bolt", 1);
        assertGraveyardCount(playerA, "Dissolve", 1);
        assertTappedCount("Commander's Sphere", true, 1);
    }

    @Test
    public void test_SplitSecondLeavesOnlyTheManaAbility() {
        addCard(Zone.COMMAND, playerA, "Kestia, the Cultivator", 1);
        addCard(Zone.BATTLEFIELD, playerA, "Commander's Sphere", 1);
        addCard(Zone.BATTLEFIELD, playerA, "Island", 2);
        addCard(Zone.HAND, playerA, "Dissolve", 1);

        addCard(Zone.BATTLEFIELD, playerB, "Forest", 3);
        addCard(Zone.HAND, playerB, "Krosan Grip", 1); // {2}{G} instant, split second, destroy artifact or enchantment

        castSpell(2, PhaseStep.PRECOMBAT_MAIN, playerB, "Krosan Grip", "Commander's Sphere");
        // split second: no spells and no abilities but mana abilities while Krosan Grip is on the stack
        checkPlayableAbility("no Dissolve", 2, PhaseStep.PRECOMBAT_MAIN, playerA, "Cast Dissolve", false);
        checkPlayableAbility("no sacrifice", 2, PhaseStep.PRECOMBAT_MAIN, playerA, "Sacrifice", false);
        checkPlayableAbility("mana ability", 2, PhaseStep.PRECOMBAT_MAIN, playerA, "{T}: Add one mana", true);

        setStrictChooseMode(true);
        setStopAt(2, PhaseStep.END_TURN);
        execute();

        assertGraveyardCount(playerA, "Commander's Sphere", 1);
        assertHandCount(playerA, "Dissolve", 1);
    }
}
