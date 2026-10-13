package mage.server.career;

import com.google.gson.Gson;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Adaptive difficulty (S049): a hidden rating per player and format, in AI skill (1 to 10), that moves after each game
 * played with it so the player wins about 55% of the time: a win raises it by 0.45 steps, a loss lowers it by 0.55.
 * The player sees it as a sentence, never as a number to chase.
 */
public final class AdaptiveSkill {

    public static final double TARGET = 0.55;
    static final double STEP = 0.8;
    static final double START = 4.0;
    static final int RECENT = 20;
    private static final String KEY = "adaptive.";

    /** tables playing with adaptive skill: the player and the format to record the result under */
    private static final Map<UUID, String[]> TABLES = new ConcurrentHashMap<>();

    private AdaptiveSkill() {
    }

    static class Rating {
        double rating = START;
        int games;
        int wins;
        /** the latest results, oldest first: 1 a win, 0 a loss */
        List<Integer> recent = new ArrayList<>();
    }

    /** What the player sees. */
    public static class AdaptiveView {
        public String format;
        /** the skill the next game plays at */
        public int skill;
        public int games;
        /** wins of the latest games, and how many those are */
        public int recentWins;
        public int recentGames;
        public String sentence;
    }

    static Rating read(String user, String format) throws SQLException {
        String value = mage.server.solo.SoloStore.get().state(user, KEY + format);
        Rating rating = value == null ? null : new Gson().fromJson(value, Rating.class);
        return rating == null ? new Rating() : rating;
    }

    public static int skillFor(String user, String format) throws SQLException {
        return skill(read(user, format).rating);
    }

    static int skill(double rating) {
        return (int) Math.max(1, Math.min(10, Math.round(rating)));
    }

    /** the rating after a game: it settles where the player wins {@value #TARGET} of the time */
    static double next(double rating, boolean won) {
        double moved = rating + (won ? STEP * (1 - TARGET) : -STEP * TARGET);
        return Math.max(1, Math.min(10, moved));
    }

    public static AdaptiveView view(String user, String format) throws SQLException {
        Rating rating = read(user, format);
        AdaptiveView view = new AdaptiveView();
        view.format = format;
        view.skill = skill(rating.rating);
        view.games = rating.games;
        view.recentGames = rating.recent.size();
        view.recentWins = rating.recent.stream().mapToInt(Integer::intValue).sum();
        view.sentence = sentence(view);
        return view;
    }

    static String sentence(AdaptiveView view) {
        String level = view.skill <= 2 ? "gently" : view.skill <= 4 ? "at an easy pace" : view.skill <= 6 ? "at a fair pace"
                : view.skill <= 8 ? "hard" : "as hard as it can";
        if (view.recentGames < 3) {
            return "The computer will play you " + level + " and find your level over the next few games.";
        }
        return "The computer plays you " + level + ": you've won " + view.recentWins + " of your last " + view.recentGames
                + ", and it adjusts to keep games close.";
    }

    /** a table plays with adaptive skill; its result moves the rating when it ends */
    public static void mark(UUID tableId, String user, String format) {
        if (tableId != null) {
            TABLES.put(tableId, new String[]{user, format});
        }
    }

    public static boolean marked(UUID tableId) {
        return tableId != null && TABLES.containsKey(tableId);
    }

    /** a marked table's game ended */
    public static void ended(UUID tableId, String user, boolean won) {
        String[] marked = tableId == null ? null : TABLES.remove(tableId);
        if (marked == null || !marked[0].equalsIgnoreCase(user)) {
            return;
        }
        try {
            record(marked[0], marked[1], won);
        } catch (SQLException | RuntimeException e) {
            org.apache.log4j.Logger.getLogger(AdaptiveSkill.class).warn("Adaptive skill: a result wasn't recorded", e);
        }
    }

    static synchronized void record(String user, String format, boolean won) throws SQLException {
        Rating rating = read(user, format);
        rating.rating = next(rating.rating, won);
        rating.games++;
        rating.wins += won ? 1 : 0;
        rating.recent.add(won ? 1 : 0);
        while (rating.recent.size() > RECENT) {
            rating.recent.remove(0);
        }
        mage.server.solo.SoloStore.get().putState(user, KEY + format, new Gson().toJson(rating));
    }
}
