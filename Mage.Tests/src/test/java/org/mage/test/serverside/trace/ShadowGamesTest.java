package org.mage.test.serverside.trace;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import mage.cards.Card;
import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.cards.decks.importer.DeckImporter;
import mage.cards.repository.CardScanner;
import mage.constants.MultiplayerAttackOption;
import mage.constants.PhaseStep;
import mage.constants.RangeOfInfluence;
import mage.game.Game;
import mage.game.GameOptions;
import mage.game.TwoPlayerDuel;
import mage.game.TwoPlayerMatch;
import mage.game.match.Match;
import mage.game.match.MatchOptions;
import mage.game.mulligan.MulliganType;
import mage.game.trace.DecisionLog;
import mage.game.trace.GameTrace;
import mage.player.human.HumanPlayer;
import mage.players.Player;
import mage.players.net.UserData;
import mage.util.RandomUtil;
import mage.util.ThreadUtils;
import org.junit.Assume;
import org.junit.Test;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.UUID;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;

/**
 * Shadow games for the Rust engine (migration phase 4, docs/rust/ShadowGames.md): two human seats played by the
 * seeded {@link RandomDriver} with the Pioneer and Standard Career school decks. Each game is written as one JSON file:
 * the decks, the state when the first player gets priority (libraries, hands, the game's random state), every answer
 * from then on, and the trace. The Rust engine plays the same answers from the same start and must give the same
 * trace.
 * <p>
 * Skipped by default. Run with:
 * <pre>
 * mvn -pl Mage.Tests test -Dtest=ShadowGamesTest -Dxmage.shadowGames=$PWD/shadow \
 *     -Dxmage.dataCollectors.printGameLogs=false -Dlog4j.configuration=file:$PWD/.travis/log4j.properties
 * </pre>
 * Options: xmage.shadowGames.games (default 20), .first (first game, default 0), .turns (default 20),
 * .schools (default pioneer,standard: folders of Mage.Server/career-src), .decks (comma separated folders of deck
 * files to use instead, such as the engine's tiers in rust/crates/playmat-engine/shadow-decks), .views (record the
 * asked player's view at every n-th answer, default 0: none; docs/rust/Views.md).
 */
public class ShadowGamesTest {

    private static final String PREFIX = "xmage.shadowGames";
    private static final long GAME_TIMEOUT_MS = 5 * 60 * 1000L;

    @Test
    public void recordShadowGames() throws Exception {
        String dir = System.getProperty(PREFIX, "");
        Assume.assumeFalse("shadow games are written only with -D" + PREFIX + "=<dir>", dir.isEmpty());
        int games = Integer.getInteger(PREFIX + ".games", 20);
        int first = Integer.getInteger(PREFIX + ".first", 0);
        int turns = Integer.getInteger(PREFIX + ".turns", 20);
        String schools = System.getProperty(PREFIX + ".schools", "pioneer,standard");
        String deckDir = System.getProperty(PREFIX + ".decks", "");
        CardScanner.scan();

        List<String> names = new ArrayList<>();
        List<DeckCardLists> decks = new ArrayList<>();
        for (String school : (deckDir.isEmpty() ? schools : deckDir).split(",")) {
            File folder = deckDir.isEmpty() ? new File("../Mage.Server/career-src", school.trim()) : new File(school.trim());
            File[] files = folder.listFiles((d, name) -> name.endsWith(".txt"));
            if (files == null) {
                throw new IllegalStateException("no decks in " + folder.getAbsolutePath());
            }
            Arrays.sort(files);
            for (File file : files) {
                StringBuilder errors = new StringBuilder();
                decks.add(DeckImporter.importDeckFromFile(file.toString(), errors, false));
                names.add(folder.getName() + "/" + file.getName().replace(".txt", ""));
            }
        }
        File out = new File(dir);
        out.mkdirs();
        Gson gson = new GsonBuilder().disableHtmlEscaping().create();
        int n = decks.size();
        for (int i = first; i < first + games; i++) {
            int a = i % n;
            int b = (a + 1 + (i / n) % (n - 1)) % n;
            Map<String, Object> record = play(i, names.get(a), decks.get(a), names.get(b), decks.get(b), turns);
            Files.write(new File(out, String.format("game-%05d.json", i)).toPath(),
                    gson.toJson(record).getBytes(StandardCharsets.UTF_8));
            System.out.println("[shadow] game " + i + " " + names.get(a) + " vs " + names.get(b) + ": "
                    + record.get("result") + ", " + ((List<?>) record.get("decisions")).size() + " answers, "
                    + ((List<?>) record.get("trace")).size() + " trace entries");
        }
    }

    private static Map<String, Object> play(long seed, String nameA, DeckCardLists deckA, String nameB, DeckCardLists deckB,
                                            int turns) throws Exception {
        Map<String, Object> start = new LinkedHashMap<>();
        DecisionLog log = DecisionLog.recording();
        AtomicReference<Game> gameRef = new AtomicReference<>();
        List<Integer> decisionAt = java.util.Collections.synchronizedList(new ArrayList<>());
        GameTrace trace = new GameTrace(true, entry -> {
            decisionAt.add(log.getDecisions().size());
            if (entry.index == 0) {
                // the first priority: everything before it (shuffles, mulligans, opening hands) is the start
                start.putAll(mage.game.trace.ShadowStart.capture(gameRef.get(), log, entry.text));
            }
        });
        Game game = new TwoPlayerDuel(MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL,
                MulliganType.GAME_DEFAULT.getMulligan(0), 60, 20, 7);
        gameRef.set(game);
        Match match = new TwoPlayerMatch(new MatchOptions("shadow games", "Two Player Duel", false));
        HumanPlayer one = new HumanPlayer("A", RangeOfInfluence.ALL, 1);
        HumanPlayer two = new HumanPlayer("B", RangeOfInfluence.ALL, 1);
        one.setUserData(UserData.getDefaultUserDataView());
        two.setUserData(UserData.getDefaultUserDataView());
        GameTraceTest.seat(game, match, one, deckA);
        GameTraceTest.seat(game, match, two, deckB);
        GameOptions options = new GameOptions();
        options.testMode = false;
        options.rollbackTurnsAllowed = false;
        options.stopOnTurn = turns;
        options.stopAtStep = PhaseStep.UNTAP;
        options.seed = seed;
        options.trace = trace;
        options.decisions = log;
        game.setGameOptions(options);

        ExecutorService answers = Executors.newSingleThreadExecutor();
        RandomDriver driver = new RandomDriver(game, answers, new Random(seed));
        ShadowViews views = new ShadowViews(Integer.getInteger(PREFIX + ".views", 0));
        game.addPlayerQueryEventListener(event -> views.ask(game, event, log.getDecisions().size()));
        game.addPlayerQueryEventListener(driver::ask);
        AtomicReference<Throwable> failure = new AtomicReference<>();
        Thread thread = new Thread(() -> {
            try {
                RandomUtil.withRandom(new Random(seed), () -> {
                    game.start(seed % 2 == 0 ? one.getId() : two.getId());
                    return null;
                });
            } catch (Throwable e) {
                failure.set(e);
            }
        }, ThreadUtils.THREAD_PREFIX_GAME + " shadow games");
        thread.start();
        thread.join(GAME_TIMEOUT_MS);
        answers.shutdownNow();
        answers.awaitTermination(5, TimeUnit.SECONDS);
        String result;
        if (thread.isAlive()) {
            game.setGameStopped(true);
            thread.interrupt();
            result = "timeout";
        } else if (failure.get() != null) {
            failure.get().printStackTrace();
            result = "error: " + failure.get();
        } else {
            result = one.hasWon() ? "A won" : two.hasWon() ? "B won" : "draw";
        }

        Map<String, Object> record = new LinkedHashMap<>();
        record.put("version", 1);
        record.put("seed", seed);
        record.put("turns", turns);
        record.put("decks", Arrays.asList(deck(nameA, deckA), deck(nameB, deckB)));
        record.put("start", start);
        List<Map<String, Object>> decisions = new ArrayList<>();
        for (DecisionLog.Decision d : log.getDecisions()) {
            Map<String, Object> m = new LinkedHashMap<>();
            m.put("seat", d.seat);
            if (d.target != null) m.put("target", d.target);
            if (d.bool != null) m.put("bool", d.bool);
            if (d.integer != null) m.put("integer", d.integer);
            if (d.string != null) m.put("string", d.string);
            if (d.manaType != null) m.put("manaType", d.manaType);
            if (d.manaPlayer != null) m.put("manaPlayer", d.manaPlayer);
            decisions.add(m);
        }
        record.put("decisions", decisions);
        record.put("questions", driver.getQuestions());
        if (!views.getViews().isEmpty()) {
            record.put("views", views.getViews());
        }
        List<Map<String, Object>> entries = new ArrayList<>();
        for (GameTrace.Entry e : trace.getEntries()) {
            Map<String, Object> m = new LinkedHashMap<>();
            m.put("point", e.point);
            m.put("decision", e.index < decisionAt.size() ? decisionAt.get(e.index) : null); // answers given before it
            m.put("fp", e.hash);
            m.put("text", e.text);
            if (e.random >= 0) m.put("random", e.random); // where two engines' random numbers part
            entries.add(m);
        }
        record.put("trace", entries);
        record.put("result", result);
        return record;
    }

    private static Map<String, Object> deck(String name, DeckCardLists list) {
        Map<String, Object> deck = new LinkedHashMap<>();
        deck.put("name", name);
        List<String> cards = new ArrayList<>();
        for (DeckCardInfo info : list.getCards()) {
            cards.add(info.getAmount() + " " + info.getCardName());
        }
        List<String> sideboard = new ArrayList<>();
        for (DeckCardInfo info : list.getSideboard()) {
            sideboard.add(info.getAmount() + " " + info.getCardName());
        }
        deck.put("main", cards);
        deck.put("sideboard", sideboard);
        return deck;
    }
}
