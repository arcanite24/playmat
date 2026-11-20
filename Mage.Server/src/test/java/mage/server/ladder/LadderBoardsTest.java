package mage.server.ladder;

import mage.server.solo.Leaderboards;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.time.LocalDate;
import java.time.ZoneOffset;
import java.util.Arrays;
import java.util.List;
import java.util.UUID;
import java.util.stream.Collectors;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

/** The season boards (#434): peak rank, Mythic rating and fastest climb, read from the ladder's tables. */
public class LadderBoardsTest {

    private static final int MYTHIC = LadderRank.MYTHIC_STEP;

    private LadderStore store;
    private long clock = 1_000;

    @BeforeEach
    void open() throws Exception {
        // the calendar starts well before now, so seasons 0 and 1 have both begun
        Seasons.use(new Seasons(LocalDate.now(ZoneOffset.UTC).minusMonths(3).withDayOfMonth(1), 2));
        store = new LadderStore("jdbc:sqlite::memory:");
        RankedService.use(RankedService.forTests(store, RankedServiceTest.roster()));
    }

    @AfterEach
    void close() throws Exception {
        Seasons.use(null);
        RankedService.use(null);
        store.close();
    }

    private void stand(String user, int season, int peak, int games) throws Exception {
        LadderStore.Standing s = new LadderStore.Standing();
        s.user = user;
        s.format = "standard";
        s.queue = "league";
        s.season = season;
        s.step = peak;
        s.peak = peak;
        s.won = games;
        s.updated = clock++;
        store.putStanding(s);
    }

    private void game(String user, int season, String status, int stepAfter, Double ratingAfter) throws Exception {
        LadderStore.GameRow g = new LadderStore.GameRow();
        g.id = UUID.randomUUID().toString();
        g.user = user;
        g.format = "standard";
        g.queue = "league";
        g.season = season;
        g.bot = "bot";
        g.started = clock++;
        store.startGame(g);
        store.endGame(g.id, status, clock++, stepAfter - 1, stepAfter, ratingAfter, ratingAfter, null);
    }

    private static List<String> users(Leaderboards.Leaderboard board) {
        return board.entries.stream().map(entry -> entry.user).collect(Collectors.toList());
    }

    private static Leaderboards.Leaderboard board(String mode, String board, String user) throws Exception {
        return Leaderboards.get().board(mode, board, user, Leaderboards.SCOPE_SERVER, null, 50);
    }

    @Test
    void peakRanksBestFirstFewerGamesBreakTiesAndPlacedOnlyPlayersAreLeftOut() throws Exception {
        stand("ana", 1, MYTHIC, 40);
        stand("bo", 1, MYTHIC, 25);
        stand("cy", 1, 2 * LadderRank.RANK_STEPS + 5, 12);
        stand("dee", 1, 3 * LadderRank.RANK_STEPS, 0); // placed by the reset, no game this season
        stand("ana", 0, 5, 3);

        Leaderboards.Leaderboard view = board(LadderBoards.PEAK, "league.standard.1", "cy");
        assertEquals(Arrays.asList("bo", "ana", "cy"), users(view));
        assertEquals("Mythic", view.entries.get(0).detail);
        assertEquals("Gold 3", view.yours.detail);
        assertEquals(3, view.yours.rank);
        assertEquals(Arrays.asList("ana"), users(board(LadderBoards.PEAK, "league.standard.0", null)), "a past season keeps its own board");
        assertEquals(3, board(LadderBoards.PEAK, "league.standard", null).total, "no season is the season now");
    }

    @Test
    void mythicRatingIsTheLastRatingOfTheSeason() throws Exception {
        stand("ana", 1, MYTHIC, 3);
        game("ana", 1, "won", MYTHIC, 1900.4);
        game("ana", 1, "lost", MYTHIC, 1880.6);
        game("ana", 1, "void", MYTHIC, 2500.0);
        stand("bo", 1, MYTHIC, 1);
        game("bo", 1, "won", MYTHIC, 1950.0);
        stand("cy", 1, MYTHIC - 1, 1);
        game("cy", 1, "won", MYTHIC - 1, 2100.0);

        Leaderboards.Leaderboard view = board(LadderBoards.MYTHIC, "league.standard.1", null);
        assertEquals(Arrays.asList("bo", "ana"), users(view), "only Mythic players, a void game doesn't count");
        assertEquals(1881L, view.entries.get(1).v1);
    }

    @Test
    void fastestClimbCountsPlayedGamesUpToMythic() throws Exception {
        game("ana", 1, "won", 10, null);
        game("ana", 1, "void", 10, null);
        game("ana", 1, "lost", 10, null);
        game("ana", 1, "won", MYTHIC, null);
        game("ana", 1, "won", MYTHIC, null);
        game("bo", 1, "won", MYTHIC, null);
        game("cy", 1, "won", MYTHIC - 1, null);
        game("bo", 0, "won", MYTHIC, null);

        Leaderboards.Leaderboard view = board(LadderBoards.CLIMB, "league.standard.1", null);
        assertEquals(Arrays.asList("bo", "ana"), users(view));
        assertEquals(1L, view.entries.get(0).v1);
        assertEquals(3L, view.entries.get(1).v1, "three played games, the void one left out, the one after Mythic too");
    }

    @Test
    void aDeletedAccountLeavesTheBoards() throws Exception {
        stand("ana", 1, MYTHIC, 1);
        game("ana", 1, "won", MYTHIC, 1900.0);
        stand("bo", 1, 5, 1);
        store.deleteUser("ana");
        assertEquals(Arrays.asList("bo"), users(board(LadderBoards.PEAK, "league.standard.1", null)));
        assertEquals(0, board(LadderBoards.CLIMB, "league.standard.1", null).total);
        assertEquals(0, board(LadderBoards.MYTHIC, "league.standard.1", null).total);
    }

    @Test
    void badBoardsAndWritesAreRefused() throws Exception {
        assertThrows(IllegalArgumentException.class, () -> board(LadderBoards.PEAK, "league.standard.9", null), "a season not started yet");
        assertThrows(IllegalArgumentException.class, () -> board(LadderBoards.PEAK, "league.legacy.0", null));
        assertThrows(IllegalArgumentException.class, () -> board(LadderBoards.PEAK, "ranked.standard.0", null));
        assertThrows(IllegalArgumentException.class, () -> board(LadderBoards.PEAK, "league.standard.x", null));
        assertThrows(IllegalArgumentException.class, () -> Leaderboards.get().submit(LadderBoards.PEAK, "league.standard.0", "ana", 80, null, null));
        assertThrows(IllegalArgumentException.class, () -> Leaderboards.get().attempt(LadderBoards.CLIMB, "league.standard.0", "ana"));
        assertNotNull(board(LadderBoards.PEAK, "open.pioneer.0", null));
    }
}
