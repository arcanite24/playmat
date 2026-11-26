package org.mage.test.cards.single.fic;

import mage.constants.PhaseStep;
import mage.constants.Zone;
import org.junit.Test;
import org.mage.test.player.TestPlayer;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * @author Susucr
 */
public class SummonEsperValigarmandaTest extends CardTestPlayerBase {

    /**
     * {@link mage.cards.s.SummonEsperValigarmanda Summon: Esper Valigarmanda} {3}{R}
     * Enchantment Creature — Saga Drake
     * I — Exile an instant or sorcery card from each graveyard.
     * II, III, IV — Add {R} for each lore counter on this Saga. You may cast an instant or sorcery card exiled with this Saga, and mana of any type can be spent to cast that spell.
     * Flying, haste
     */
    private static final String esper = "Summon: Esper Valigarmanda";

    @Test
    public void test_SimplePlay() {
        addCard(Zone.HAND, playerA, esper, 1);
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 4);
        addCard(Zone.GRAVEYARD, playerB, "Lightning Bolt", 1);
        addCard(Zone.GRAVEYARD, playerA, "Shock", 1);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, esper);
        setChoice(playerA, "Shock");
        setChoice(playerA, "Lightning Bolt");

        // turn 3
        setChoice(playerA, "Shock"); // choose to be cast
        setChoice(playerA, true); // choose to cast
        addTarget(playerA, playerB);

        checkLife("T3: after bolt life", 3, PhaseStep.POSTCOMBAT_MAIN, playerB, 20 - 2);

        // turn 5
        setChoice(playerA, TestPlayer.CHOICE_SKIP);

        checkLife("T5: no cast", 5, PhaseStep.POSTCOMBAT_MAIN, playerB, 20 - 2);

        // turn 7
        setChoice(playerA, "Lightning Bolt");
        setChoice(playerA, true); // choose to cast
        addTarget(playerA, playerB);

        setStrictChooseMode(true);
        setStopAt(7, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertLife(playerB, 20 - 2 - 3);
    }

    /**
     * Reported: with only the saga's {R}{R} in the pool, red couldn't pay Stitch Together's {B}{B}. Red paid a {B}
     * only once a Swamp had already paid the other one.
     */
    private void setupStitchTogether() {
        addCard(Zone.HAND, playerA, esper, 1);
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 4);
        addCard(Zone.GRAVEYARD, playerA, "Stitch Together", 1); // {B}{B} sorcery
        addCard(Zone.GRAVEYARD, playerA, "Grizzly Bears", 1);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, esper);
        setChoice(playerA, "Stitch Together"); // chapter I exiles it

        // turn 3, chapter II: {R}{R} and cast Stitch Together
        setChoice(playerA, "Stitch Together");
        setChoice(playerA, true);
        addTarget(playerA, "Grizzly Bears");
    }

    private void assertStitchTogetherPaidWithRed() {
        assertHandCount(playerA, "Grizzly Bears", 1);
        assertExileCount(playerA, "Stitch Together", 0);
        assertGraveyardCount(playerA, "Stitch Together", 1);
        assertTappedCount("Mountain", false, 4); // paid from the pool only
    }

    @Test
    public void test_AnyType_TwoRedPayBlackBlack_AutoPayment() {
        setupStitchTogether();

        setStrictChooseMode(true);
        setStopAt(3, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertStitchTogetherPaidWithRed();
    }

    @Test
    public void test_AnyType_TwoRedPayBlackBlack_RestrictedAutoPayment() {
        setupStitchTogether();
        // the saga itself is cast with full auto-payment, then the pool is restricted for turn 3
        runCode("restrict auto-payment", 2, PhaseStep.END_TURN, playerA, (info, player, game) -> {
            player.getUserData().setManaPoolAutomaticRestricted(true);
            player.getManaPool().setAutoPaymentRestricted(true);
        });

        setStrictChooseMode(true);
        setStopAt(3, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertStitchTogetherPaidWithRed();
    }

    @Test
    public void test_AnyType_TwoRedPayBlackBlack_ManualPoolClicks() {
        setupStitchTogether();
        // the saga itself is cast with auto-payment, then mana is paid by clicks on the pool on turn 3
        runCode("disable auto-payment", 2, PhaseStep.END_TURN, playerA, (info, player, game) -> {
            disableManaAutoPayment(playerA);
        });
        setChoice(playerA, "Red");
        setChoice(playerA, "Red");

        setStrictChooseMode(true);
        setStopAt(3, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertStitchTogetherPaidWithRed();
    }

    @Test
    public void test_AnyType_SwampAndRedPayBlackBlack_ManualPoolClicks() {
        setupStitchTogether();
        addCard(Zone.BATTLEFIELD, playerA, "Swamp", 1);
        runCode("disable auto-payment", 2, PhaseStep.END_TURN, playerA, (info, player, game) -> {
            disableManaAutoPayment(playerA);
        });
        setChoice(playerA, "{T}: Add {B}"); // the Swamp
        setChoice(playerA, "Black");
        setChoice(playerA, "Red");

        setStrictChooseMode(true);
        setStopAt(3, PhaseStep.POSTCOMBAT_MAIN);
        execute();

        assertHandCount(playerA, "Grizzly Bears", 1);
        assertGraveyardCount(playerA, "Stitch Together", 1);
        assertTapped("Swamp", true);
        assertTappedCount("Mountain", false, 4);
    }
}
