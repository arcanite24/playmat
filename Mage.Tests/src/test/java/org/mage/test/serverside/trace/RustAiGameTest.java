package org.mage.test.serverside.trace;

import mage.cards.decks.DeckCardLists;
import mage.cards.decks.importer.DeckImporter;
import mage.cards.repository.CardScanner;
import mage.constants.MultiplayerAttackOption;
import mage.constants.PhaseStep;
import mage.constants.RangeOfInfluence;
import mage.game.Game;
import mage.game.GameOptions;
import mage.game.TwoPlayerDuel;
import mage.game.match.Match;
import mage.game.match.MatchOptions;
import mage.game.mulligan.MulliganType;
import mage.game.trace.DecisionLog;
import mage.game.trace.GameTrace;
import mage.game.trace.RustAiBridge;
import mage.player.ai.ComputerPlayerControllableProxy;
import mage.player.human.HumanPlayer;
import mage.players.net.UserData;
import mage.util.RandomUtil;
import mage.util.ThreadUtils;
import mage.game.TwoPlayerMatch;
import org.junit.Assert;
import org.junit.Assume;
import org.junit.Test;

import java.io.File;
import java.util.*;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;

/**
 * The Rust AI playing the Java AI's seat (docs/rust/Ai.md): the Pioneer and Standard school decks, a human seat
 * answered by the seeded {@link RandomDriver}, the AI's seat through {@code playmat-ai-bridge}. Every game must keep the
 * Rust copy in step with Java to the end, with the Rust AI answering.
 * <p>
 * Skipped unless the bridge is named. Build it with {@code cargo build --release -p playmat-ai} in rust/, then:
 * <pre>
 * mvn -pl Mage.Tests test -Dtest=RustAiGameTest -Dxmage.rustAi.bin=$PWD/rust/target/release/playmat-ai-bridge \
 *     -Dxmage.dataCollectors.printGameLogs=false -Dlog4j.configuration=file:$PWD/.travis/log4j.properties
 * </pre>
 * Options: xmage.rustAiGames.games (default 10), .first (default 0), .turns (default 20); .java=true plays the same
 * games with the Java AI alone, to compare (wins against the same random opponent, and time).
 */
public class RustAiGameTest {

    private static final String PREFIX = "xmage.rustAiGames";

    @Test
    public void rustAiPlaysTheAiSeat() throws Exception {
        boolean java = Boolean.getBoolean(PREFIX + ".java");
        Assume.assumeFalse("needs -Dxmage.rustAi.bin=<playmat-ai-bridge>",
                !java && System.getProperty("xmage.rustAi.bin", "").isEmpty());
        int games = Integer.getInteger(PREFIX + ".games", 10);
        int first = Integer.getInteger(PREFIX + ".first", 0);
        int turns = Integer.getInteger(PREFIX + ".turns", 20);
        CardScanner.scan();
        List<String> names = new ArrayList<>();
        List<DeckCardLists> decks = new ArrayList<>();
        for (String school : new String[]{"pioneer", "standard"}) {
            File[] files = new File("../Mage.Server/career-src", school).listFiles((d, name) -> name.endsWith(".txt"));
            Arrays.sort(files);
            for (File file : files) {
                decks.add(DeckImporter.importDeckFromFile(file.toString(), new StringBuilder(), false));
                names.add(school + "/" + file.getName().replace(".txt", ""));
            }
        }
        int n = decks.size();
        List<String> problems = new ArrayList<>();
        long answered = 0;
        int wins = 0;
        long started = System.nanoTime();
        for (int i = first; i < first + games; i++) {
            int a = i % n;
            int b = (a + 1 + (i / n) % (n - 1)) % n;
            long before = RustAiBridge.ANSWERED.get();
            String result = play(i, decks.get(a), decks.get(b), turns, problems, names.get(a) + " vs " + names.get(b), java);
            if (result.startsWith("AI won")) {
                wins++;
            }
            long now = RustAiBridge.ANSWERED.get() - before;
            answered += now;
            System.out.println("[rust ai] game " + i + " " + names.get(a) + " vs " + names.get(b) + ": " + result + ", "
                    + now + " Rust AI answers");
        }
        System.out.println("[rust ai] " + (java ? "Java" : "Rust") + " AI won " + wins + " of " + games + " in "
                + (System.nanoTime() - started) / 1_000_000 + " ms");
        if (java) {
            return;
        }
        Assert.assertTrue("the Rust AI answered nothing", answered > 0);
        Assert.assertTrue(problems.size() + " of " + games + " games left the Rust AI:\n" + String.join("\n", problems),
                problems.isEmpty());
    }

    private static String play(long seed, DeckCardLists deckA, DeckCardLists deckB, int turns, List<String> problems,
                               String name, boolean java) throws Exception {
        Game game = new TwoPlayerDuel(MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL,
                MulliganType.GAME_DEFAULT.getMulligan(0), 60, 20, 7);
        Match match = new TwoPlayerMatch(new MatchOptions("rust ai", "Two Player Duel", false));
        HumanPlayer human = new HumanPlayer("A", RangeOfInfluence.ALL, 1);
        UserData settings = UserData.getDefaultUserDataView();
        if (seed % 4 >= 2) {
            // a player's own settings, not the defaults: the Rust copy follows them through the passes they make
            settings.getUserSkipPrioritySteps().getYourTurn().setUpkeep(true);
            settings.getUserSkipPrioritySteps().getYourTurn().setMain2(false);
            settings.getUserSkipPrioritySteps().getOpponentTurn().setEndOfTurn(true);
            settings.setConfirmEmptyManaPool(false);
            settings.setPassPriorityCast(true);
        }
        human.setUserData(settings);
        ComputerPlayerControllableProxy ai = new ComputerPlayerControllableProxy("B", RangeOfInfluence.ALL, 4);
        GameTraceTest.seat(game, match, human, deckA);
        GameTraceTest.seat(game, match, ai, deckB);
        DecisionLog log = DecisionLog.recording();
        RustAiBridge bridge = new RustAiBridge(ai.getId(), log, "test " + seed);
        Assert.assertTrue(java || game.getPlayer(ai.getId()).attachRustAi(bridge));
        GameTrace trace = new GameTrace(System.getenv("PLAYMAT_AI_DEBUG") != null, entry -> bridge.onTrace(game, entry));
        trace.setOnEnd(bridge::end);
        GameOptions options = new GameOptions();
        options.testMode = false;
        options.rollbackTurnsAllowed = false;
        options.stopOnTurn = turns;
        options.stopAtStep = PhaseStep.UNTAP;
        options.seed = seed;
        if (!java) {
            options.trace = trace;
            options.decisions = log;
        }
        game.setGameOptions(options);
        ExecutorService answers = Executors.newSingleThreadExecutor();
        RandomDriver driver = new RandomDriver(game, answers, new Random(seed));
        game.addPlayerQueryEventListener(driver::ask);
        Throwable[] failure = {null};
        Thread thread = new Thread(() -> {
            try {
                RandomUtil.withRandom(new Random(seed), () -> {
                    game.start(seed % 2 == 0 ? human.getId() : ai.getId());
                    return null;
                });
            } catch (Throwable e) {
                failure[0] = e;
            }
        }, ThreadUtils.THREAD_PREFIX_GAME + " rust ai");
        thread.start();
        thread.join(5 * 60 * 1000L);
        answers.shutdownNow();
        answers.awaitTermination(5, TimeUnit.SECONDS);
        if (thread.isAlive()) {
            game.setGameStopped(true);
            thread.interrupt();
            problems.add("game " + seed + " " + name + ": timeout");
            return "timeout";
        }
        if (failure[0] != null) {
            problems.add("game " + seed + " " + name + ": " + failure[0]);
            return "error " + failure[0];
        }
        if (bridge.getFailure() != null) {
            problems.add("game " + seed + " " + name + " (turn " + game.getTurnNum() + "): " + bridge.getFailure());
        }
        return (human.hasWon() ? "human won" : game.getPlayer(ai.getId()).hasWon() ? "AI won" : "draw")
                + " on turn " + game.getTurnNum() + (bridge.getFailure() != null ? ", Rust AI stopped" : "");
    }
}
