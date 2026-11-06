package mage.player.ai;

import mage.game.Game;
import mage.game.trace.DecisionLog;
import mage.game.trace.RustAiBridge;
import mage.player.human.HumanPlayer;
import mage.player.human.PlayerResponse;
import mage.players.PlayerImpl;
import mage.players.net.UserData;

/**
 * The Java AI's player with its dialogs answered by the Rust AI (docs/rust/Ai.md): the same dialogs a human player
 * gets, with the client's default settings, which is what the Rust copy of the game asks. Made for each dialog, like
 * the fake human of a controlled player ({@code HumanPlayer.prepareControllableProxy}).
 */
public class RustHumanPlayer extends HumanPlayer {

    private final transient PlayerResponse answers;
    private final transient RustAiBridge bridge;

    public RustHumanPlayer(PlayerImpl player, RustAiBridge bridge) {
        this(player, new PlayerResponse(), bridge);
    }

    private RustHumanPlayer(PlayerImpl player, PlayerResponse answers, RustAiBridge bridge) {
        super(player, answers);
        this.answers = answers;
        this.bridge = bridge;
        setUserData(UserData.getDefaultUserDataView());
    }

    @Override
    protected void waitForResponse(Game game) {
        answers.resetAnswers();
        DecisionLog.Answer a = bridge.ask(game);
        if (a == null) {
            return;
        }
        answers.setUUID(a.target);
        answers.setBoolean(a.bool);
        answers.setInteger(a.integer);
        answers.setString(a.string);
        answers.setManaType(a.manaType);
        answers.setResponseManaPlayerId(a.manaPlayer);
    }

    /** once the bridge stops, the dialog ends and the Java AI plays on */
    @Override
    public boolean canRespond() {
        return super.canRespond() && bridge.isActive();
    }
}
