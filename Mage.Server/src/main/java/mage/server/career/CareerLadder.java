package mage.server.career;

import com.google.gson.Gson;

import java.sql.SQLException;
import java.util.List;
import java.util.Map;

/**
 * The roster ladder past its first three tiers (S037–S042, S048–S050): rank badges, deck rotation by seed, difficulty
 * pips from measured win rates, the nemesis (whoever beat you last comes back with an edge and a bounty), the rival's
 * four-chapter arc, opponent personalities and adaptive skill, all on top of the roster duel.
 */
public final class CareerLadder {

    /** the nemesis's extra life (S040) */
    public static final int NEMESIS_LIFE = 3;
    /** extra coins for beating the nemesis (S040) */
    public static final int NEMESIS_BOUNTY = 40;
    private static final String NEMESIS = "career.nemesis";

    private CareerLadder() {
    }

    /** A player's standing on the ladder: the highest tier they've opened past, and its badge (S037). */
    public static class CareerRank {
        public int tier;
        public String tierName;
        public String badge;
    }

    /** The nemesis: who beat the player last, until the player beats them back. */
    public static class Nemesis {
        public String opponentId;
        public long since;
    }

    /**
     * The rank: a tier's badge is earned by the wins that open the next tier; the last tier's by beating each of its
     * opponents once. Null before the first.
     */
    public static CareerRank rank(String user) throws SQLException {
        CareerContent content = CareerContent.get();
        Map<String, Integer> wins = CareerStore.get().winsByOpponent(user);
        CareerRank rank = null;
        List<CareerContent.Tier> tiers = content.tiers();
        for (int i = 0; i < tiers.size(); i++) {
            CareerContent.Tier tier = tiers.get(i);
            int tierWins = 0;
            boolean all = true;
            for (CareerContent.Opponent opponent : content.opponents()) {
                if (opponent.tier == tier.tier) {
                    int w = wins.getOrDefault(opponent.id, 0);
                    tierWins += w;
                    all &= w > 0;
                }
            }
            boolean last = i == tiers.size() - 1;
            boolean earned = last ? all : tierWins >= CareerRules.WINS_TO_UNLOCK_NEXT_TIER;
            if (!earned) {
                break;
            }
            rank = new CareerRank();
            rank.tier = tier.tier;
            rank.tierName = tier.name;
            rank.badge = tier.badge == null ? tier.name : tier.badge;
        }
        return rank;
    }

    /** difficulty pips, 1 (easiest) to 5, from the measured win rate a starter deck has against the opponent (S039) */
    public static int pips(CareerContent.Opponent opponent) {
        int measured = CareerContent.measuredWinRate(opponent);
        if (opponent.measured == null) {
            // unmeasured: by tier
            return Math.max(1, Math.min(5, opponent.tier));
        }
        return Math.max(1, Math.min(5, 1 + (90 - measured) / 18));
    }

    /** the seed that picks the opponent's list this game (S038): the player and how often they've met */
    public static long deckSeed(String user, String opponentId) throws SQLException {
        Integer met = CareerStore.get().stats(user).get("met_" + opponentId);
        return (long) user.toLowerCase(java.util.Locale.ENGLISH).hashCode() * 31 + (met == null ? 0 : met);
    }

    public static Nemesis nemesis(String user) throws SQLException {
        String value = mage.server.solo.SoloStore.get().state(user, NEMESIS);
        return value == null ? null : new Gson().fromJson(value, Nemesis.class);
    }

    /** whether a rival chapter is open: its tier must be, and the chapter before beaten (S041) */
    static boolean rivalOpen(CareerContent.Opponent opponent, Map<String, Integer> wins) {
        if (opponent.rival == null || opponent.rival.chapter <= 1) {
            return true;
        }
        for (CareerContent.Opponent other : CareerContent.get().opponents()) {
            if (other.rival != null && other.rival.chapter == opponent.rival.chapter - 1) {
                return wins.getOrDefault(other.id, 0) > 0;
            }
        }
        return true;
    }

    /**
     * How a roster duel starts: the tier's twists, the opponent's own (a boss's phases, a raid's life), the nemesis's
     * edge, and the opponent's personality.
     */
    public static CareerSetup setup(String user, CareerContent.Opponent opponent) throws SQLException {
        CareerContent.Tier tier = CareerContent.get().tier(opponent.tier);
        CareerSetup.CareerTwists twists = CareerSetup.combine(tier.twists, opponent.twists);
        Nemesis nemesis = nemesis(user);
        if (nemesis != null && opponent.id.equals(nemesis.opponentId)) {
            CareerSetup.CareerTwists edge = new CareerSetup.CareerTwists();
            edge.lifeBonus = NEMESIS_LIFE;
            twists = CareerSetup.combine(twists, edge);
        }
        return CareerSetup.withPersonality(CareerSetup.ofTwists(twists), opponent.personality);
    }

    /** a roster duel ended: the meeting is counted, the nemesis set or settled, the bounty paid, the rival's line said */
    static void finished(CareerStore store, String user, String opponentId, boolean won, CareerProgress.CareerGameResult result, long now) throws SQLException {
        CareerContent.Opponent opponent = CareerContent.get().opponent(opponentId);
        if (opponent == null) {
            return;
        }
        store.addStat(user, "met_" + opponentId, 1);
        result.mode = new CareerProgress.CareerModeResult("duel", opponent.name);
        result.mode.ref = opponentId;
        mage.server.solo.SoloStore solo = mage.server.solo.SoloStore.get();
        Nemesis nemesis = nemesis(user);
        boolean wasNemesis = nemesis != null && opponentId.equals(nemesis.opponentId);
        if (won) {
            if (wasNemesis) {
                solo.putState(user, NEMESIS, "null");
                store.addCoins(user, NEMESIS_BOUNTY);
                result.coins += NEMESIS_BOUNTY;
                result.mode.coins += NEMESIS_BOUNTY;
                result.mode.lines.add("Nemesis beaten: " + NEMESIS_BOUNTY + " coins of bounty.");
                store.addStat(user, "nemeses_beaten", 1);
            }
            if (opponent.rival != null) {
                result.mode.lines.add(opponent.name + ": “" + opponent.rival.after + "”");
                store.maxStat(user, "rival_chapter", opponent.rival.chapter);
            }
        } else if (!opponent.raid) {
            Nemesis next = new Nemesis();
            next.opponentId = opponentId;
            next.since = now;
            solo.putState(user, NEMESIS, new Gson().toJson(next));
            if (!wasNemesis) {
                result.mode.lines.add(opponent.name + " is your nemesis now: beat them for a bounty, but they'll start with "
                        + NEMESIS_LIFE + " extra life.");
            }
        }
        if (result.mode.lines.isEmpty()) {
            result.mode = null;
        }
    }
}
