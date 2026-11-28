package org.mage.test.view;

import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.view.CardView;
import mage.view.GameView;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

/**
 * A double-faced card shows its other face wherever it is looked at: on the stack while it's being cast (#169) and in
 * the deck builder, which shows cards from the database (#163), not only on the battlefield.
 *
 * @author arcanite24
 */
public class DoubleFacedBackFaceViewTest extends CardTestPlayerBase {

    @Test
    public void test_TransformingSpellOnStackHasBackFace() {
        // Thing in the Ice {1}{U}, transforms into Awoken Horror
        addCard(Zone.HAND, playerA, "Thing in the Ice");
        addCard(Zone.BATTLEFIELD, playerA, "Island", 2);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Thing in the Ice");
        runCode("back face on the stack", 1, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) -> {
            GameView gameView = getGameView(player);
            Assert.assertEquals(1, gameView.getStack().size());
            CardView spell = gameView.getStack().values().iterator().next();
            Assert.assertTrue("a transforming spell can be turned over", spell.canTransform());
            Assert.assertNotNull("the stack view carries the back face", spell.getSecondCardFace());
            Assert.assertEquals("Awoken Horror", spell.getSecondCardFace().getName());
        });

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();
    }

    @Test
    public void test_ModalSpellOnStackHasOtherFace() {
        // Valakut Awakening {2}{R} // Valakut Stoneforge (land)
        addCard(Zone.HAND, playerA, "Valakut Awakening");
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 3);

        castSpell(1, PhaseStep.PRECOMBAT_MAIN, playerA, "Valakut Awakening");
        runCode("other face on the stack", 1, PhaseStep.PRECOMBAT_MAIN, playerA, (info, player, game) -> {
            CardView spell = getGameView(player).getStack().values().iterator().next();
            Assert.assertNotNull(spell.getSecondCardFace());
            Assert.assertEquals("Valakut Stoneforge", spell.getSecondCardFace().getName());
        });

        setStopAt(1, PhaseStep.END_TURN);
        execute();
    }

    @Test
    public void test_DatabaseCardHasBackFace() {
        CardInfo info = CardRepository.instance.findCards("The Long Reach of Night").stream().findFirst().orElse(null);
        Assert.assertNotNull(info);
        CardView view = new CardView(info.createMockCard());
        Assert.assertTrue(view.canTransform());
        Assert.assertNotNull("deck building shows the back face", view.getSecondCardFace());
        Assert.assertEquals("Animus of Night's Reach", view.getSecondCardFace().getName());
    }
}
