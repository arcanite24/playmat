package mage.server.solo;

import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.server.admin.RuntimeSettings;
import mage.server.career.CareerCampaigns;
import mage.server.career.CareerChallenges;
import mage.server.career.CareerContent;
import mage.server.career.CareerContentHooks;
import mage.server.career.CareerPuzzles;
import mage.server.career.CareerStore;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.stream.Stream;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Single-player foundations on the server: the solo database, leaderboards (and the weekly challenge moving onto
 * them), the AI-against-AI batch runner and content packs.
 */
public class SoloFoundationsTest {

    private Path dir;
    private SoloStore store;

    @BeforeEach
    public void setUp() throws Exception {
        dir = Files.createTempDirectory("solo-test");
        store = new SoloStore("jdbc:sqlite:" + dir.resolve("solo.db"));
        SoloStore.use(store);
        RuntimeSettings.use(RuntimeSettings.create(null));
        Leaderboards.get().forgetMigration();
    }

    @AfterEach
    public void tearDown() throws Exception {
        store.close();
        SoloStore.use(null);
        RuntimeSettings.use(null);
        CareerStore.use(null);
        ContentPacks.get().useRoot(java.nio.file.Paths.get("db", "content-packs"));
        CareerContentHooks.reloadAll();
        try (Stream<Path> paths = Files.walk(dir)) {
            paths.sorted(Comparator.reverseOrder()).forEach(path -> path.toFile().delete());
        }
    }

    // ---- saved games

    @Test
    public void savedGamesListKeepAndPrune() throws Exception {
        assertEquals(3, store.schemaVersion(), "saves, what-if lines, then mined puzzles, shared decks, the inbox and player state");
        store.saveGame("t1", "Ana", SoloGames.AUTO, false, 3, "Ana vs Bot", "Bot", "Two Player Duel", new byte[]{1}, new byte[]{2, 3});
        store.saveGame("t2", "Ana", SoloGames.AUTO, true, 5, "Career", "Wren", "Two Player Duel", new byte[]{1}, new byte[]{4});
        store.saveGame("t3", "Bea", SoloGames.AUTO, false, 1, "Bea vs Bot", "Bot", "Two Player Duel", new byte[]{1}, new byte[]{5});
        // a later turn of the same game replaces it
        store.saveGame("t1", "Ana", SoloGames.AUTO, false, 4, "Ana vs Bot", "Bot", "Two Player Duel", new byte[]{1}, new byte[]{6});
        List<SoloStore.SavedRow> ana = store.savedGames("Ana");
        assertEquals(2, ana.size());
        assertEquals(4, ana.stream().filter(row -> row.id.equals("t1")).findFirst().get().turn);
        assertArrayEquals(new byte[]{6}, store.loadGame("t1", "Ana")[1]);
        assertNull(store.loadGame("t1", "Bea"), "someone else's save");

        store.setKind("t1", SoloGames.KEPT);
        assertEquals(SoloGames.KEPT, store.savedGames("Ana").stream().filter(row -> row.id.equals("t1")).findFirst().get().kind);
        assertFalse(store.deleteGame("t1", "Bea"));
        assertTrue(store.deleteGame("t1", "Ana"));

        // one save per player kept: Ana's older one goes
        store.saveGame("t4", "Ana", SoloGames.AUTO, false, 2, "Again", "Bot", "Two Player Duel", new byte[]{1}, new byte[]{7});
        assertEquals(1, store.pruneGames(0, 1));
        assertEquals(Collections.singletonList("t4"), store.savedGames("Ana").stream().map(row -> row.id).collect(java.util.stream.Collectors.toList()));
        // and everything older than now
        assertEquals(2, store.pruneGames(System.currentTimeMillis() + 1000, 10));
    }

    @Test
    public void savedTableReadsBack() throws Exception {
        SoloGames.SavedTable table = new SoloGames.SavedTable();
        table.user = "Ana";
        table.title = "Ana vs Bot";
        table.options = new mage.game.match.MatchOptions("Ana vs Bot", "Two Player Duel", false);
        SoloGames.SavedSeat seat = new SoloGames.SavedSeat();
        seat.name = "Bot";
        seat.type = mage.players.PlayerType.COMPUTER_MAD;
        seat.skill = 4;
        seat.deck = deck("Forest", 20);
        seat.wins = 1;
        table.seats.add(seat);
        SoloGames.SavedTable read = (SoloGames.SavedTable) SoloGames.readObject(SoloGames.writeObject(table));
        assertEquals("Ana", read.user);
        assertEquals(1, read.seats.get(0).wins);
        assertEquals(20, read.seats.get(0).deck.getCards().get(0).getAmount());
    }

    // ---- leaderboards

    @Test
    public void leaderboardsKeepTheBestAndRankByTheModesRule() throws Exception {
        Leaderboards boards = Leaderboards.get();
        String week = "2026-W42";
        boards.attempt(Leaderboards.WEEKLY_CHALLENGE, week, "Ana");
        assertTrue(boards.submit(Leaderboards.WEEKLY_CHALLENGE, week, "Ana", 7, 5L, null));
        assertFalse(boards.submit(Leaderboards.WEEKLY_CHALLENGE, week, "Ana", 8, 20L, null), "more turns is worse");
        assertTrue(boards.submit(Leaderboards.WEEKLY_CHALLENGE, week, "ana", 7, 9L, null), "same turns, more life is better");
        boards.attempt(Leaderboards.WEEKLY_CHALLENGE, week, "Bea");
        boards.submit(Leaderboards.WEEKLY_CHALLENGE, week, "Bea", 6, 1L, null);
        boards.attempt(Leaderboards.WEEKLY_CHALLENGE, week, "Cal");

        Leaderboards.Leaderboard board = boards.board(Leaderboards.WEEKLY_CHALLENGE, week, "Ana", Leaderboards.SCOPE_SERVER, null, 50);
        assertEquals(3, board.total);
        assertEquals(Arrays.asList("bea", "ana", "cal"), names(board));
        assertEquals(2, board.yours.rank);
        assertEquals(Long.valueOf(9), board.yours.v2);
        assertNull(board.entries.get(2).v1, "tried, no score");

        // friends: only the names given, and you
        Leaderboards.Leaderboard friends = boards.board(Leaderboards.WEEKLY_CHALLENGE, week, "Ana", Leaderboards.SCOPE_FRIENDS,
                Collections.singletonList("Cal"), 50);
        assertEquals(Arrays.asList("ana", "cal"), names(friends));

        // top 1 still shows your line
        Leaderboards.Leaderboard top = boards.board(Leaderboards.WEEKLY_CHALLENGE, week, "Cal", Leaderboards.SCOPE_SERVER, null, 1);
        assertEquals(1, top.entries.size());
        assertEquals(3, top.yours.rank);

        // puzzles: more stars is better
        boards.submit(Leaderboards.PUZZLE, "p1", "Ana", 2, 3L, null);
        boards.submit(Leaderboards.PUZZLE, "p1", "Bea", 3, 1L, null);
        assertEquals(Arrays.asList("bea", "ana"), names(boards.board(Leaderboards.PUZZLE, "p1", null, null, null, 10)));
        assertThrows(IllegalArgumentException.class, () -> boards.attempt("nope", "x", "Ana"));
    }

    @Test
    public void weeklyChallengeResultsMoveOntoTheLeaderboards() throws Exception {
        CareerStore career = new CareerStore("jdbc:sqlite:" + dir.resolve("career.db"));
        CareerStore.use(career);
        try (java.sql.Connection connection = java.sql.DriverManager.getConnection("jdbc:sqlite:" + dir.resolve("career.db"));
             java.sql.Statement statement = connection.createStatement()) {
            statement.execute("INSERT INTO challenge_results (user, week, attempts, won, best_turns, best_life, at)"
                    + " VALUES ('ana', '2026-W40', 3, 1, 6, 12, 1), ('bea', '2026-W40', 2, 0, NULL, NULL, 2)");
        }
        Leaderboards.Leaderboard board = Leaderboards.get().board(Leaderboards.WEEKLY_CHALLENGE, "2026-W40", "bea",
                Leaderboards.SCOPE_SERVER, null, 50);
        assertEquals(Arrays.asList("ana", "bea"), names(board));
        assertEquals(Long.valueOf(6), board.entries.get(0).v1);
        assertEquals(3, board.entries.get(0).attempts);
        assertNull(board.yours.v1);
        career.close();
    }

    private static List<String> names(Leaderboards.Leaderboard board) {
        return board.entries.stream().map(entry -> entry.user).collect(java.util.stream.Collectors.toList());
    }

    // ---- what-if lines (S013, S014)

    @Test
    public void turnsAreKeptPerGameForTheirOwner() throws Exception {
        String game = java.util.UUID.randomUUID().toString();
        store.putLine(game, "Ana", new byte[]{1});
        store.putTurn(game, 1, "Ana", new byte[]{10});
        store.putTurn(game, 2, "Computer", new byte[]{20});
        store.putTurn(game, 2, "Computer", new byte[]{21}); // a turn written again replaces itself
        assertEquals("Ana", store.lineOwner(game));
        assertEquals(2, store.turns(game).size());
        assertEquals("Computer", store.turns(game).get(1).active);
        byte[][] second = store.loadTurn(game, 2, "Ana");
        assertNotNull(second);
        assertEquals(1, second[0][0], "the table");
        assertEquals(21, second[1][0], "the newest snapshot of the turn");
        assertNull(store.loadTurn(game, 2, "Bea"), "only the owner plays a game's turns again");
        assertNull(store.loadTurn(game, 3, "Ana"));

        // pruning: past the newest line per player, a line goes with its turns
        String newer = java.util.UUID.randomUUID().toString();
        Thread.sleep(5);
        store.putLine(newer, "Ana", new byte[]{2});
        assertEquals(1, store.pruneLines(0, 1));
        assertNull(store.lineOwner(game));
        assertTrue(store.turns(game).isEmpty());
        assertEquals("Ana", store.lineOwner(newer));
    }

    @Test
    public void aMatchupKeepsOnlyTheClosestGamesReplays() throws Exception {
        BatchRunner runner = BatchRunner.get();
        runner.use((spec, n, seed, owner) -> {
            BatchRunner.GameRecord record = new BatchRunner.GameRecord();
            record.n = n;
            record.seed = seed;
            record.turns = 8;
            record.winner = n % 2 == 0 ? "A" : "B";
            record.first = "A";
            record.lifeA = n % 2 == 0 ? 2 * n + 1 : 0;
            record.lifeB = n % 2 == 0 ? 0 : 2 * n + 1;
            record.replayId = java.util.UUID.randomUUID().toString();
            return record;
        });
        try {
            BatchRunner.JobSpec spec = spec(5);
            spec.keepReplays = 2;
            SoloStore.JobRow row = runner.submit("Ana", "matchup", spec);
            runner.drain(10_000);
            BatchRunner.JobResult result = BatchRunner.result(store.job(row.id));
            assertEquals(2, result.closest.size(), "the two closest games keep their replays");
            assertEquals(1, result.closest.get(0).margin());
            assertEquals(3, result.closest.get(1).margin());
            long kept = result.games.stream().filter(game -> game.replayId != null).count();
            assertEquals(2, kept, "the others' replays are gone");
        } finally {
            runner.use(BatchRunner::playGame);
        }
    }

    // ---- batches

    @Test
    public void batchesRunWithinTheQuotaAndKeepScore() throws Exception {
        BatchRunner runner = BatchRunner.get();
        AtomicInteger played = new AtomicInteger();
        runner.use((spec, n, seed, owner) -> {
            played.incrementAndGet();
            BatchRunner.GameRecord record = new BatchRunner.GameRecord();
            record.n = n;
            record.seed = seed;
            record.turns = 5 + n;
            record.winner = n % 3 == 0 ? "A" : n % 3 == 1 ? "B" : "draw";
            record.first = n % 2 == 0 ? "A" : "B";
            return record;
        });
        try {
            BatchRunner.JobSpec spec = spec(6);
            spec.seed = 100L;
            SoloStore.JobRow row = runner.submit("Ana", null, spec);
            assertEquals(BatchRunner.QUEUED, row.status);
            runner.drain(10_000);
            SoloStore.JobRow done = store.job(row.id);
            assertEquals(BatchRunner.DONE, done.status);
            BatchRunner.JobResult result = BatchRunner.result(done);
            assertEquals(6, result.games.size());
            assertEquals(2, result.winsA);
            assertEquals(2, result.winsB);
            assertEquals(2, result.draws);
            assertEquals(100L + 5, result.games.get(5).seed, "game n plays seed + n");
            assertEquals(6, played.get());

            // the day's quota
            assertThrows(IllegalArgumentException.class, () -> runner.submit("Ana", null, spec(RuntimeSettings.BATCH_GAMES_PER_DAY_DEFAULT)));
            // bad specs
            BatchRunner.JobSpec noDeck = spec(1);
            noDeck.deckB = null;
            assertThrows(IllegalArgumentException.class, () -> runner.submit("Bea", null, noDeck));
            assertThrows(IllegalArgumentException.class, () -> runner.submit("Bea", null, spec(0)));

            // cancelling a queued job: nothing plays
            SoloStore.JobRow queued = runner.submit("Bea", "versus", spec(3));
            assertTrue(runner.cancel(queued.id, "Bea"));
            assertFalse(runner.cancel(queued.id, "Bea"), "already cancelled");
            runner.drain(2_000);
            assertEquals(BatchRunner.CANCELLED, store.job(queued.id).status);
            assertEquals(6, played.get());
        } finally {
            runner.use(BatchRunner::playGame);
        }
    }

    @Test
    public void stoppedRunnerPlaysNothing() throws Exception {
        RuntimeSettings settings = settings();
        settings.set(RuntimeSettings.BATCH_CONCURRENCY, "0", "tests", 1L);
        RuntimeSettings.use(settings);
        assertEquals(0, settings.batchConcurrency());
        BatchRunner runner = BatchRunner.get();
        AtomicInteger played = new AtomicInteger();
        runner.use((spec, n, seed, owner) -> {
            played.incrementAndGet();
            return null;
        });
        try {
            SoloStore.JobRow row = runner.submit("Ana", null, spec(2));
            runner.tick();
            Thread.sleep(200);
            assertEquals(0, played.get());
            assertEquals(BatchRunner.QUEUED, store.job(row.id).status);
            runner.cancel(row.id, null);
        } finally {
            runner.use(BatchRunner::playGame);
        }
    }

    private RuntimeSettings settings() throws Exception {
        return RuntimeSettings.create(new mage.server.admin.AdminStore("jdbc:sqlite:" + dir.resolve("admin.db")));
    }

    private static BatchRunner.JobSpec spec(int games) {
        BatchRunner.JobSpec spec = new BatchRunner.JobSpec();
        spec.deckA = deck("Forest", 40);
        spec.deckB = deck("Mountain", 40);
        spec.games = games;
        return spec;
    }

    private static DeckCardLists deck(String card, int amount) {
        DeckCardLists deck = new DeckCardLists();
        deck.getCards().add(new DeckCardInfo(card, "1", "M21", amount));
        return deck;
    }

    @Test
    public void runtimeSettingsTakeBatchLimits() throws Exception {
        assertEquals(RuntimeSettings.BATCH_CONCURRENCY_DEFAULT, RuntimeSettings.get().batchConcurrency());
        assertEquals(RuntimeSettings.BATCH_GAMES_PER_DAY_DEFAULT, RuntimeSettings.get().batchGamesPerDay());
        RuntimeSettings settings = settings();
        assertThrows(IllegalArgumentException.class, () -> settings.set(RuntimeSettings.BATCH_CONCURRENCY, "17", "tests", 1L));
        assertEquals("4", settings.set(RuntimeSettings.BATCH_CONCURRENCY, "4", "tests", 1L));
        assertEquals(4, settings.batchConcurrency());
        assertEquals("0", settings.set(RuntimeSettings.BATCH_GAMES_PER_DAY, "0", "tests", 1L));
        assertEquals(0, settings.batchGamesPerDay());
    }

    // ---- content packs

    private static final String PACK = "{\"id\":\"test-pack\",\"name\":\"Test pack\",\"version\":\"1\",\"author\":\"tests\","
            + "\"puzzles\":[{\"id\":\"pack-puzzle-1\",\"name\":\"Pack puzzle\",\"text\":\"Win.\","
            + "\"you\":{\"life\":3,\"hand\":[\"Lightning Bolt\"]},\"opponent\":{\"life\":3}}],"
            + "\"opponents\":[{\"id\":\"pack-rival\",\"name\":\"Pack Rival\",\"tier\":1,\"colors\":\"G\",\"deck\":\"rival.dck\"}],"
            + "\"challenges\":[{\"id\":\"pack-week\",\"week\":\"2030-W01\",\"name\":\"Pack week\",\"text\":\"Win.\","
            + "\"playerDeck\":\"rival.dck\",\"opponent\":{\"name\":\"Pack Rival\",\"deck\":\"rival.dck\",\"skill\":3,"
            + "\"twists\":{\"timed\":[{\"when\":\"turn\",\"turn\":5,\"gainLife\":5,\"message\":\"The rival rallies.\"}]}}}],"
            + "\"campaigns\":[{\"id\":\"pack-campaign\",\"name\":\"Pack campaign\",\"chapters\":[{\"id\":\"c1\",\"name\":\"One\","
            + "\"deck\":\"rival.dck\",\"nodes\":[{\"id\":\"n1\",\"type\":\"duel\",\"name\":\"First\","
            + "\"opponent\":{\"name\":\"Pack Rival\",\"deck\":\"rival.dck\",\"skill\":2}}]}]}]}";

    @Test
    public void contentPacksLoadMergeAndExplainProblems() throws Exception {
        Path root = dir.resolve("packs");
        writePack(root.resolve("test-pack"), PACK, "rival.dck", "20 [M21:274] Forest\n");
        // a broken one: its deck is missing and it reuses a built-in id
        String builtInPuzzle = CareerPuzzles.get().puzzles().keySet().iterator().next();
        writePack(root.resolve("broken"), "{\"id\":\"broken\",\"name\":\"Broken\",\"puzzles\":[{\"id\":\"" + builtInPuzzle
                + "\",\"name\":\"x\",\"you\":{},\"opponent\":{}}],\"opponents\":[{\"id\":\"b\",\"name\":\"B\",\"tier\":1,\"deck\":\"gone.dck\"}]}",
                null, null);
        ContentPacks packs = ContentPacks.get();
        packs.useRoot(root);
        List<ContentPacks.ContentPackView> views = packs.reload();
        ContentPacks.ContentPackView good = views.stream().filter(view -> view.id.equals("test-pack")).findFirst().get();
        assertTrue(good.loaded, String.join("; ", good.problems));
        assertEquals(1, good.puzzles);
        assertEquals(1, good.decks);
        ContentPacks.ContentPackView broken = views.stream().filter(view -> view.id.equals("broken")).findFirst().get();
        assertFalse(broken.loaded);
        assertTrue(broken.problems.stream().anyMatch(problem -> problem.contains("already has a puzzle")), broken.problems.toString());
        assertTrue(broken.problems.stream().anyMatch(problem -> problem.contains("gone.dck")), broken.problems.toString());

        // merged into the Career content
        assertTrue(CareerPuzzles.get().puzzles().containsKey("pack-puzzle-1"));
        assertTrue(CareerCampaigns.get().campaigns().containsKey("pack-campaign"));
        assertNotNull(CareerContent.get().opponent("pack-rival"));
        assertTrue(CareerChallenges.get().scheduled().containsKey("2030-W01"));
        assertEquals("pack:test-pack/rival.dck", CareerCampaigns.get().campaigns().get("pack-campaign").chapters.get(0).deck);
        // twenty Forests, as one line or (read through the card database) one entry a copy
        assertEquals(20, CareerCampaigns.get().deck("pack:test-pack/rival.dck").getCards().stream()
                .filter(card -> "Forest".equals(card.getCardName())).mapToInt(mage.cards.decks.DeckCardInfo::getAmount).sum());

        // removing it takes it out again
        assertTrue(packs.remove("test-pack"));
        assertFalse(CareerPuzzles.get().puzzles().containsKey("pack-puzzle-1"));
        assertFalse(packs.remove("../etc"));
    }

    @Test
    public void contentPacksInstallFromAZipAndIgnoreStrayFiles() throws Exception {
        Path root = dir.resolve("packs");
        ContentPacks packs = ContentPacks.get();
        packs.useRoot(root);
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        try (ZipOutputStream zip = new ZipOutputStream(bytes)) {
            put(zip, "test-pack/pack.json", PACK);
            put(zip, "test-pack/decks/rival.dck", "20 [M21:274] Forest\n");
            put(zip, "test-pack/../../evil.txt", "nope");
            put(zip, "test-pack/decks/../../evil.dck", "nope");
            put(zip, "test-pack/run.sh", "nope");
        }
        ContentPacks.ContentPackView view = packs.install(bytes.toByteArray());
        assertTrue(view.loaded, String.join("; ", view.problems));
        try (Stream<Path> files = Files.walk(dir)) {
            assertTrue(files.noneMatch(path -> path.getFileName().toString().startsWith("evil") || path.getFileName().toString().equals("run.sh")));
        }
        assertThrows(IllegalArgumentException.class, () -> packs.install(new byte[]{1, 2, 3}));
    }

    private static void put(ZipOutputStream zip, String name, String text) throws Exception {
        zip.putNextEntry(new ZipEntry(name));
        zip.write(text.getBytes(StandardCharsets.UTF_8));
        zip.closeEntry();
    }

    private static void writePack(Path pack, String json, String deckName, String deck) throws Exception {
        Files.createDirectories(pack.resolve("decks"));
        Files.write(pack.resolve("pack.json"), json.getBytes(StandardCharsets.UTF_8));
        if (deckName != null) {
            Files.write(pack.resolve("decks").resolve(deckName), deck.getBytes(StandardCharsets.UTF_8));
        }
    }
}
