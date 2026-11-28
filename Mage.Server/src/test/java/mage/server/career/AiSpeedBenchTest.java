package mage.server.career;

import mage.cards.decks.Deck;
import mage.cards.decks.DeckCardLists;
import mage.cards.decks.importer.DeckImporter;
import mage.cards.repository.CardScanner;
import mage.collectors.DataCollectorServices;
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
import mage.player.ai.ComputerPlayerControllableProxy;
import mage.player.ai.SearchStats;
import mage.players.Player;
import mage.util.ThreadUtils;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

import java.io.File;
import java.lang.management.ManagementFactory;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicReference;

/**
 * AI speed bench: seeded one-on-one AI games with the Pioneer and Standard school decks, measuring what a player
 * waits on. Prints CPU and wall time per game, the AI's decision times (median, 95th percentile, worst) and the
 * slowest decisions with their board. Game i plays deck pair i with seed i, so two runs on the same code play the
 * same games until the code changes a decision.
 * <p>
 * Skipped by default. Run with:
 * <pre>
 * mvn -o -q -pl Mage.Server.Plugins/Mage.Player.AI.MAD,Mage.Server test -Dtest=AiSpeedBenchTest \
 *     -Dsurefire.failIfNoSpecifiedTests=false -Dxmage.aiSpeedBench=true -Dxmage.ai.searchStats=true \
 *     -Dxmage.dataCollectors.printGameLogs=false -Dlog4j.configuration=file:$PWD/.travis/log4j.properties
 * </pre>
 * Options: xmage.aiSpeedBench.games (default 24), .skill (default 4), .maxTurns (default 30), .schools (default
 * pioneer,standard: folders of Mage.Server/career-src), .first (first game index, default 0), .only (comma separated
 * game indexes to play instead).
 * <p>
 * Games don't replay exactly (object ids are random, and hash order follows them), so compare runs by their totals:
 * CPU per search node is the steadiest number, and the decision percentiles over many games.
 * The report is printed with BENCH lines (the surefire XML's system-out when run with -q).
 */
public class AiSpeedBenchTest {

    private static final String PREFIX = "xmage.aiSpeedBench";
    private static final long GAME_TIMEOUT_MS = 15 * 60 * 1000L;
    private static final Map<String, mage.cards.repository.CardInfo> CARD_INFO_CACHE = new ConcurrentHashMap<>();

    @Test
    public void bench() throws Exception {
        Assumptions.assumeTrue(Boolean.getBoolean(PREFIX), "the AI speed bench runs only with -D" + PREFIX + "=true");
        Assumptions.assumeTrue(SearchStats.ENABLED, "the AI speed bench needs -Dxmage.ai.searchStats=true");
        int games = Integer.getInteger(PREFIX + ".games", 24);
        int first = Integer.getInteger(PREFIX + ".first", 0);
        int skill = Integer.getInteger(PREFIX + ".skill", 4);
        int maxTurns = Integer.getInteger(PREFIX + ".maxTurns", 30);
        String schools = System.getProperty(PREFIX + ".schools", "pioneer,standard");
        String only = System.getProperty(PREFIX + ".only", "");

        CardScanner.scan();
        DataCollectorServices.init(false, false);
        List<String> names = new ArrayList<>();
        List<DeckCardLists> decks = new ArrayList<>();
        for (String school : schools.split(",")) {
            File dir = new File("career-src", school.trim());
            File[] files = dir.listFiles((d, name) -> name.endsWith(".txt"));
            if (files == null) {
                throw new IllegalStateException("no decks in " + dir.getAbsolutePath());
            }
            Arrays.sort(files);
            for (File file : files) {
                StringBuilder errors = new StringBuilder();
                DeckCardLists deck = DeckImporter.importDeckFromFile(file.toString(), errors, false);
                Deck.load(deck, false, false, CARD_INFO_CACHE);
                names.add(school.trim() + "/" + file.getName().replace(".txt", ""));
                decks.add(deck);
            }
        }
        System.out.println(String.format("BENCH %d decks, skill %d, %s, draw at turn %d",
                decks.size(), skill, only.isEmpty() ? "games " + first + ".." + (first + games - 1) : "games " + only, maxTurns));

        List<SearchStats.Search> all = new ArrayList<>();
        double totalCpu = 0;
        double totalWall = 0;
        SearchStats.drain();
        List<Integer> indexes = new ArrayList<>();
        if (only.isEmpty()) {
            for (int i = first; i < first + games; i++) {
                indexes.add(i);
            }
        } else {
            for (String index : only.split(",")) {
                indexes.add(Integer.parseInt(index.trim()));
            }
            games = indexes.size();
        }
        for (int i : indexes) {
            // pairs walk the decks so every deck plays every few games: deck i against deck i + 1 + i / n
            int n = decks.size();
            int a = i % n;
            int b = (a + 1 + (i / n) % (n - 1)) % n;
            long cpuBefore = processCpu();
            long wallBefore = System.nanoTime();
            String outcome = playGame(decks.get(a), decks.get(b), skill, maxTurns, i);
            double wall = (System.nanoTime() - wallBefore) / 1e9;
            double cpu = (processCpu() - cpuBefore) / 1e9;
            List<SearchStats.Search> searches = SearchStats.drain();
            all.addAll(searches);
            totalCpu += cpu;
            totalWall += wall;
            long[] ms = searches.stream().mapToLong(s -> s.decisionNanos / 1_000_000).sorted().toArray();
            System.out.println(String.format("BENCH game %d %s vs %s: %s, %.1f s wall, %.1f CPU-s, %d decisions, p95 %d ms, max %d ms",
                    i, names.get(a), names.get(b), outcome, wall, cpu, ms.length, pct(ms, 0.95), ms.length == 0 ? 0 : ms[ms.length - 1]));
        }
        long[] ms = all.stream().mapToLong(s -> s.decisionNanos / 1_000_000).sorted().toArray();
        long deadlines = all.stream().filter(s -> s.deadline).count();
        long interrupts = all.stream().filter(s -> s.interrupted).count();
        double searchCpu = all.stream().mapToLong(s -> Math.max(0, s.cpuNanos)).sum() / 1e9;
        double searchWall = all.stream().mapToLong(s -> s.wallNanos).sum() / 1e9;
        double decisionWall = all.stream().mapToLong(s -> s.decisionNanos).sum() / 1e9;
        long nodes = all.stream().mapToLong(s -> s.nodes).sum();
        System.out.println(String.format("BENCH total: %d games, %.2f CPU-s per game, %.2f s wall per game; decisions %.1f s wall, of it searches %.1f s (%.1f CPU-s), %d nodes, %.0f us CPU per node",
                games, totalCpu / games, totalWall / games, decisionWall, searchWall, searchCpu, nodes, nodes == 0 ? 0 : searchCpu * 1e6 / nodes));
        System.out.println(String.format("BENCH decisions: %d, p50 %d ms, p90 %d ms, p95 %d ms, p99 %d ms, max %d ms, over 1 s %d, over 3 s %d, at deadline %d, interrupted %d",
                ms.length, pct(ms, 0.5), pct(ms, 0.9), pct(ms, 0.95), pct(ms, 0.99), ms.length == 0 ? 0 : ms[ms.length - 1],
                Arrays.stream(ms).filter(t -> t > 1000).count(), Arrays.stream(ms).filter(t -> t > 3000).count(), deadlines, interrupts));
        all.sort(Comparator.comparingLong((SearchStats.Search s) -> s.decisionNanos).reversed());
        for (SearchStats.Search s : all.subList(0, Math.min(20, all.size()))) {
            System.out.println(String.format("BENCH slow: game %s %s turn %d %s, %d ms decision, %d ms search, %d ms search CPU, %d nodes, %d options, %d permanents, %d in hand%s%s",
                    s.game, s.player, s.turn, s.step, s.decisionNanos / 1_000_000, s.wallNanos / 1_000_000, s.cpuNanos / 1_000_000, s.nodes, s.options,
                    s.permanents, s.hand, s.deadline ? ", deadline" : "", s.interrupted ? ", interrupted" : ""));
        }
    }

    private static long processCpu() {
        java.lang.management.OperatingSystemMXBean os = ManagementFactory.getOperatingSystemMXBean();
        return ((com.sun.management.OperatingSystemMXBean) os).getProcessCpuTime();
    }

    private static long pct(long[] sorted, double p) {
        if (sorted.length == 0) {
            return 0;
        }
        return sorted[Math.min(sorted.length - 1, (int) Math.ceil(p * sorted.length) - 1)];
    }

    private static String playGame(DeckCardLists deckA, DeckCardLists deckB, int skill, int maxTurns, long seed) throws Exception {
        Game game = new TwoPlayerDuel(MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL,
                MulliganType.GAME_DEFAULT.getMulligan(0), 60, 20, 7);
        Match match = new TwoPlayerMatch(new MatchOptions("bench", "Two Player Duel", false));
        Player a = new ComputerPlayerControllableProxy("A", RangeOfInfluence.ALL, skill);
        Player b = new ComputerPlayerControllableProxy("B", RangeOfInfluence.ALL, skill);
        seat(game, match, a, deckA);
        seat(game, match, b, deckB);
        GameOptions options = new GameOptions();
        options.testMode = false;
        options.stopOnTurn = maxTurns;
        options.stopAtStep = PhaseStep.UNTAP;
        options.seed = seed;
        game.setGameOptions(options);

        AtomicReference<Throwable> failure = new AtomicReference<>();
        Thread thread = new Thread(() -> {
            try {
                game.start(seed % 2 == 0 ? a.getId() : b.getId());
            } catch (Throwable e) {
                failure.set(e);
            }
        }, ThreadUtils.THREAD_PREFIX_GAME + " bench");
        thread.start();
        thread.join(GAME_TIMEOUT_MS);
        if (thread.isAlive()) {
            thread.interrupt();
            game.setGameStopped(true);
            thread.join(10_000);
            return "stuck at turn " + game.getTurnNum();
        }
        if (failure.get() != null) {
            failure.get().printStackTrace();
            return "error at turn " + game.getTurnNum();
        }
        String winner = game.getPlayer(a.getId()).hasWon() ? "A won" : game.getPlayer(b.getId()).hasWon() ? "B won" : "draw";
        return winner + " turn " + game.getTurnNum();
    }

    private static void seat(Game game, Match match, Player player, DeckCardLists list) throws Exception {
        Deck deck = Deck.load(list, false, false, CARD_INFO_CACHE);
        game.loadCards(deck.getCards(), player.getId());
        game.loadCards(deck.getSideboard(), player.getId());
        game.addPlayer(player, deck);
        match.addPlayer(player, deck);
    }
}
