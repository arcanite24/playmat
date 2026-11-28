package org.mage.test.cards.mana;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * Colorless hybrid symbols ({C/U}): {C} pays any of them, a color only its own. Ulalek, Fused Atrocity costs
 * {C/W}{C/U}{C/B}{C/R}{C/G}, so {C}{C}{C}{C}{U} pays it only if the {U} gets {C/U}.
 */
public class ColorlessHybridManaTest extends CardTestPlayerBase {

    private static final String ULALEK = "Ulalek, Fused Atrocity";

    @Test
    public void testAutoTap() {
        addCard(Zone.BATTLEFIELD, playerA, "Wastes", 4);
        addCard(Zone.BATTLEFIELD, playerA, "Island", 1);
        addCard(Zone.HAND, playerA, ULALEK);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, ULALEK);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();

        assertPermanentCount(playerA, ULALEK, 1);
    }

    @Test
    public void testAllManaInPool() {
        addCard(Zone.BATTLEFIELD, playerA, "Wastes", 4);
        addCard(Zone.BATTLEFIELD, playerA, "Island", 1);
        addCard(Zone.HAND, playerA, ULALEK);

        activateManaAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "{T}: Add {C}", 4);
        activateManaAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "{T}: Add {U}", 1);
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, ULALEK);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();

        assertPermanentCount(playerA, ULALEK, 1);
    }

    @Test
    public void testColorlessInPoolColorTappedLater() {
        // the four {C} pay four symbols before the Island is tapped: its {U} has to take {C/U} over from a {C}
        addCard(Zone.BATTLEFIELD, playerA, "Wastes", 4);
        addCard(Zone.BATTLEFIELD, playerA, "Island", 1);
        addCard(Zone.HAND, playerA, ULALEK);

        activateManaAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "{T}: Add {C}", 4);
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, ULALEK);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();

        assertPermanentCount(playerA, ULALEK, 1);
        assertTappedCount("Island", true, 1);
    }

    @Test
    public void testTwoColorsAndColorless() {
        // {U} and {G} pay their own symbols, {C}{C}{C} the other three
        addCard(Zone.BATTLEFIELD, playerA, "Wastes", 3);
        addCard(Zone.BATTLEFIELD, playerA, "Island", 1);
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 1);
        addCard(Zone.HAND, playerA, ULALEK);

        activateManaAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "{T}: Add {C}", 3);
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, ULALEK);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();

        assertPermanentCount(playerA, ULALEK, 1);
    }

    @Test
    public void testTwoColorsAllInPool() {
        // the reported game: {U}{B}{C}{C}{C} floating, the {C} must leave {C/U} and {C/B} to the colors
        addCard(Zone.BATTLEFIELD, playerA, "Wastes", 3);
        addCard(Zone.BATTLEFIELD, playerA, "Island", 1);
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 1);
        addCard(Zone.HAND, playerA, ULALEK);

        activateManaAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "{T}: Add {U}", 1);
        activateManaAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "{T}: Add {B}", 1);
        activateManaAbility(1, PhaseStep.PRECOMBAT_MAIN, playerA, "{T}: Add {C}", 3);
        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, ULALEK);

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.BEGIN_COMBAT);
        execute();

        assertPermanentCount(playerA, ULALEK, 1);
    }
}
