package mage.server.ladder;

/**
 * The visible rank (#431): Bronze, Silver, Gold, Platinum and Diamond, each with tiers 4 to 1 of four pips, then
 * Mythic. A standing is one number, its step: 0 is Bronze 4 with no pips, each pip is a step, and
 * {@link #MYTHIC_STEP} is Mythic. Wins climb; losses cost a pip from Silver up, but Silver never loses a tier and from
 * Gold up a rank once reached is kept. The hidden rating ({@link LadderRatings}) picks opponents; this only counts.
 */
public final class LadderRank {

    public static final String[] RANKS = {"bronze", "silver", "gold", "platinum", "diamond", "mythic"};
    public static final int TIERS = 4;
    public static final int PIPS = 4;
    /** steps in one rank: four tiers of four pips */
    public static final int RANK_STEPS = TIERS * PIPS;
    /** the first step of Mythic */
    public static final int MYTHIC_STEP = 5 * RANK_STEPS;
    /** a run of this many wins earns a bonus pip, in the ranks that have one */
    public static final int STREAK = 3;
    /** the last rank with the streak bonus (Gold) */
    static final int LAST_STREAK_RANK = 2;

    private LadderRank() {
    }

    /** the rank's index in {@link #RANKS} */
    public static int rank(int step) {
        return Math.min(5, Math.max(0, step) / RANK_STEPS);
    }

    public static String rankName(int step) {
        return RANKS[rank(step)];
    }

    /** the tier, 4 (lowest) to 1; 0 in Mythic */
    public static int tier(int step) {
        if (step >= MYTHIC_STEP) {
            return 0;
        }
        return TIERS - (Math.max(0, step) % RANK_STEPS) / PIPS;
    }

    /** pips filled in the tier, 0 to 3; 0 in Mythic */
    public static int pips(int step) {
        return step >= MYTHIC_STEP ? 0 : Math.max(0, step) % PIPS;
    }

    /** "Gold 2", "Mythic" */
    public static String label(int step) {
        String name = rankName(step);
        String title = Character.toUpperCase(name.charAt(0)) + name.substring(1);
        return step >= MYTHIC_STEP ? title : title + " " + tier(step);
    }

    /** What one game does to a standing */
    public static final class Move {
        public final int step;
        public final int streak;
        /** pips gained (or lost, negative) */
        public final int delta;
        /** a streak bonus was paid */
        public final boolean bonus;

        Move(int step, int streak, int delta, boolean bonus) {
            this.step = step;
            this.streak = streak;
            this.delta = delta;
            this.bonus = bonus;
        }
    }

    /**
     * The standing after a game: `score` 1 a win, 0.5 a draw, 0 a loss. A win is a pip, and a third win in a row
     * through Gold a second. A loss costs nothing in Bronze; in Silver a pip, never below the tier; from Gold up a
     * pip, never below the rank. Mythic is kept. A draw changes nothing but ends the streak.
     */
    public static Move after(int step, int streak, double score) {
        if (score >= 1) {
            int nextStreak = streak + 1;
            boolean bonus = nextStreak >= STREAK && rank(step) <= LAST_STREAK_RANK;
            int gain = bonus ? 2 : 1;
            int next = Math.min(MYTHIC_STEP, step + gain);
            return new Move(next, nextStreak, next - step, bonus);
        }
        if (score <= 0) {
            int rank = rank(step);
            int next;
            if (rank == 0 || step >= MYTHIC_STEP) {
                next = step;
            } else if (rank == 1) {
                int tierFloor = step - pips(step);
                next = Math.max(tierFloor, step - 1);
            } else {
                int rankFloor = rank * RANK_STEPS;
                next = Math.max(rankFloor, step - 1);
            }
            return new Move(next, 0, next - step, false);
        }
        return new Move(step, 0, 0, false);
    }
}
