package org.mage.test.playmat;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * Repro tests for rules reports from Playmat players.
 *
 * @author arcanite24
 */
public class PlaymatIssuesTest extends CardTestPlayerBase {

    /** #161: Dryad of the Ilysian Grove lets you play an additional land on each of your turns. */
    @Test
    public void test_DryadOfTheIlysianGrove_SecondLand() {
        addCard(Zone.BATTLEFIELD, playerA, "Dryad of the Ilysian Grove");
        addCard(Zone.HAND, playerA, "Forest", 2);

        playLand(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Forest");
        checkPlayableAbility("second land", 1, PhaseStep.PRECOMBAT_MAIN, playerA, "Play Forest", true);
        playLand(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Forest");

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertPermanentCount(playerA, "Forest", 2);
    }

    /** #162: with Leyline of Mutation and Leyline of the Guildpact, five Forests pay {W}{U}{B}{R}{G} for a 7-mana spell. */
    @Test
    public void test_LeylineOfMutation_FiveLandsCastSevenDrop() {
        addCard(Zone.BATTLEFIELD, playerA, "Leyline of Mutation");
        addCard(Zone.BATTLEFIELD, playerA, "Leyline of the Guildpact");
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 5);
        addCard(Zone.HAND, playerA, "Pelakka Wurm"); // {4}{G}{G}{G}

        checkPlayableAbility("castable for WUBRG", 1, PhaseStep.PRECOMBAT_MAIN, playerA, "Cast Pelakka Wurm", true);
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Pelakka Wurm");
        setChoice(playerA, "Cast with alternative cost: {W}{U}{B}{R}{G} (source: Leyline of Mutation");
        // each Forest is every basic land type: it asks which color to add (not strict, so it picks what's needed)

        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertPermanentCount(playerA, "Pelakka Wurm", 1);
    }

    /** #140: Maelstrom Archangel's combat damage trigger casts the chosen card for free. */
    @Test
    public void test_MaelstromArchangel_CastsChosenCard() {
        addCard(Zone.BATTLEFIELD, playerA, "Maelstrom Archangel");
        addCard(Zone.HAND, playerA, "Grizzly Bears");

        attack(3, playerA, "Maelstrom Archangel");
        // the only nonland card in hand: picked without asking
        setChoice(playerA, "Yes");

        setStrictChooseMode(true);
        setStopAt(3, PhaseStep.END_TURN);
        execute();

        assertLife(playerB, 20 - 5);
        assertPermanentCount(playerA, "Grizzly Bears", 1);
        assertGraveyardCount(playerA, "Grizzly Bears", 0);
    }
}
