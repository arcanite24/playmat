package mage.server.ladder;

import mage.server.career.CareerStore;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import java.time.ZoneOffset;
import java.util.UUID;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

/** Ladder seasons (#434): the calendar, the soft reset, and what a season's end pays. */
public class SeasonsTest {

    private static final Seasons CALENDAR = new Seasons(LocalDate.of(2026, 11, 1), 2);

    private LadderStore store;
    private Path careerFile;
    private CareerStore career;
    private RankedService ranked;

    @BeforeEach
    void open() throws Exception {
        Seasons.use(CALENDAR);
        store = new LadderStore("jdbc:sqlite::memory:");
        careerFile = Files.createTempFile("seasons-test", ".db");
        career = new CareerStore("jdbc:sqlite:" + careerFile);
        CareerStore.use(career);
        ranked = RankedService.forTests(store, RankedServiceTest.roster());
        RankedService.use(ranked);
    }

    @AfterEach
    void close() throws Exception {
        Seasons.use(null);
        RankedService.use(null);
        CareerStore.use(null);
        career.close();
        store.close();
        Files.deleteIfExists(careerFile);
    }

    private static long at(int year, int month, int day) {
        return LocalDate.of(year, month, day).atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli() + 3_600_000L;
    }

    @Test
    void theCalendarIsTwoMonthSeasonsFromTheStart() {
        assertEquals(0, CALENDAR.index(at(2026, 10, 20)), "before the start is the first season");
        assertEquals(0, CALENDAR.index(at(2026, 11, 1)));
        assertEquals(0, CALENDAR.index(at(2026, 12, 31)));
        assertEquals(1, CALENDAR.index(at(2027, 1, 1)));
        assertEquals(6, CALENDAR.index(at(2027, 11, 1)));
        assertEquals(at(2027, 1, 1) - 3_600_000L, CALENDAR.endsAt(0));
        assertEquals("Season 1", Seasons.name(0));
        Seasons fallback = Seasons.fromSettings("not a date", "2");
        assertEquals(Seasons.DEFAULT_START, fallback.start());
    }

    @Test
    void aResetPlacesLowerAndNeverPromotes() {
        int rank = LadderRank.RANK_STEPS;
        assertEquals("Platinum 4", LadderRank.label(LadderRank.placement(LadderRank.MYTHIC_STEP + 3)));
        assertEquals("Platinum 4", LadderRank.label(LadderRank.placement(4 * rank + 5)));
        assertEquals("Gold 4", LadderRank.label(LadderRank.placement(3 * rank + 15)));
        assertEquals("Silver 4", LadderRank.label(LadderRank.placement(2 * rank)));
        assertEquals("Bronze 1", LadderRank.label(LadderRank.placement(rank + 9)));
        assertEquals(3, LadderRank.placement(3), "Bronze stays where it was");
        assertEquals(0, LadderRank.placement(0));
    }

    private void stand(String queue, int season, int step, int peak) throws Exception {
        LadderStore.Standing s = new LadderStore.Standing();
        s.user = "neri";
        s.format = "standard";
        s.queue = queue;
        s.season = season;
        s.step = step;
        s.peak = peak;
        store.putStanding(s);
    }

    @Test
    void aNewSeasonPlacesThePlayerOnceAndPaysTheLeaguesEndOnce() throws Exception {
        int gold2 = 2 * LadderRank.RANK_STEPS + 9;
        stand(RankedService.LEAGUE, 0, gold2, gold2 + 2);
        long january = at(2027, 1, 10);
        assertEquals(1, ranked.season(january));
        for (int i = 0; i < 3; i++) {
            ranked.rollover("Neri", "standard", RankedService.LEAGUE, 1, january);
        }
        LadderStore.Standing placed = store.standing("neri", "standard", RankedService.LEAGUE, 1);
        assertEquals("Silver 4", LadderRank.label(placed.step));
        assertEquals(placed.step, placed.peak, "the placement is the new season's peak");
        assertEquals(3, career.leagueTokens("neri", "standard"), "Gold's season end pays 3 packs, once");
        // the season before is untouched
        assertEquals(gold2, store.standing("neri", "standard", RankedService.LEAGUE, 0).step);

        // a win from the placement pays nothing below it: no Bronze, no Silver, no tier
        BotRoster.Bot bot = ranked.pick("neri", "standard", RankedService.LEAGUE, january);
        UUID table = UUID.randomUUID();
        ranked.register(table, "neri", "standard", RankedService.LEAGUE, bot, "deck", january);
        RankedService.Result result = ranked.matchEnded(table, 1, january);
        assertTrue(result.reached.isEmpty(), "placement already reached Silver: " + result.reached);
        assertEquals(0, result.paid == null ? 0 : result.paid.packs);
        assertEquals(3, career.leagueTokens("neri", "standard"));
    }

    @Test
    void theOpenQueuesEndPaysNoPacksAndItsRanksPayOnceEver() throws Exception {
        stand(RankedService.OPEN, 0, 2 * LadderRank.RANK_STEPS - 1, 2 * LadderRank.RANK_STEPS - 1);
        long december = at(2026, 12, 10);
        RankedService.Result gold = play(RankedService.OPEN, december);
        assertEquals(1, gold.paid.tokens, "Gold pays a capsule token");
        long january = at(2027, 1, 10);
        ranked.rollover("neri", "standard", RankedService.OPEN, 1, january);
        assertEquals("Silver 4", LadderRank.label(store.standing("neri", "standard", RankedService.OPEN, 1).step));
        assertEquals(0, career.leagueTokens("neri", "standard"));
        // climb back to Gold in season 2: the rank is reached for the season, but the token was the account's
        LadderStore.Standing s = store.standing("neri", "standard", RankedService.OPEN, 1);
        s.step = 2 * LadderRank.RANK_STEPS - 1;
        store.putStanding(s);
        RankedService.Result again = play(RankedService.OPEN, january);
        assertTrue(again.reached.contains("gold"));
        assertNull(again.paid, "Open rank rewards pay once per account");
    }

    @Test
    void aGameCountsForTheSeasonItStartedIn() throws Exception {
        long lastNight = at(2026, 12, 31) + 20 * 3_600_000L;
        BotRoster.Bot bot = ranked.pick("neri", "standard", RankedService.OPEN, lastNight);
        UUID table = UUID.randomUUID();
        ranked.register(table, "neri", "standard", RankedService.OPEN, bot, "deck", lastNight);
        assertNotNull(ranked.matchEnded(table, 1, at(2027, 1, 1)));
        assertEquals(1, store.standing("neri", "standard", RankedService.OPEN, 0).step);
        assertNull(store.standing("neri", "standard", RankedService.OPEN, 1));
        assertEquals(0, store.game(table.toString()).season);
    }

    @Test
    void aChangedCalendarNeverGoesBackASeason() throws Exception {
        stand(RankedService.OPEN, 3, 5, 5);
        Seasons.use(new Seasons(LocalDate.of(2027, 1, 1), 6));
        assertEquals(3, ranked.season(at(2027, 2, 1)), "the calendar says 0, the ladder has played 3");
    }

    private RankedService.Result play(String queue, long now) throws Exception {
        BotRoster.Bot bot = ranked.pick("neri", "standard", queue, now);
        UUID table = UUID.randomUUID();
        ranked.register(table, "neri", "standard", queue, bot, "deck", now);
        return ranked.matchEnded(table, 1, now);
    }
}
