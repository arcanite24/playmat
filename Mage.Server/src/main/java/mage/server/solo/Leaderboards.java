package mage.server.solo;

import mage.server.career.CareerStore;
import org.apache.log4j.Logger;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Comparator;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;

/**
 * Leaderboards for any mode with a scoring rule (S005). A mode has boards (a week, a puzzle, a seed...), each player
 * one row per board holding their best score and how often they tried. A board is read for the whole server or for
 * a list of names (friends).
 * <p>
 * A score is up to two numbers, compared first to second; each mode says which way is better. Players without a
 * score yet (attempts only) rank after everyone with one, fewest attempts first.
 */
public final class Leaderboards {

    private static final Logger logger = Logger.getLogger(Leaderboards.class);
    private static final Leaderboards INSTANCE = new Leaderboards();

    /** the weekly Career challenge: board = ISO week, v1 = turns (fewer is better), v2 = life left (more is better) */
    public static final String WEEKLY_CHALLENGE = "weekly-challenge";
    /** a weekly challenge scored by life left (S032): v1 = life (more is better), v2 = turns (fewer is better) */
    public static final String WEEKLY_CHALLENGE_LIFE = "weekly-challenge-life";
    /** a weekly challenge won without casting a creature (S032): v1 = turns (fewer is better), v2 = life left */
    public static final String WEEKLY_CHALLENGE_NO_CREATURES = "weekly-challenge-no-creatures";
    /** Career puzzles: board = puzzle id, v1 = stars (more is better), v2 = attempts to first solve (fewer is better) */
    public static final String PUZZLE = "puzzle";

    /** the gauntlet map: board = "a" + ascension level, v1 = wins (more is better), v2 = relics (more is better) */
    public static final String GAUNTLET = "gauntlet";
    /** the highest ascension cleared: board "all", v1 = level, v2 = wins */
    public static final String ASCENSION = "ascension";
    /** one seed's runs, to compare with friends who played it: board = the seed, v1 = wins */
    public static final String GAUNTLET_SEED = "gauntlet-seed";
    /** boss rush: board "all", v1 = bosses beaten */
    public static final String BOSS_RUSH = "boss-rush";
    /** endless: board "all", v1 = wins */
    public static final String ENDLESS = "endless";
    /** solo limited runs: board = the run's kind and shape (sealed, draft, sealed-arena, ...), v1 = wins, v2 = losses (fewer is better) */
    public static final String LIMITED = "limited";
    /** the daily puzzle: board = the day (2026-10-13), v1 = attempts to solve (fewer is better), v2 = seconds (fewer is better) */
    public static final String DAILY_PUZZLE = "daily-puzzle";
    /** a horde: board = the horde's id, v1 = turns survived (more is better), v2 = life left (more is better) */
    public static final String HORDE = "horde";

    public static final String SCOPE_SERVER = "server";
    public static final String SCOPE_FRIENDS = "friends";

    /** How a mode scores. */
    public static final class Spec {
        public final String mode;
        public final String label;
        public final boolean v1HigherIsBetter;
        public final boolean v2HigherIsBetter;
        public final String v1Label;
        public final String v2Label;

        public Spec(String mode, String label, boolean v1HigherIsBetter, String v1Label, boolean v2HigherIsBetter, String v2Label) {
            this.mode = mode;
            this.label = label;
            this.v1HigherIsBetter = v1HigherIsBetter;
            this.v2HigherIsBetter = v2HigherIsBetter;
            this.v1Label = v1Label;
            this.v2Label = v2Label;
        }

        /** negative when {@code a} is the better score */
        int compare(Long a1, Long a2, Long b1, Long b2) {
            if (a1 == null || b1 == null) {
                return a1 == null ? (b1 == null ? 0 : 1) : -1;
            }
            int first = v1HigherIsBetter ? Long.compare(b1, a1) : Long.compare(a1, b1);
            if (first != 0) {
                return first;
            }
            long x = a2 == null ? (v2HigherIsBetter ? Long.MIN_VALUE : Long.MAX_VALUE) : a2;
            long y = b2 == null ? (v2HigherIsBetter ? Long.MIN_VALUE : Long.MAX_VALUE) : b2;
            return v2HigherIsBetter ? Long.compare(y, x) : Long.compare(x, y);
        }
    }

    /** One line of a board. */
    public static class LeaderboardEntry {
        public int rank;
        public String user;
        /** null: tried, no score yet */
        public Long v1;
        public Long v2;
        public int attempts;
        public String detail;
        public long at;
        public boolean you;
    }

    /** A board as a player sees it. */
    public static class Leaderboard {
        public String mode;
        public String label;
        public String board;
        public String scope;
        public String v1Label;
        public String v2Label;
        public boolean v1HigherIsBetter;
        public boolean v2HigherIsBetter;
        /** players on the board */
        public int total;
        public List<LeaderboardEntry> entries = new ArrayList<>();
        /** the asking player's line, also when it's below the top */
        public LeaderboardEntry yours;
    }

    private final Map<String, Spec> specs = new LinkedHashMap<>();
    private volatile boolean migrated;

    private Leaderboards() {
        register(new Spec(WEEKLY_CHALLENGE, "Weekly challenge", false, "Turns", true, "Life left"));
        register(new Spec(WEEKLY_CHALLENGE_LIFE, "Weekly challenge (most life)", true, "Life left", false, "Turns"));
        register(new Spec(WEEKLY_CHALLENGE_NO_CREATURES, "Weekly challenge (no creatures)", false, "Turns", true, "Life left"));
        register(new Spec(PUZZLE, "Puzzle", true, "Stars", false, "Tries"));
        register(new Spec(GAUNTLET, "Gauntlet", true, "Wins", true, "Relics"));
        register(new Spec(ASCENSION, "Ascension", true, "Level", true, "Wins"));
        register(new Spec(GAUNTLET_SEED, "Seeded run", true, "Wins", true, ""));
        register(new Spec(BOSS_RUSH, "Boss rush", true, "Bosses", true, ""));
        register(new Spec(ENDLESS, "Endless", true, "Wins", true, ""));
        register(new Spec(LIMITED, "Limited", true, "Wins", false, "Losses"));
        register(new Spec(DAILY_PUZZLE, "Daily puzzle", false, "Tries", false, "Seconds"));
        register(new Spec(HORDE, "Horde", true, "Turns", true, "Life left"));
    }

    public static Leaderboards get() {
        return INSTANCE;
    }

    public synchronized void register(Spec spec) {
        specs.put(spec.mode, spec);
    }

    public Spec spec(String mode) {
        return specs.get(mode);
    }

    public Collection<Spec> specs() {
        return specs.values();
    }

    static String key(String user) {
        return user.toLowerCase(Locale.ENGLISH);
    }

    /** a player tried; their score, if any, stays */
    public synchronized void attempt(String mode, String board, String user) throws SQLException {
        require(mode);
        migrate();
        SoloStore store = SoloStore.get();
        SoloStore.ScoreRow row = store.score(mode, board, key(user));
        if (row == null) {
            row = new SoloStore.ScoreRow();
            row.mode = mode;
            row.board = board;
            row.user = key(user);
            row.attempts = 1;
        } else {
            row.attempts++;
        }
        row.at = System.currentTimeMillis();
        store.putScore(row);
    }

    /**
     * A score: kept when it beats the player's best on the board.
     *
     * @return whether it's their new best
     */
    public synchronized boolean submit(String mode, String board, String user, long v1, Long v2, String detail) throws SQLException {
        Spec spec = require(mode);
        migrate();
        SoloStore store = SoloStore.get();
        SoloStore.ScoreRow row = store.score(mode, board, key(user));
        if (row == null) {
            row = new SoloStore.ScoreRow();
            row.mode = mode;
            row.board = board;
            row.user = key(user);
            row.attempts = 1;
        }
        boolean best = row.v1 == null || spec.compare(v1, v2, row.v1, row.v2) < 0;
        if (best) {
            row.v1 = v1;
            row.v2 = v2;
            row.detail = detail;
        }
        row.at = System.currentTimeMillis();
        store.putScore(row);
        return best;
    }

    /** whether a player has a score on a board */
    public boolean hasScore(String mode, String board, String user) throws SQLException {
        SoloStore.ScoreRow row = SoloStore.get().score(mode, board, key(user));
        return row != null && row.v1 != null;
    }

    /**
     * A board, best first.
     *
     * @param names for the friends scope: whose lines to show (the asking player is always on it)
     * @param limit how many lines from the top; the asking player's line is in {@code yours} too
     */
    public Leaderboard board(String mode, String board, String user, String scope, Collection<String> names, int limit) throws SQLException {
        Spec spec = require(mode);
        migrate();
        List<SoloStore.ScoreRow> rows = SoloStore.get().scores(mode, board);
        Set<String> among = null;
        boolean friends = SCOPE_FRIENDS.equals(scope);
        if (friends) {
            among = new HashSet<>();
            if (names != null) {
                for (String name : names) {
                    among.add(key(name));
                }
            }
            if (user != null) {
                among.add(key(user));
            }
        }
        List<SoloStore.ScoreRow> shown = new ArrayList<>();
        for (SoloStore.ScoreRow row : rows) {
            if (among == null || among.contains(row.user)) {
                shown.add(row);
            }
        }
        shown.sort(Comparator.<SoloStore.ScoreRow>comparingInt(r -> 0)
                .thenComparing((a, b) -> spec.compare(a.v1, a.v2, b.v1, b.v2))
                .thenComparingInt(r -> r.attempts)
                .thenComparingLong(r -> r.at));
        Leaderboard view = new Leaderboard();
        view.mode = mode;
        view.label = spec.label;
        view.board = board;
        view.scope = friends ? SCOPE_FRIENDS : SCOPE_SERVER;
        view.v1Label = spec.v1Label;
        view.v2Label = spec.v2Label;
        view.v1HigherIsBetter = spec.v1HigherIsBetter;
        view.v2HigherIsBetter = spec.v2HigherIsBetter;
        view.total = shown.size();
        String you = user == null ? null : key(user);
        int rank = 0;
        for (SoloStore.ScoreRow row : shown) {
            rank++;
            boolean mine = row.user.equals(you);
            if (rank > limit && !mine) {
                continue;
            }
            LeaderboardEntry entry = new LeaderboardEntry();
            entry.rank = rank;
            entry.user = row.user;
            entry.v1 = row.v1;
            entry.v2 = row.v2;
            entry.attempts = row.attempts;
            entry.detail = row.detail;
            entry.at = row.at;
            entry.you = mine;
            if (rank <= limit) {
                view.entries.add(entry);
            }
            if (mine) {
                view.yours = entry;
            }
        }
        return view;
    }

    private Spec require(String mode) {
        Spec spec = specs.get(mode);
        if (spec == null) {
            throw new IllegalArgumentException("There's no leaderboard for " + mode);
        }
        return spec;
    }

    /** the weekly challenge's own table predates the leaderboards: copy it over once */
    public synchronized void migrate() {
        if (migrated) {
            return;
        }
        migrated = true;
        try {
            SoloStore store = SoloStore.get();
            if (store.countScores(WEEKLY_CHALLENGE) > 0) {
                return;
            }
            List<Object[]> rows = CareerStore.get().challengeRows();
            for (Object[] row : rows) {
                SoloStore.ScoreRow score = new SoloStore.ScoreRow();
                score.mode = WEEKLY_CHALLENGE;
                score.user = key((String) row[0]);
                score.board = (String) row[1];
                score.attempts = Math.max(1, (Integer) row[2]);
                if ((Boolean) row[3] && row[4] != null) {
                    score.v1 = ((Integer) row[4]).longValue();
                    score.v2 = row[5] == null ? null : ((Integer) row[5]).longValue();
                }
                score.at = (Long) row[6];
                store.putScore(score);
            }
            if (!rows.isEmpty()) {
                logger.info("Leaderboards: moved " + rows.size() + " weekly challenge results");
            }
        } catch (SQLException | RuntimeException e) {
            migrated = false;
            logger.warn("Leaderboards: the weekly challenge results couldn't be moved yet", e);
        }
    }

    /** for tests */
    public void forgetMigration() {
        migrated = false;
    }
}
