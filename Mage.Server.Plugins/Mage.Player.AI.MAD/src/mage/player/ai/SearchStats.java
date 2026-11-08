package mage.player.ai;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ConcurrentLinkedQueue;

/**
 * What each AI search cost, for the speed bench (AiSpeedBenchTest in Mage.Server's tests): off unless
 * {@code -Dxmage.ai.searchStats=true}, so a server pays nothing for it.
 */
public final class SearchStats {

    public static final boolean ENABLED = Boolean.getBoolean("xmage.ai.searchStats");

    private static final ConcurrentLinkedQueue<Search> SEARCHES = new ConcurrentLinkedQueue<>();

    /** One search: one priority decision of the AI. */
    public static final class Search {
        public final String game;
        public final String player;
        public final int turn;
        public final String step;
        /** the whole priority decision: copying the game, the search and picking the line */
        public final long decisionNanos;
        /** the search alone */
        public final long wallNanos;
        /** CPU time of the search thread */
        public final long cpuNanos;
        public final int nodes;
        /** actions the root looked at */
        public final int options;
        public final int permanents;
        public final int hand;
        /** stopped at its think time rather than finishing (or using its node budget) */
        public final boolean deadline;
        /** interrupted after its think time plus the grace seconds ("thinks too long") */
        public final boolean interrupted;

        Search(String game, String player, int turn, String step, long decisionNanos, long wallNanos, long cpuNanos, int nodes, int options,
               int permanents, int hand, boolean deadline, boolean interrupted) {
            this.game = game;
            this.player = player;
            this.turn = turn;
            this.step = step;
            this.decisionNanos = decisionNanos;
            this.wallNanos = wallNanos;
            this.cpuNanos = cpuNanos;
            this.nodes = nodes;
            this.options = options;
            this.permanents = permanents;
            this.hand = hand;
            this.deadline = deadline;
            this.interrupted = interrupted;
        }
    }

    private SearchStats() {
    }

    static void record(Search search) {
        SEARCHES.add(search);
    }

    /** the same search, timed as a whole decision */
    static Search decision(Search search, long decisionNanos) {
        return new Search(search.game, search.player, search.turn, search.step, decisionNanos, search.wallNanos,
                search.cpuNanos, search.nodes, search.options, search.permanents, search.hand, search.deadline,
                search.interrupted);
    }

    /** the searches recorded since the last call */
    public static List<Search> drain() {
        List<Search> searches = new ArrayList<>();
        for (Search search = SEARCHES.poll(); search != null; search = SEARCHES.poll()) {
            searches.add(search);
        }
        return searches;
    }
}
