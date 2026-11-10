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
 * <p>
 * With -Dxmage.shadowGames.replay=&lt;dir&gt; it plays each game in that folder again from its decks, seed and answers
 * instead of the random driver, and writes it with Java's trace now: for a game whose answers were edited by hand after
 * Java stopped asking a question (docs/rust/HumanDialogs.md).
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
            Map<String, Object> record = play(i, names.get(a), decks.get(a), names.get(b), decks.get(b), turns, null);
            Files.write(new File(out, String.format("game-%05d.json", i)).toPath(),
                    gson.toJson(record).getBytes(StandardCharsets.UTF_8));
            System.out.println("[shadow] game " + i + " " + names.get(a) + " vs " + names.get(b) + ": "
                    + record.get("result") + ", " + ((List<?>) record.get("decisions")).size() + " answers, "
                    + ((List<?>) record.get("trace")).size() + " trace entries");
        }
    }

    @Test
    public void replayShadowGames() throws Exception {
        String dir = System.getProperty(PREFIX, "");
        String in = System.getProperty(PREFIX + ".replay", "");
        Assume.assumeFalse("shadow games are replayed only with -D" + PREFIX + ".replay=<dir> -D" + PREFIX + "=<dir>",
                dir.isEmpty() || in.isEmpty());
        CardScanner.scan();
        File out = new File(dir);
        out.mkdirs();
        Gson gson = new GsonBuilder().disableHtmlEscaping().create();
        File[] files = new File(in).listFiles((d, name) -> name.endsWith(".json"));
        Assume.assumeTrue("no games in " + in, files != null);
        Arrays.sort(files);
        for (File file : files) {
            Map<?, ?> game = gson.fromJson(new String(Files.readAllBytes(file.toPath()), StandardCharsets.UTF_8), Map.class);
            List<String> names = new ArrayList<>();
            List<DeckCardLists> decks = new ArrayList<>();
            for (Object o : (List<?>) game.get("decks")) {
                Map<?, ?> deck = (Map<?, ?>) o;
                StringBuilder text = new StringBuilder();
                for (Object line : (List<?>) deck.get("main")) {
                    text.append(line).append('\n');
                }
                for (Object line : (List<?>) deck.get("sideboard")) {
                    text.append("SB: ").append(line).append('\n');
                }
                File tmp = File.createTempFile("shadow-deck", ".txt");
                tmp.deleteOnExit();
                Files.write(tmp.toPath(), text.toString().getBytes(StandardCharsets.UTF_8));
                StringBuilder errors = new StringBuilder();
                decks.add(DeckImporter.importDeckFromFile(tmp.toString(), errors, false));
                names.add((String) deck.get("name"));
            }
            List<DecisionLog.Decision> decisions = new ArrayList<>();
            for (Object o : (List<?>) game.get("decisions")) {
                Map<?, ?> d = (Map<?, ?>) o;
                decisions.add(new DecisionLog.Decision(((Number) d.get("seat")).intValue(), (String) d.get("target"),
                        (Boolean) d.get("bool"), d.get("integer") == null ? null : ((Number) d.get("integer")).intValue(),
                        (String) d.get("string"), (String) d.get("manaType"), (String) d.get("manaPlayer")));
            }
            long seed = ((Number) game.get("seed")).longValue();
            int turns = ((Number) game.get("turns")).intValue();
            Map<String, Object> record = play(seed, names.get(0), decks.get(0), names.get(1), decks.get(1), turns, decisions);
            Files.write(new File(out, file.getName()).toPath(), gson.toJson(record).getBytes(StandardCharsets.UTF_8));
            System.out.println("[shadow] replayed " + file.getName() + ": " + record.get("result") + ", "
                    + decisions.size() + " answers, " + ((List<?>) record.get("trace")).size() + " trace entries");
        }
    }

    /** {@code replay}: the answers to give, or null for the random driver's */
    private static Map<String, Object> play(long seed, String nameA, DeckCardLists deckA, String nameB, DeckCardLists deckB,
                                            int turns, List<DecisionLog.Decision> replay) throws Exception {
        Map<String, Object> start = new LinkedHashMap<>();
        DecisionLog log = replay == null ? DecisionLog.recording() : DecisionLog.replaying(replay);
        java.util.function.IntSupplier answered = () -> replay == null ? log.getDecisions().size() : log.used();
        AtomicReference<Game> gameRef = new AtomicReference<>();
        List<Integer> decisionAt = java.util.Collections.synchronizedList(new ArrayList<>());
        GameTrace trace = new GameTrace(true, entry -> {
            decisionAt.add(answered.getAsInt());
            if (entry.index == 0) {
                // the first priority: everything before it (shuffles, mulligans, opening hands) is the start
                start.putAll(mage.game.trace.ShadowStart.capture(gameRef.get(), log, entry.text));
                start.put("decision", answered.getAsInt());
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
        RandomDriver driver = replay == null ? new RandomDriver(game, answers, new Random(seed)) : null;
        // a replay's questions: each human query's type and message, as the driver writes them
        List<String> asked = java.util.Collections.synchronizedList(new ArrayList<>());
        ShadowViews views = new ShadowViews(Integer.getInteger(PREFIX + ".views", 0));
        game.addPlayerQueryEventListener(event -> views.ask(game, event, answered.getAsInt()));
        game.addTableEventListener(event -> {
            if (event.getEventType() == mage.game.events.TableEvent.EventType.INFO) {
                views.log(game, event.getMessage(), answered.getAsInt());
            }
        });
        if (driver != null) {
            game.addPlayerQueryEventListener(driver::ask);
        } else {
            game.addPlayerQueryEventListener(event -> {
                if (game.getPlayer(event.getPlayerId()) instanceof HumanPlayer) {
                    asked.add(event.getQueryType() + " " + (event.getMessage() == null ? "" : event.getMessage()));
                }
            });
        }
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
        if (replay != null && log.getFailure() != null) {
            result = "error: replay " + log.getFailure();
        } else if (replay != null && log.used() != replay.size()) {
            result = "error: replay used " + log.used() + " of " + replay.size() + " answers";
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
        record.put("questions", driver != null ? driver.getQuestions() : asked);
        if (!views.getViews().isEmpty()) {
            record.put("views", views.getViews());
            record.put("prompts", views.getPrompts());
            record.put("log", views.getLog());
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
