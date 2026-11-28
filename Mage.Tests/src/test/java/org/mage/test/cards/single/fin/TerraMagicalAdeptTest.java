package org.mage.test.cards.single.fin;

import mage.abilities.keyword.HasteAbility;
import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.game.permanent.Permanent;
import mage.game.permanent.PermanentToken;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.serverside.base.CardTestPlayerBase;

import java.util.List;
import java.util.stream.Collectors;

/**
 * @author arcanite24
 */
public class TerraMagicalAdeptTest extends CardTestPlayerBase {

    @Test
    public void testCopyOfEsperTerraTokenDoesNotCopyHaste() {
        addCard(Zone.BATTLEFIELD, playerA, "Terra, Magical Adept");
        addCard(Zone.BATTLEFIELD, playerA, "Nyxbloom Ancient");
        addCard(Zone.BATTLEFIELD, playerA, "Summon: Good King Mog XII");
        addCard(Zone.BATTLEFIELD, playerA, "Mountain", 1);
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 1);
        addCard(Zone.HAND, playerA, "Tormod's Crypt");

        // Turn 3: Let Summon: Good King Mog XII's Chapter III trigger resolve (sets up delayed trigger).
        waitStackResolved(3, PhaseStep.PRECOMBAT_MAIN);
        // Transform Terra into Esper Terra -> Chapter I creates a token copy of Nyxbloom Ancient and it gains haste.
        activateAbility(3, PhaseStep.PRECOMBAT_MAIN, playerA, "<i>Trance</i>");
        addTarget(playerA, "Nyxbloom Ancient");

        // Cast a noncreature spell to trigger Good King Mog's Chapter III delayed trigger and copy the Nyxbloom Ancient token.
        waitStackResolved(3, PhaseStep.PRECOMBAT_MAIN);
        castSpell(3, PhaseStep.PRECOMBAT_MAIN, playerA, "Tormod's Crypt");
        setChoice(playerA, "Nyxbloom Ancient");

        setStrictChooseMode(true);
        setStopAt(3, PhaseStep.BEGIN_COMBAT);
        execute();

        assertPermanentCount(playerA, "Esper Terra", 1);
        assertPermanentCount(playerA, "Nyxbloom Ancient", 3);

        List<Permanent> nyxbloomTokens = currentGame.getBattlefield().getAllActivePermanents(playerA.getId()).stream()
                .filter(p -> p.getName().equals("Nyxbloom Ancient") && p instanceof PermanentToken)
                .collect(Collectors.toList());
        Assert.assertEquals("Should have 2 Nyxbloom Ancient tokens", 2, nyxbloomTokens.size());

        long tokensWithHaste = nyxbloomTokens.stream()
                .filter(p -> p.getAbilities(currentGame).containsClass(HasteAbility.class))
                .count();
        Assert.assertEquals("Only the token created by Esper Terra should have haste, not the copy of it", 1, tokensWithHaste);

        long tokensThatCanAttack = nyxbloomTokens.stream()
                .filter(p -> p.canAttack(playerB.getId(), currentGame))
                .count();
        Assert.assertEquals("Only the token with haste should be able to attack this turn", 1, tokensThatCanAttack);
    }
}
