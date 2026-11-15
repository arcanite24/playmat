package org.mage.test.serverside;

import mage.abilities.Ability;
import mage.abilities.costs.mana.ManaCost;
import mage.constants.PhaseStep;
import mage.constants.RangeOfInfluence;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.permanent.Permanent;
import mage.server.game.GameSessionPlayer;
import mage.view.GameView;
import org.junit.Assert;
import org.junit.Test;
import org.mage.test.player.TestComputerPlayer;
import org.mage.test.player.TestPlayer;
import org.mage.test.serverside.base.CardTestPlayerBase;

import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

/**
 * A player asked to pay mana while another player holds priority (Propaganda's or Archon of Absolution's attack tax
 * during declare attackers) must be shown their own mana sources as playable, or a client that only lets the player
 * click what the server lists can't pay at all.
 *
 * @author arcanite24
 */
public class PayManaPlayableTest extends CardTestPlayerBase {

    @Test
    public void test_PayingPlayerWithoutPrioritySeesOwnManaSources() {
        addCard(Zone.BATTLEFIELD, playerA, "Forest", 2);

        // playerB holds priority on playerA's turn
        runCode("check playable", 1, PhaseStep.PRECOMBAT_MAIN, playerB, (info, player, game) -> {
            Permanent forest = game.getBattlefield().getAllActivePermanents(playerA.getId()).stream()
                    .filter(permanent -> permanent.getName().equals("Forest"))
                    .findFirst()
                    .orElse(null);
            Assert.assertNotNull(forest);
            Assert.assertEquals(playerB.getId(), game.getPriorityPlayerId());

            GameView idle = getGameView(playerA);
            Assert.assertFalse("without priority or a payment, nothing is playable",
                    idle.getCanPlayObjects() != null && idle.getCanPlayObjects().containsObject(forest.getId()));

            playerA.setPayManaMode(true);
            try {
                GameView paying = getGameView(playerA);
                Assert.assertNotNull(paying.getCanPlayObjects());
                Assert.assertTrue("a player paying mana must see their lands as playable",
                        paying.getCanPlayObjects().containsObject(forest.getId()));

                GameView other = getGameView(playerB);
                Assert.assertFalse("the priority player is not shown the payer's lands",
                        other.getCanPlayObjects() != null && other.getCanPlayObjects().containsObject(forest.getId()));
            } finally {
                playerA.setPayManaMode(false);
            }
        });

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();
    }

    /**
     * A player sees the views a real game sends them, through one session's cache: the question to declare attackers
     * (nothing is playable then), then the attack tax's mana payment. Each question gives the asking player priority,
     * as HumanPlayer.prepareForResponse does.
     */
    private static class ViewingPlayer extends TestPlayer {

        final GameSessionPlayer.PlayableCache cache = new GameSessionPlayer.PlayableCache();
        final List<GameView> attackViews = new ArrayList<>();
        final List<GameView> payViews = new ArrayList<>();

        ViewingPlayer(TestComputerPlayer computerPlayer) {
            super(computerPlayer);
        }

        private GameView view(Game game) {
            game.getState().setPriorityPlayerId(getId());
            return GameSessionPlayer.prepareGameView(game, getId(), getId(), cache);
        }

        @Override
        public void selectAttackers(Game game, UUID attackingPlayerId) {
            attackViews.add(view(game));
            super.selectAttackers(game, attackingPlayerId);
        }

        @Override
        public boolean playMana(Ability ability, ManaCost unpaid, String promptText, Game game) {
            setPayManaMode(true);
            try {
                payViews.add(view(game));
            } finally {
                setPayManaMode(false);
            }
            return super.playMana(ability, unpaid, promptText, game);
        }
    }

    @Override
    protected TestPlayer createNewPlayer(String playerName, RangeOfInfluence rangeOfInfluence) {
        if (playerName.equals("PlayerA")) {
            return new ViewingPlayer(new TestComputerPlayer(playerName, rangeOfInfluence));
        }
        return super.createNewPlayer(playerName, rangeOfInfluence);
    }

    /**
     * Sphere of Safety and Dain, Lord of the Iron Hills (#398): the attack tax is asked right after the question to
     * declare attackers, in the same game state. The playable list cached for that question (empty: nothing is
     * playable while attackers are declared) must not be reused for the payment.
     */
    @Test
    public void test_AttackTaxPaymentListsSourcesAfterDeclaringAttackers() {
        // Creatures can't attack you or planeswalkers you control unless their controller pays {X} for each of those
        // creatures, where X is the number of enchantments you control.
        addCard(Zone.BATTLEFIELD, playerB, "Sphere of Safety");
        addCard(Zone.BATTLEFIELD, playerA, "Silvercoat Lion");
        addCard(Zone.BATTLEFIELD, playerA, "Plains", 2);

        attack(1, playerA, "Silvercoat Lion");
        setChoice(playerA, true); // Pay {1} to attack?

        setStrictChooseMode(true);
        setStopAt(1, PhaseStep.END_TURN);
        execute();

        assertLife(playerB, 20 - 2);
        assertTappedCount("Plains", true, 1);

        ViewingPlayer viewer = (ViewingPlayer) playerA;
        Assert.assertFalse("asked to declare attackers", viewer.attackViews.isEmpty());
        Assert.assertFalse("asked to pay the tax", viewer.payViews.isEmpty());
        UUID plains = currentGame.getBattlefield().getAllActivePermanents(playerA.getId()).stream()
                .filter(permanent -> permanent.getName().equals("Plains"))
                .map(Permanent::getId)
                .findFirst()
                .orElse(null);
        Assert.assertNotNull(plains);
        GameView paying = viewer.payViews.get(0);
        Assert.assertTrue("the attack tax payment must list the player's lands",
                paying.getCanPlayObjects() != null && paying.getCanPlayObjects().containsObject(plains));
    }
}
