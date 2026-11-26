package mage.server.ladder;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * Anti-stuck, deck-diverse matchmaking for AI Ranked:
 * <ul>
 *     <li>Deduplicates candidates by deck (so two skill copies of the same deck are never treated as distinct opponents).</li>
 *     <li>Guarantees rotation after every game -- and strictly bans the just-lost deck and bot from repeating back-to-back.</li>
 *     <li>Applies a 4-game cooldown on lost decks, a 3-game cooldown on other recent decks, and soft archetype/color penalties.</li>
 *     <li>Applies gentle rating relief on 2+ consecutive losses without skewing Glicko-2 updates.</li>
 * </ul>
 */
public final class RankedMatchmaker {

    /** Minimum distinct decks in the primary rating window */
    static final int MIN_DISTINCT_DECKS = LadderRatings.MIN_DISTINCT_DECKS;
    /** Recent games inspected for diversity, loss streaks, and freshness */
    static final int HISTORY_LIMIT = 15;
    /** Hard cooldown on decks the player lost to */
    static final int LOSS_DECK_COOLDOWN = 4;
    /** Cooldown on decks recently faced (win or draw) */
    static final int RECENT_DECK_COOLDOWN = 3;
    /** Gaussian spread (Elo) for smooth proximity weighting */
    static final double RATING_SIGMA = 90.0;

    private final LadderRatings ratings;
    private final BotRoster roster;

    public RankedMatchmaker(LadderRatings ratings, BotRoster roster) {
        this.ratings = ratings;
        this.roster = roster;
    }

    /**
     * Picks the next opponent for `user` in `(format, queue)` given their current Glicko-2 rating and recent finished games
     * (newest first).
     */
    public BotRoster.Bot pick(String format, String queue, double playerRating, List<LadderStore.GameRow> recent) throws SQLException {
        int lossStreak = consecutiveLosses(recent);
        double targetShift = lossStreak >= 2 ? -25.0 * Math.min(lossStreak, 3) : 0.0;
        double targetRating = playerRating - 25.0 + targetShift;

        List<LadderRatings.Candidate> window = ratings.candidatesByDeck(format, queue, playerRating, targetShift, MIN_DISTINCT_DECKS);
        if (window.isEmpty()) {
            throw new IllegalStateException("There are no ladder bots for " + format);
        }

        // Identify recent decks, bots, archetypes, and colors from finished games
        String lastBotId = null;
        String lastDeck = null;
        String lastLostArchetype = null;
        String lastColors = null;
        Set<String> lostDecks = new HashSet<>();
        Set<String> recentDecks = new HashSet<>();
        Set<String> recentArchetypes = new HashSet<>();
        Set<String> seen15Decks = new HashSet<>();

        for (int i = 0; i < recent.size(); i++) {
            LadderStore.GameRow g = recent.get(i);
            BotRoster.Bot met = roster.bot(g.bot);
            String deck = g.botDeck != null && !g.botDeck.isEmpty()
                    ? g.botDeck
                    : (met != null ? LadderRatings.deckKey(met) : g.bot);
            String arch = g.botArchetype != null && !g.botArchetype.isEmpty()
                    ? g.botArchetype
                    : (met != null ? met.effectiveArchetype() : "");
            String colors = met != null && met.colors != null ? met.colors : "";

            if (i == 0) {
                lastBotId = g.bot;
                lastDeck = deck;
                lastColors = colors;
                if ("lost".equals(g.status)) {
                    lastLostArchetype = arch;
                }
            }
            if (i < LOSS_DECK_COOLDOWN && "lost".equals(g.status)) {
                lostDecks.add(deck);
            }
            if (i < RECENT_DECK_COOLDOWN) {
                recentDecks.add(deck);
            }
            if (i < 2 && !arch.isEmpty()) {
                recentArchetypes.add(arch);
            }
            seen15Decks.add(deck);
        }

        List<LadderRatings.Candidate> pool = selectPool(window, format, queue, targetRating, lastBotId, lastDeck, lostDecks, recentDecks);
        return weightedPick(pool, targetRating, lastLostArchetype, recentArchetypes, lastColors, seen15Decks, lossStreak);
    }

    private List<LadderRatings.Candidate> selectPool(
            List<LadderRatings.Candidate> window,
            String format,
            String queue,
            double targetRating,
            String lastBotId,
            String lastDeck,
            Set<String> lostDecks,
            Set<String> recentDecks
    ) throws SQLException {
        // Tier 1: window candidates excluding all recent decks and lost decks
        List<LadderRatings.Candidate> fresh = filter(window, lastBotId, lastDeck, lostDecks, recentDecks);
        if (fresh.size() >= 2) {
            return fresh;
        }
        // Tier 2: window candidates excluding last deck and lost decks (relax win/draw cooldown)
        List<LadderRatings.Candidate> noLoss = filter(window, lastBotId, lastDeck, lostDecks, null);
        if (!noLoss.isEmpty()) {
            return noLoss;
        }
        // Tier 3: widen to ALL distinct decks in the format, still respecting lost-deck cooldown
        List<LadderRatings.Candidate> all = ratings.allByDeck(format, queue, targetRating);
        List<LadderRatings.Candidate> allNoLoss = filter(all, lastBotId, lastDeck, lostDecks, null);
        if (!allNoLoss.isEmpty()) {
            return allNoLoss;
        }
        // Tier 4: relax older lost-deck cooldowns, but NEVER repeat the immediately previous deck/bot if another exists
        List<LadderRatings.Candidate> rotated = filter(all, lastBotId, lastDeck, null, null);
        if (!rotated.isEmpty()) {
            return rotated;
        }
        // Tier 5: if every bot in the format shares one single deck file, still avoid the exact previous bot id when possible
        List<LadderRatings.Candidate> differentBot = new ArrayList<>();
        for (BotRoster.Bot bot : roster.bots(format, queue)) {
            if (!bot.id.equals(lastBotId)) {
                differentBot.add(new LadderRatings.Candidate(bot, ratings.bot(bot, queue)));
            }
        }
        if (!differentBot.isEmpty()) {
            return differentBot;
        }
        return window;
    }

    private static List<LadderRatings.Candidate> filter(
            List<LadderRatings.Candidate> candidates,
            String lastBotId,
            String lastDeck,
            Set<String> lostDecks,
            Set<String> recentDecks
    ) {
        List<LadderRatings.Candidate> out = new ArrayList<>();
        for (LadderRatings.Candidate c : candidates) {
            String deck = LadderRatings.deckKey(c.bot);
            if (lastBotId != null && lastBotId.equals(c.bot.id)) {
                continue;
            }
            if (lastDeck != null && lastDeck.equals(deck)) {
                continue;
            }
            if (lostDecks != null && lostDecks.contains(deck)) {
                continue;
            }
            if (recentDecks != null && recentDecks.contains(deck)) {
                continue;
            }
            out.add(c);
        }
        return out;
    }

    private static BotRoster.Bot weightedPick(
            List<LadderRatings.Candidate> pool,
            double targetRating,
            String lastLostArchetype,
            Set<String> recentArchetypes,
            String lastColors,
            Set<String> seen15Decks,
            int lossStreak
    ) {
        if (pool.size() == 1) {
            return pool.get(0).bot;
        }
        double[] weights = new double[pool.size()];
        double total = 0.0;
        for (int i = 0; i < pool.size(); i++) {
            LadderRatings.Candidate c = pool.get(i);
            double diff = c.rating.rating - targetRating;
            double wRating = Math.max(0.05, Math.exp(-(diff * diff) / (2.0 * RATING_SIGMA * RATING_SIGMA)));
            String arch = c.bot.effectiveArchetype();
            double wArch = 1.0;
            if (lastLostArchetype != null && !lastLostArchetype.isEmpty() && lastLostArchetype.equals(arch)) {
                wArch = lossStreak >= 2 ? 0.15 : 0.25;
            } else if (recentArchetypes.contains(arch)) {
                wArch = 0.5;
            }
            String colors = c.bot.colors == null ? "" : c.bot.colors;
            double wColor = (!colors.isEmpty() && colors.equals(lastColors)) ? 0.6 : 1.0;
            double wFresh = seen15Decks.contains(LadderRatings.deckKey(c.bot)) ? 1.0 : 1.35;
            double w = wRating * wArch * wColor * wFresh;
            weights[i] = w;
            total += w;
        }
        if (total <= 0.0) {
            return pool.get(mage.util.RandomUtil.nextInt(pool.size())).bot;
        }
        double roll = mage.util.RandomUtil.nextDouble() * total;
        double acc = 0.0;
        for (int i = 0; i < pool.size(); i++) {
            acc += weights[i];
            if (roll <= acc) {
                return pool.get(i).bot;
            }
        }
        return pool.get(pool.size() - 1).bot;
    }

    static int consecutiveLosses(List<LadderStore.GameRow> recent) {
        int streak = 0;
        for (LadderStore.GameRow g : recent) {
            if ("lost".equals(g.status)) {
                streak++;
            } else {
                break;
            }
        }
        return streak;
    }
}
