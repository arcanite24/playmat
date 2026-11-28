package org.mage.test.cards.single.tdc;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.counters.CounterType;
import org.junit.Test;
import org.mage.test.player.TestPlayer;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * @author Susucr
 */
public class BetorAncestorsVoiceTest extends CardTestPlayerBase {

    /**
     * {@link mage.cards.b.BetorAncestorsVoice Betor, Ancestor's Voice} {2}{W}{B}{G}
     * Legendary Creature — Spirit Dragon
     * Flying, lifelink
     * At the beginning of your end step, put a number of +1/+1 counters on up to one other target creature you control equal to the amount of life you gained this turn. Return up to one target creature card with mana value less than or equal to the amount of life you lost this turn from your graveyard to the battlefield.
     * 3/5
     */
    private static final String betor = "Betor, Ancestor's Voice";

    // Bug Report: If you got Betor and Rhox Faithmender on the field, and attack with both.
    // The gui shows you got the 8 life you are supposed to get, but in the end of turn trigger,
    // there will be only 4 counters put on a creature.
    @Test
    public void test_RhowFaithmender() {
        addCard(Zone.BATTLEFIELD, playerA, betor, 1);
        addCard(Zone.BATTLEFIELD, playerA, "Rhox Faithmender", 1);

        attack(1, playerA, betor, playerB);
        attack(1, playerA, "Rhox Faithmender", playerB);

        addTarget(playerA, "Rhox Faithmender"); // Betor trigger's first target
        addTarget(playerA, TestPlayer.TARGET_SKIP); // second target

        setStrictChooseMode(true);
        setStopAt(2, PhaseStep.PRECOMBAT_MAIN);
        execute();

        assertLife(playerA, 20 + 8);
        assertCounterCount(playerA, "Rhox Faithmender", CounterType.P1P1, 8);
    }

    @Test
    public void test_OnlyTargetsYourGraveyard() {
        addCard(Zone.BATTLEFIELD, playerA, betor, 1);
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 1);
        addCard(Zone.HAND, playerA, "Shock", 1);
        addCard(Zone.GRAVEYARD, playerA, "Grizzly Bears", 1);
        addCard(Zone.GRAVEYARD, playerB, "Runeclaw Bear", 1);

        // Player A loses 2 life so MV <= 2 creatures are eligible
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Shock", playerA);

        addTarget(playerA, TestPlayer.TARGET_SKIP); // first target: no other creature to put counters on
        addTarget(playerA, "Grizzly Bears"); // second target: must only allow own graveyard

        setStrictChooseMode(true);
        setStopAt(2, PhaseStep.PRECOMBAT_MAIN);
        execute();

        assertPermanentCount(playerA, "Grizzly Bears", 1);
        assertGraveyardCount(playerB, "Runeclaw Bear", 1);
    }

    @Test
    public void test_CannotTargetOpponentsGraveyard() {
        addCard(Zone.BATTLEFIELD, playerA, betor, 1);
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 1);
        addCard(Zone.HAND, playerA, "Shock", 1);
        addCard(Zone.GRAVEYARD, playerB, "Runeclaw Bear", 1);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Shock", playerA);

        addTarget(playerA, TestPlayer.TARGET_SKIP); // first target: no other creature player A controls
        addTarget(playerA, "Runeclaw Bear"); // illegal target: in opponent's graveyard

        setStrictChooseMode(true);
        setStopAt(2, PhaseStep.PRECOMBAT_MAIN);
        try {
            execute();
            org.junit.Assert.fail("Betor should not be able to target a creature card in an opponent's graveyard");
        } catch (AssertionError expected) {
            // Expected: Runeclaw Bear in player B's graveyard is not a legal target
        }

        assertGraveyardCount(playerB, "Runeclaw Bear", 1);
        assertPermanentCount(playerA, "Runeclaw Bear", 0);
    }
}
