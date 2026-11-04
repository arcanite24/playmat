package org.mage.test.serverside.base;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import mage.cards.Card;
import mage.constants.PhaseStep;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.PutToBattlefieldInfo;
import mage.game.trace.GameTrace;
import mage.game.trace.StateFingerprint;
import org.mage.test.player.PlayerAction;
import org.mage.test.player.TestPlayer;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Writes a card test as JSON that another engine can run (Rust migration phase 0): the players, the cards each zone
 * starts with, the scripted actions, choices, targets and modes, where the game stops, and the state it reached (its
 * fingerprint text and trace, see docs/rust/StateFingerprint.md). The JUnit assertions themselves are Java; the end
 * state stands in for them, since it holds everything they read. Format: docs/rust/TestExport.md.
 * <p>
 * Off unless -Dxmage.exportTests=dir is set. Each execute() of a test writes dir/Class/method.json (method-2.json for a
 * second execute in the same test).
 */
public final class TestExport {

    static final String PROPERTY = "xmage.exportTests";
    static final int VERSION = 1;

    private static final Gson GSON = new GsonBuilder().setPrettyPrinting().disableHtmlEscaping().create();
    private static final Map<String, Integer> runsPerTest = new LinkedHashMap<>();

    private TestExport() {
    }

    public static boolean enabled() {
        String dir = System.getProperty(PROPERTY);
        return dir != null && !dir.isEmpty();
    }

    public static final class Export {
        int version = VERSION;
        String test;
        String gameType;
        int stopOnTurn;
        String stopAtStep;
        String activePlayer;
        List<PlayerScript> players = new ArrayList<>();
        /** false when the test runs Java code mid-game (runCode, custom checks): the export can't say all it does */
        boolean complete = true;
        Result result;
    }

    static final class PlayerScript {
        String name;
        boolean strictChooseMode;
        Map<String, String> commands = new LinkedHashMap<>();
        /** the whole library as the game starts (the deck the player was made with, then cards the test added), top first */
        List<String> startLibrary = new ArrayList<>();
        List<CardSpec> library = new ArrayList<>();
        List<CardSpec> hand = new ArrayList<>();
        List<CardSpec> battlefield = new ArrayList<>();
        List<CardSpec> graveyard = new ArrayList<>();
        List<CardSpec> exile = new ArrayList<>();
        List<CardSpec> command = new ArrayList<>();
        List<Action> actions = new ArrayList<>();
        List<String> choices = new ArrayList<>();
        List<String> targets = new ArrayList<>();
        List<String> modes = new ArrayList<>();
    }

    static final class CardSpec {
        String name;
        String set;
        String number;
        Boolean tapped;

        static CardSpec of(Card card, Boolean tapped) {
            CardSpec spec = new CardSpec();
            spec.name = card.getName();
            spec.set = card.getExpansionSetCode();
            spec.number = card.getCardNumber();
            spec.tapped = tapped;
            return spec;
        }
    }

    static final class Action {
        int turn;
        String step;
        String action;
        /** a Java check or code block runs here; the action string alone doesn't say what it does */
        Boolean code;
    }

    static final class Result {
        int turn;
        String step;
        String winner;
        String fingerprint;
        String state;
        List<String> trace;
    }

    /** the script as it stands just before the game starts */
    public static Export before(MageTestPlayerBase test, Game game, List<TestPlayer> players, int stopOnTurn, PhaseStep stopAt,
                         mage.players.Player active) {
        Export export = new Export();
        export.test = testName();
        export.gameType = game.getClass().getSimpleName();
        export.stopOnTurn = stopOnTurn;
        export.stopAtStep = stopAt.name();
        export.activePlayer = active.getName();
        for (TestPlayer player : players) {
            PlayerScript script = new PlayerScript();
            script.name = player.getName();
            script.strictChooseMode = player.getStrictChooseMode();
            for (Map.Entry<Zone, String> entry : test.getCommands(player).entrySet()) {
                script.commands.put(entry.getKey().name(), entry.getValue());
            }
            for (java.util.UUID id : player.getLibrary().getCardList()) {
                Card card = game.getCard(id);
                script.startLibrary.add(card == null ? "?" : card.getName());
            }
            test.getLibraryCards(player).forEach(c -> script.library.add(CardSpec.of(c, null)));
            test.getHandCards(player).forEach(c -> script.hand.add(CardSpec.of(c, null)));
            for (PutToBattlefieldInfo info : test.getBattlefieldCards(player)) {
                script.battlefield.add(CardSpec.of(info.getCard(), info.isTapped()));
            }
            test.getGraveCards(player).forEach(c -> script.graveyard.add(CardSpec.of(c, null)));
            test.getExiledCards(player).forEach(c -> script.exile.add(CardSpec.of(c, null)));
            test.getCommandCards(player).forEach(c -> script.command.add(CardSpec.of(c, null)));
            for (PlayerAction playerAction : player.getActions()) {
                Action action = new Action();
                action.turn = playerAction.getTurnNum();
                action.step = playerAction.getStep().name();
                action.action = playerAction.getAction();
                if (playerAction.getCodePayload() != null) {
                    action.code = true;
                    export.complete = false;
                }
                script.actions.add(action);
            }
            script.choices.addAll(player.getChoices());
            script.targets.addAll(player.getTargets());
            script.modes.addAll(player.getModeChoices());
            export.players.add(script);
        }
        return export;
    }

    /** the state the game reached, then the file */
    public static void after(Export export, Game game, GameTrace trace) {
        Result result = new Result();
        result.turn = game.getTurnNum();
        result.step = game.getTurnStepType() == null ? "-" : game.getTurnStepType().name();
        result.winner = game.getWinner();
        result.state = StateFingerprint.text(game);
        result.fingerprint = StateFingerprint.hashText(result.state);
        result.trace = trace == null ? null : trace.hashes();
        export.result = result;
        write(export);
    }

    private static void write(Export export) {
        String name = export.test;
        int run;
        synchronized (runsPerTest) {
            run = runsPerTest.merge(name, 1, Integer::sum);
        }
        int dot = name.lastIndexOf('.');
        String className = dot < 0 ? "Unknown" : name.substring(0, dot);
        String method = (dot < 0 ? name : name.substring(dot + 1)) + (run > 1 ? "-" + run : "");
        Path path = Paths.get(System.getProperty(PROPERTY), className, method + ".json");
        try {
            Files.createDirectories(path.getParent());
            Files.write(path, GSON.toJson(export).getBytes(StandardCharsets.UTF_8));
        } catch (IOException e) {
            throw new IllegalStateException("Can't write test export " + path, e);
        }
    }

    /** Class.method of the test being run: the first frame below the framework's own classes */
    private static String testName() {
        for (StackTraceElement frame : Thread.currentThread().getStackTrace()) {
            String className = frame.getClassName();
            if (className.startsWith("org.mage.test.") && !className.startsWith("org.mage.test.serverside.base.")
                    && !className.startsWith("org.mage.test.player.")) {
                return className + "." + frame.getMethodName();
            }
        }
        return "Unknown.unknown";
    }
}
