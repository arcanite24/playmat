package mage.server.ladder;

import mage.server.career.CareerStore;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.io.StringReader;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.UUID;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

/** Ranked's Open queue (#431): picks, a table's life, what a result moves and pays. */
public class RankedServiceTest {

    private static final long NOW = 1_800_000_000_000L;

    private LadderStore store;
    private Path careerFile;
    private CareerStore career;
    private RankedService ranked;

    @BeforeEach
    void open() throws Exception {
        store = new LadderStore("jdbc:sqlite::memory:");
        careerFile = Files.createTempFile("ranked-test", ".db");
        career = new CareerStore("jdbc:sqlite:" + careerFile);
        CareerStore.use(career);
        ranked = RankedService.forTests(store, roster());
        RankedService.use(ranked);
    }

    @AfterEach
    void close() throws Exception {
        RankedService.use(null);
        CareerStore.use(null);
        career.close();
        store.close();
        Files.deleteIfExists(careerFile);
    }

    @Test
    void thePickHoldsUntilItIsPlayed() throws Exception {
        BotRoster.Bot first = ranked.pick("Neri", "standard", RankedService.OPEN, NOW);
        for (int i = 0; i < 10; i++) {
            assertEquals(first.id, ranked.pick("neri", "standard", RankedService.OPEN, NOW).id, "asking again can't skip it");
        }
        UUID table = UUID.randomUUID();
        ranked.register(table, "Neri", "standard", RankedService.OPEN, first, "Mono Red", NOW);
        assertTrue(RankedService.isRanked(table));
        assertTrue(ranked.playing("NERI"));
        // a table that never got going gives the pick back
        ranked.forget(table, NOW);
        assertFalse(RankedService.isRanked(table));
        assertEquals("void", store.game(table.toString()).status);
        assertEquals(first.id, ranked.pick("neri", "standard", RankedService.OPEN, NOW).id);
    }

    @Test
    void aPickAvoidsTheBotsJustMet() throws Exception {
        Set<String> met = new HashSet<>();
        for (int i = 0; i < 3; i++) {
            BotRoster.Bot bot = ranked.pick("neri", "standard", RankedService.OPEN, NOW);
            assertFalse(met.contains(bot.id), bot.id + " was met in the last games");
            met.add(bot.id);
            UUID table = UUID.randomUUID();
            ranked.register(table, "neri", "standard", RankedService.OPEN, bot, "deck", NOW);
            ranked.matchEnded(table, 0, NOW);
        }
    }

    @Test
    void aWinMovesTheRankAndPaysBronzeOnce() throws Exception {
        UUID table = play("standard", 1);
        RankedService.Result result = ranked.result(table);
        assertEquals("won", result.status);
        assertEquals(0, result.stepBefore);
        assertEquals(1, result.stepAfter);
        assertEquals(Arrays.asList("bronze"), result.reached, "the first game reaches Bronze");
        assertNotNull(result.paid);
        assertEquals(3, result.paid.cosmetics.size(), "Bronze's title, sleeve and frame");
        assertEquals(0, result.paid.tokens);
        assertTrue(result.ratingAfter > result.ratingBefore);
        LadderStore.Standing standing = store.standing("neri", "standard", RankedService.OPEN, ranked.season(NOW));
        assertEquals(1, standing.step);
        assertEquals(1, standing.won);
        assertEquals("won", store.game(table.toString()).status);
        assertEquals("java", store.game(table.toString()).engine);
        assertTrue(career.unlocksOf("neri").stream().anyMatch(u -> u[1].equals("ladder-bronze")));

        RankedService.Result second = ranked.result(play("standard", 1));
        assertTrue(second.reached.isEmpty(), "Bronze pays once");
        assertNull(second.paid);
        // another format is another ladder, but the cosmetics were already the account's
        RankedService.Result pioneer = ranked.result(play("pioneer", 0));
        assertEquals(Arrays.asList("bronze"), pioneer.reached);
        assertEquals(0, pioneer.paid.cosmetics.size());
        assertEquals("lost", pioneer.status);
        assertEquals(0, pioneer.stepAfter, "Bronze losses cost nothing");
    }

    @Test
    void climbingPastGoldPaysEveryRankOnTheWayAndAToken() throws Exception {
        LadderStore.Standing standing = new LadderStore.Standing();
        standing.user = "neri";
        standing.format = "standard";
        standing.queue = RankedService.OPEN;
        standing.season = ranked.season(NOW);
        standing.step = 2 * LadderRank.RANK_STEPS - 1;
        standing.peak = standing.step;
        store.putStanding(standing);
        RankedService.Result result = ranked.result(play("standard", 1));
        assertEquals(Arrays.asList("gold"), result.reached, "Silver's peak already paid Bronze and Silver");
        assertEquals(1, result.paid.tokens, "Gold pays a capsule token");
        assertEquals("Gold 4", LadderRank.label(result.stepAfter));
    }

    @Test
    void aMatchEndsOnceAndTheRustHostIsRecorded() throws Exception {
        BotRoster.Bot bot = ranked.pick("neri", "standard", RankedService.OPEN, NOW);
        UUID table = UUID.randomUUID();
        ranked.register(table, "neri", "standard", RankedService.OPEN, bot, "deck", NOW);
        RankedService.hostedOnRust(table);
        assertNotNull(ranked.matchEnded(table, 0.5, NOW));
        assertNull(ranked.matchEnded(table, 1, NOW), "a second end changes nothing");
        assertEquals("drew", store.game(table.toString()).status);
        assertEquals("rust", store.game(table.toString()).engine);
        assertEquals(1, store.standing("neri", "standard", RankedService.OPEN, ranked.season(NOW)).drew);
    }

    @Test
    void gamesLeftPlayingByAnEarlierRunAreVoid() throws Exception {
        LadderStore.GameRow row = new LadderStore.GameRow();
        row.id = UUID.randomUUID().toString();
        row.user = "neri";
        row.format = "standard";
        row.queue = RankedService.OPEN;
        row.bot = "mid";
        row.deck = "deck";
        row.started = NOW;
        store.startGame(row);
        RankedService fresh = RankedService.forTests(store, roster());
        fresh.voidInterrupted(NOW + 1);
        assertEquals("void", store.game(row.id).status);
        assertTrue(store.playing().isEmpty());
    }

    @Test
    void deletingTheAccountForgetsItsLadder() throws Exception {
        play("standard", 1);
        assertFalse(((List<?>) store.export("Neri").get("games")).isEmpty());
        store.deleteUser("Neri");
        assertNull(store.standing("neri", "standard", RankedService.OPEN, ranked.season(NOW)));
        assertTrue(store.games("neri", "standard", RankedService.OPEN, 10).isEmpty());
        assertNull(store.rating(LadderRatings.userSubject("neri"), "standard", RankedService.OPEN));
        assertTrue(((List<?>) store.export("neri").get("rewards")).isEmpty());
    }

    @Test
    void bo3IsItsOwnRankAndRatingButPaysRanksOnceWithOpen() throws Exception {
        assertTrue(RankedService.isQueue(RankedService.BO3));
        assertEquals(2, RankedService.winsNeeded(RankedService.BO3));
        assertEquals(1, RankedService.winsNeeded(RankedService.OPEN));
        // Bo3 meets the Open queue's bots, the League-only ones stay out
        BotRoster.Bot bot = ranked.pick("neri", "standard", RankedService.BO3, NOW);
        assertTrue(bot.plays(RankedService.OPEN));

        RankedService.Result open = ranked.result(play("standard", 1));
        assertEquals(Arrays.asList("bronze"), open.reached);
        assertNotNull(open.paid);
        RankedService.Result bo3 = ranked.result(play("standard", 1, RankedService.BO3));
        assertEquals(RankedService.BO3, bo3.queue);
        assertEquals(0, bo3.stepBefore, "Bo3 starts its own climb");
        assertEquals(Arrays.asList("bronze"), bo3.reached, "Bronze is reached in Bo3 too");
        assertNull(bo3.paid, "but its rewards were paid by the Open queue");
        int season = ranked.season(NOW);
        assertEquals(1, store.standing("neri", "standard", RankedService.BO3, season).step);
        assertEquals(1, store.standing("neri", "standard", RankedService.OPEN, season).step);
        assertNotNull(store.rating(LadderRatings.userSubject("neri"), "standard", RankedService.BO3), "a Bo3 rating of its own");
    }

    @Test
    void aLimitedRunMovesTheLimitedRankOnceByWinsLessLosses() throws Exception {
        assertTrue(RankedService.isFormat(RankedService.LIMITED));
        assertFalse(ranked.formats().contains(RankedService.LIMITED), "no bots, so no queue");
        RankedService.Result run = ranked.limitedRun("Neri", "run-1", "sealed", "classic", "Foundations sealed", 3, 1, NOW);
        assertEquals("won", run.status);
        assertEquals(0, run.stepBefore);
        assertEquals(2, run.stepAfter, "3-1 is two pips, and no streak bonus");
        assertEquals(Arrays.asList("bronze"), run.reached);
        assertEquals(3, run.paid.cosmetics.size(), "the first Limited rank pays Bronze like any ladder");
        assertNull(ranked.limitedRun("neri", "run-1", "sealed", "classic", "Foundations sealed", 3, 1, NOW), "a run counts once");
        int season = ranked.season(NOW);
        LadderStore.Standing standing = store.standing("neri", RankedService.LIMITED, RankedService.OPEN, season);
        assertEquals(2, standing.step);
        assertEquals(1, standing.won);
        LadderStore.GameRow row = store.game("limited-run-1");
        assertEquals("won", row.status);
        assertEquals("limited:sealed-classic", row.bot);
        assertEquals(Integer.valueOf(2), row.stepAfter);
        assertNull(row.ratingAfter, "Limited has no rating");
        // the Constructed ladders don't move
        assertNull(store.standing("neri", "standard", RankedService.OPEN, season));

        // a bad run costs one pip at most, and nothing in Bronze
        RankedService.Result bad = ranked.limitedRun("neri", "run-2", "draft", "arena", "Foundations draft", 0, 3, NOW);
        assertEquals("lost", bad.status);
        assertEquals(2, bad.stepAfter);
        standing.step = LadderRank.RANK_STEPS + 2;
        standing.peak = standing.step;
        store.putStanding(standing);
        RankedService.Result silver = ranked.limitedRun("neri", "run-3", "draft", "arena", "Foundations draft", 1, 3, NOW);
        assertEquals(-1, silver.delta);
        RankedService.Result even = ranked.limitedRun("neri", "run-4", "sealed", "classic", "Foundations sealed", 2, 2, NOW);
        assertEquals("drew", even.status);
        assertEquals(0, even.delta);
        RankedService.Result arena = ranked.limitedRun("neri", "run-5", "draft", "arena", "Foundations draft", 7, 0, NOW);
        assertEquals(7, arena.delta, "a 7-0 Arena run is seven pips");
    }

    @Test
    void aBotOnlyInTheLeagueNeverMeetsBo3() {
        BotRoster.Bot league = new BotRoster.Bot();
        league.queues = Arrays.asList(RankedService.LEAGUE);
        assertFalse(league.plays(RankedService.BO3));
        BotRoster.Bot open = new BotRoster.Bot();
        open.queues = Arrays.asList(RankedService.OPEN);
        assertTrue(open.plays(RankedService.BO3));
    }

    private UUID play(String format, double score) throws Exception {
        return play(format, score, RankedService.OPEN);
    }

    private UUID play(String format, double score, String queue) throws Exception {
        BotRoster.Bot bot = ranked.pick("neri", format, queue, NOW);
        UUID table = UUID.randomUUID();
        ranked.register(table, "neri", format, queue, bot, "deck", NOW);
        assertNotNull(ranked.matchEnded(table, score, NOW));
        return table;
    }

    /** a bot's generation (#435): its pin goes to the Rust host, and a retired bot is never met again */
    @Test
    void aBotPlaysItsPinAndARetiredOneIsNeverPicked() throws Exception {
        StringBuilder bots = new StringBuilder();
        for (String id : new String[]{"old", "new"}) {
            bots.append(bots.length() > 0 ? "," : "").append("{\"id\":\"").append(id)
                    .append("\",\"format\":\"standard\",\"deck\":\"standard/starter.dck\",\"skill\":5,\"prior\":1200,\"rating\":1200,")
                    .append("\"rd\":60,\"games\":40,\"ai\":\"pin-").append(id).append("\",\"persona\":{\"name\":\"").append(id).append("\"}")
                    .append("old".equals(id) ? ",\"retired\":true,\"generation\":1}" : ",\"generation\":2}");
        }
        BotRoster roster = new BotRoster(new StringReader("{\"anchors\":{\"standard\":\"old\"},\"bots\":[" + bots + "]}"));
        RankedService service = RankedService.forTests(store, roster);
        RankedService.use(service);
        for (int i = 0; i < 20; i++) {
            BotRoster.Bot pick = service.pick("neri" + i, "standard", RankedService.OPEN, NOW);
            assertEquals("new", pick.id, "the retired bot isn't met");
        }
        assertNotNull(roster.bot("old"), "a retired bot stays listed, for the games against it");
        UUID table = UUID.randomUUID();
        service.register(table, "neri", "standard", RankedService.OPEN, roster.bot("new"), "deck", NOW);
        assertEquals("pin-new", RankedService.pinOf(table));
        assertNull(RankedService.pinOf(UUID.randomUUID()), "an unranked table has no pin");
    }

    static BotRoster roster() {
        StringBuilder bots = new StringBuilder();
        String[][] all = {{"standard", "weak", "1000"}, {"standard", "low", "1150"}, {"standard", "mid", "1200"},
                {"standard", "mid-high", "1250"}, {"standard", "high", "1300"}, {"standard", "strong", "1500"},
                {"pioneer", "p-weak", "1000"}, {"pioneer", "p-mid", "1200"}, {"pioneer", "p-high", "1250"}};
        for (String[] b : all) {
            if (bots.length() > 0) {
                bots.append(',');
            }
            bots.append("{\"id\":\"").append(b[1]).append("\",\"format\":\"").append(b[0])
                    .append("\",\"deck\":\"standard/starter.dck\",\"skill\":5,\"prior\":1200,\"rating\":").append(b[2])
                    .append(",\"rd\":60,\"games\":40,\"persona\":{\"name\":\"").append(b[1]).append("\"}}");
        }
        return new BotRoster(new StringReader("{\"anchors\":{\"standard\":\"weak\",\"pioneer\":\"p-weak\"},\"bots\":[" + bots + "]}"));
    }
}
