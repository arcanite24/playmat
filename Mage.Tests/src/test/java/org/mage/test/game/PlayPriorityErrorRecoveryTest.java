package org.mage.test.game;

import mage.cards.Card;
import mage.cards.decks.Deck;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.constants.MultiplayerAttackOption;
import mage.constants.RangeOfInfluence;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.TwoPlayerDuel;
import mage.game.mulligan.LondonMulligan;
import mage.game.turn.PreCombatMainPhase;
import mage.game.turn.PreCombatMainStep;
import mage.player.ai.ComputerPlayer;
import mage.player.human.HumanPlayer;
import mage.players.net.UserData;
import org.junit.Assert;
import org.junit.Test;

import java.util.Collections;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * Regression tests for #424:
 * - A runtime error during priority must NOT revert earlier committed plays in the same phase (such as a land drop).
 * - A repeatedly failing non-human (AI) opponent must pass priority instead of trapping the game in an infinite
 *   rollback loop that terminates with a technical winner.
 * - endWithTechnicalWinner records a non-empty technical error reason for end-of-game reporting.
 */
public class PlayPriorityErrorRecoveryTest {

    private static final class ScriptedHuman extends HumanPlayer {
        private final Card landToPlay;
        private boolean playedLand = false;

        ScriptedHuman(String name, Card landToPlay) {
            super(name, RangeOfInfluence.ALL, 5);
            this.landToPlay = landToPlay;
            setTestMode(false);
        }

        @Override
        public boolean priority(Game game) {
            if (!playedLand && getHand().contains(landToPlay.getId())) {
                playedLand = true;
                return playLand(landToPlay, game, false);
            }
            pass(game);
            return false;
        }
    }

    private static final class ThrowingAi extends ComputerPlayer {
        final AtomicInteger attempts = new AtomicInteger();

        ThrowingAi(String name) {
            super(name, RangeOfInfluence.ALL);
        }

        @Override
        public boolean priority(Game game) {
            attempts.incrementAndGet();
            throw new IllegalStateException("simulated AI evaluation bug");
        }
    }

    private static final class TestableDuel extends TwoPlayerDuel {
        TestableDuel() {
            super(MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL, new LondonMulligan(0), 60, 20, 7);
        }

        void bindPlayerList() {
            this.playerList = state.getPlayerList();
        }
    }

    @Test
    public void testPriorityErrorDoesNotRevertEarlierLandPlayOrAbortGame() {
        TestableDuel game = new TestableDuel();
        CardInfo plainsInfo = CardRepository.instance.findCard("Plains");
        Assert.assertNotNull(plainsInfo);
        Card plains = plainsInfo.createCard();

        ScriptedHuman human = new ScriptedHuman("Human", plains);
        human.setUserData(UserData.getDefaultUserDataView());
        ThrowingAi ai = new ThrowingAi("BrokenAI");
        ai.setUserData(UserData.getDefaultUserDataView());
        game.addPlayer(human, new Deck());
        game.addPlayer(ai, new Deck());
        game.bindPlayerList();
        human.init(game);
        ai.init(game);

        game.loadCards(Collections.singleton(plains), human.getId());
        human.moveCards(plains, Zone.HAND, null, game);

        game.getState().setActivePlayerId(human.getId());
        game.getState().getPlayerList().setCurrent(human.getId());
        PreCombatMainPhase main1 = new PreCombatMainPhase();
        main1.setStep(new PreCombatMainStep());
        game.getState().getTurn().setPhase(main1);
        game.applyEffects();

        // Run a full priority round in Main 1:
        // 1) Human plays Plains (committed action -> rollback bookmark must refresh!)
        // 2) Human passes priority to AI
        // 3) AI throws on priority -> recovers without undoing Plains, passes AI priority after repeat error
        game.playPriority(human.getId(), false);

        Assert.assertFalse("Game must not end with technical winner when AI fails priority", game.hasEnded());
        Assert.assertNull("No technical error should have ended the game", game.getTechnicalErrorReason());
        Assert.assertEquals("Plains must stay on the battlefield (not reverted to hand)",
                1, game.getBattlefield().countAll(new mage.filter.FilterPermanent(), human.getId(), game));
        Assert.assertEquals("Plains must not be returned to hand", 0, human.getHand().size());
        Assert.assertEquals("AI should retry once then pass priority cleanly", 2, ai.attempts.get());
    }

    @Test
    public void testHumanActionErrorDoesNotRevertEarlierLandPlay() {
        TestableDuel game = new TestableDuel();
        CardInfo plainsInfo = CardRepository.instance.findCard("Plains");
        Assert.assertNotNull(plainsInfo);
        Card plains = plainsInfo.createCard();

        final AtomicInteger step = new AtomicInteger();
        HumanPlayer human = new HumanPlayer("Human", RangeOfInfluence.ALL, 5) {
            @Override
            public boolean priority(Game g) {
                int s = step.getAndIncrement();
                if (s == 0) {
                    return playLand(plains, g, false);
                }
                if (s == 1) {
                    throw new IllegalStateException("simulated spell-cast bug");
                }
                pass(g);
                return false;
            }
        };
        human.setTestMode(false);
        human.setUserData(UserData.getDefaultUserDataView());
        ComputerPlayer ai = new ComputerPlayer("AI", RangeOfInfluence.ALL);
        ai.setUserData(UserData.getDefaultUserDataView());
        game.addPlayer(human, new Deck());
        game.addPlayer(ai, new Deck());
        game.bindPlayerList();
        human.init(game);
        ai.init(game);

        game.loadCards(Collections.singleton(plains), human.getId());
        human.moveCards(plains, Zone.HAND, null, game);

        game.getState().setActivePlayerId(human.getId());
        game.getState().getPlayerList().setCurrent(human.getId());
        PreCombatMainPhase main1 = new PreCombatMainPhase();
        main1.setStep(new PreCombatMainStep());
        game.getState().getTurn().setPhase(main1);
        game.applyEffects();

        game.playPriority(human.getId(), false);

        Assert.assertFalse("Game must continue after recovering from action error", game.hasEnded());
        Assert.assertEquals("Land played before the failing action must stay on the battlefield",
                1, game.getBattlefield().countAll(new mage.filter.FilterPermanent(), human.getId(), game));
        Assert.assertEquals("Land must not revert back to hand", 0, human.getHand().size());
    }

    @Test
    public void testEndWithTechnicalWinnerRecordsErrorReason() {
        TwoPlayerDuel game = new TwoPlayerDuel(
                MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL, new LondonMulligan(0), 60, 20, 7
        );
        HumanPlayer p1 = new HumanPlayer("P1", RangeOfInfluence.ALL, 5);
        HumanPlayer p2 = new HumanPlayer("P2", RangeOfInfluence.ALL, 5);
        game.addPlayer(p1, new Deck());
        game.addPlayer(p2, new Deck());

        game.endWithTechnicalWinner("Too many errors");

        Assert.assertTrue(game.hasEnded());
        Assert.assertEquals("Too many errors", game.getTechnicalErrorReason());
    }
}
