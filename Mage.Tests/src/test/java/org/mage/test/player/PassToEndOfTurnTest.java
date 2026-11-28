package org.mage.test.player;

import mage.cards.decks.Deck;
import mage.constants.MultiplayerAttackOption;
import mage.constants.PlayerAction;
import mage.constants.RangeOfInfluence;
import mage.game.Game;
import mage.game.TwoPlayerDuel;
import mage.game.mulligan.LondonMulligan;
import mage.game.turn.EndPhase;
import mage.game.turn.EndStep;
import mage.game.turn.Phase;
import mage.game.turn.PostCombatMainPhase;
import mage.game.turn.PostCombatMainStep;
import mage.game.turn.PreCombatMainPhase;
import mage.game.turn.PreCombatMainStep;
import mage.game.turn.Step;
import mage.player.human.HumanPlayer;
import mage.players.net.UserData;
import org.junit.Assert;
import org.junit.Test;

import java.util.concurrent.atomic.AtomicInteger;

/**
 * "Pass to end of turn" (PASS_PRIORITY_UNTIL_TURN_END_STEP, the F4 / pass-ahead menu item) is for the turn it was
 * pressed in. Pressed in the end step of the player before you, it used to wait for the next end step, your own: your
 * whole turn went by without a stop, first main phase included.
 */
public class PassToEndOfTurnTest {

    private static final class RecordingHumanPlayer extends HumanPlayer {
        final AtomicInteger prompts = new AtomicInteger();

        RecordingHumanPlayer(String name) {
            super(name, RangeOfInfluence.ALL, 5);
            // the web client's defaults: stops at your main phases and at the opponent's end step
            UserData userData = UserData.getDefaultUserDataView();
            userData.getUserSkipPrioritySteps().getOpponentTurn().setEndOfTurn(true);
            setUserData(userData);
        }

        @Override
        protected void prepareForResponse(Game game) {
            // no game thread in a direct unit test
        }

        @Override
        protected void waitForResponse(Game game) {
            prompts.incrementAndGet();
            skip();
        }
    }

    private RecordingHumanPlayer me;
    private RecordingHumanPlayer opponent;

    private TwoPlayerDuel game() {
        TwoPlayerDuel game = new TwoPlayerDuel(MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL, new LondonMulligan(0), 60, 20, 7);
        me = new RecordingHumanPlayer("Me");
        opponent = new RecordingHumanPlayer("Opponent");
        game.addPlayer(me, new Deck());
        game.addPlayer(opponent, new Deck());
        return game;
    }

    private static void at(Game game, RecordingHumanPlayer active, Phase phase, Step step) {
        game.getState().getPlayerList().setCurrent(active.getId());
        game.getState().setActivePlayerId(active.getId());
        phase.setStep(step);
        game.getState().getTurn().setPhase(phase);
    }

    /** the game's end of turn, for every player (GameImpl.endOfTurn) */
    private void endTurn(Game game) {
        me.endOfTurn(game);
        opponent.endOfTurn(game);
    }

    private int promptsAt(Game game) {
        int before = me.prompts.get();
        me.resetPassed();
        me.priority(game);
        return me.prompts.get() - before;
    }

    @Test
    public void pressedInTheOpponentsEndStepTheNextTurnStillStopsAtMain1() {
        TwoPlayerDuel game = game();
        at(game, opponent, new EndPhase(), new EndStep());
        me.sendPlayerAction(PlayerAction.PASS_PRIORITY_UNTIL_TURN_END_STEP, game, null);
        Assert.assertEquals("the end step it was pressed in is passed", 0, promptsAt(game));

        endTurn(game);
        Assert.assertFalse("the pass ends with its turn", me.getPassedUntilEndOfTurn());

        at(game, me, new PreCombatMainPhase(), new PreCombatMainStep());
        Assert.assertEquals("my own first main phase stops", 1, promptsAt(game));
    }

    @Test
    public void pressedMidTurnItStillStopsAtThatTurnsEndStep() {
        TwoPlayerDuel game = game();
        at(game, opponent, new PostCombatMainPhase(), new PostCombatMainStep());
        me.sendPlayerAction(PlayerAction.PASS_PRIORITY_UNTIL_TURN_END_STEP, game, null);
        Assert.assertEquals("the rest of the turn is passed", 0, promptsAt(game));

        at(game, opponent, new EndPhase(), new EndStep());
        Assert.assertEquals("it stops at the end step", 1, promptsAt(game));
        Assert.assertFalse(me.getPassedUntilEndOfTurn());
    }

    @Test
    public void pressedOnMyOwnTurnTheOpponentsTurnStopsAsUsual() {
        TwoPlayerDuel game = game();
        at(game, me, new EndPhase(), new EndStep());
        me.sendPlayerAction(PlayerAction.PASS_PRIORITY_UNTIL_TURN_END_STEP, game, null);
        Assert.assertEquals(0, promptsAt(game));

        endTurn(game);
        Assert.assertFalse(me.getPassedUntilEndOfTurn());
        at(game, opponent, new EndPhase(), new EndStep());
        Assert.assertEquals("the opponent's end step stops again", 1, promptsAt(game));
    }
}
