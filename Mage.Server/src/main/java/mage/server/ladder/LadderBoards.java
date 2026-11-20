package mage.server.ladder;

import mage.server.solo.SoloStore;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.List;

/**
 * The season boards (#434): peak rank, Mythic rating and fastest climb per format, queue and season, read from the
 * ladder's own tables and served through {@link mage.server.solo.Leaderboards} like every other board. A board is
 * named "queue.format.season" ("league.standard.0"), or "queue.format" for the season now. Nothing is copied: past
 * seasons, deleted accounts and exports all follow the ladder tables.
 */
public final class LadderBoards {

    /** peak rank: v1 = the peak step (shown as its label in detail), v2 = games played (fewer first) */
    public static final String PEAK = "ladder-peak";
    /** Mythic rating: v1 = the rating after the season's last game, v2 = games played */
    public static final String MYTHIC = "ladder-mythic";
    /** fastest to Mythic: v1 = games it took, v2 = when (earlier first) */
    public static final String CLIMB = "ladder-climb";

    private LadderBoards() {
    }

    /** where a board name points */
    static final class Board {
        final String queue;
        final String format;
        final int season;

        Board(String queue, String format, int season) {
            this.queue = queue;
            this.format = format;
            this.season = season;
        }
    }

    static Board parse(String board, int current) {
        String[] parts = board == null ? new String[0] : board.split("\\.");
        if (parts.length < 2 || parts.length > 3 || !RankedService.isQueue(parts[0]) || !RankedService.isFormat(parts[1])) {
            throw new IllegalArgumentException("A ladder board is queue.format.season, like league.standard.0, not " + board);
        }
        int season = current;
        if (parts.length == 3) {
            try {
                season = Integer.parseInt(parts[2]);
            } catch (NumberFormatException e) {
                season = -1;
            }
            if (season < 0 || season > current) {
                throw new IllegalArgumentException("There's no season " + parts[2] + " yet");
            }
        }
        return new Board(parts[0], parts[1], season);
    }

    public static List<SoloStore.ScoreRow> rows(String mode, String board) throws SQLException {
        RankedService ranked = RankedService.get();
        Board where = parse(board, ranked.season(System.currentTimeMillis()));
        LadderStore store = ranked.store();
        List<LadderStore.BoardRow> found;
        switch (mode) {
            case PEAK:
                found = store.peakBoard(where.format, where.queue, where.season);
                break;
            case MYTHIC:
                found = store.mythicBoard(where.format, where.queue, where.season);
                break;
            case CLIMB:
                found = store.climbBoard(where.format, where.queue, where.season);
                break;
            default:
                throw new IllegalArgumentException("There's no ladder board " + mode);
        }
        List<SoloStore.ScoreRow> out = new ArrayList<>();
        for (LadderStore.BoardRow row : found) {
            SoloStore.ScoreRow score = new SoloStore.ScoreRow();
            score.mode = mode;
            score.board = board;
            score.user = row.user;
            score.v1 = row.value;
            score.v2 = CLIMB.equals(mode) ? row.at : (long) row.games;
            score.attempts = row.games;
            score.detail = PEAK.equals(mode) ? LadderRank.label((int) row.value) : null;
            score.at = row.at;
            out.add(score);
        }
        return out;
    }
}
