package mage.server.ladder;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Locale;

/**
 * The ladder's hidden ratings (#430): Glicko-2 for players and bots, per format and queue. A bot starts at the rating
 * its calibration measured, a player at {@link #START}; every rated game moves both. The rating picks who a player
 * meets; the visible rank (pips and tiers, #431) is kept apart.
 */
public final class LadderRatings {

    /** a new player's rating: among the bots that play starters with a light search */
    public static final double START = 1200;
    /** the League starts lower: its first opponents are the starter decks it's played with */
    public static final double LEAGUE_START = 1000;

    /** where a player's rating starts in a queue */
    public static double start(String queue) {
        return RankedService.LEAGUE.equals(queue) ? LEAGUE_START : START;
    }
    /** players never get surer than this, so a rating keeps moving with form */
    public static final double PLAYER_MIN_RD = 50;
    /** bots never get surer than this, so a bot players beat more than expected still sinks */
    public static final double BOT_MIN_RD = 30;
    /** a day without rated games widens a player's deviation by one empty rating period */
    static final long PERIOD_MS = 24L * 60 * 60 * 1000;
    /** at most this many idle periods are applied */
    static final int MAX_IDLE_PERIODS = 365;
    /**
     * the first window of bot ratings matchmaking draws from, around the player's: centred a little below, so a player
     * at their level expects to win about 54% and keeps climbing the visible rank
     */
    public static final double BELOW = 100;
    public static final double ABOVE = 50;
    /** how far the window widens when it holds too few bots */
    static final double WIDEN = 50;
    /** the fewest bots matchmaking wants to choose from */
    static final int MIN_CANDIDATES = 3;
    /** the fewest distinct decks matchmaking wants in its candidate window */
    public static final int MIN_DISTINCT_DECKS = 8;

    /** rated games are recorded one at a time: bots are shared, and a rating is read, updated and written back */
    private static final Object RECORDING = new Object();

    private final LadderStore store;
    private final BotRoster roster;

    public LadderRatings(LadderStore store, BotRoster roster) {
        this.store = store;
        this.roster = roster;
    }

    public static LadderRatings get() {
        return new LadderRatings(LadderStore.get(), BotRoster.get());
    }

    static String userSubject(String user) {
        return "user:" + user.toLowerCase(Locale.ENGLISH);
    }

    static String botSubject(String botId) {
        return "bot:" + botId;
    }

    /** the player's rating now: stored and widened for the days since their last rated game, or a new player's */
    public Glicko2.Rating player(String user, String format, String queue, long now) throws SQLException {
        LadderStore.Row row = store.rating(userSubject(user), format, queue);
        if (row == null) {
            return new Glicko2.Rating(start(queue), Glicko2.MAX_RD, Glicko2.START_VOLATILITY);
        }
        long periods = Math.min(MAX_IDLE_PERIODS, Math.max(0, (now - row.updated) / PERIOD_MS));
        Glicko2.Rating rating = row.rating;
        for (int i = 0; i < periods && rating.rd < Glicko2.MAX_RD; i++) {
            rating = Glicko2.update(rating, Collections.<Glicko2.Result>emptyList(), PLAYER_MIN_RD);
        }
        return rating;
    }

    /** the bot's rating now: stored after live games, or what its calibration measured */
    public Glicko2.Rating bot(BotRoster.Bot bot, String queue) throws SQLException {
        LadderStore.Row row = store.rating(botSubject(bot.id), bot.format, queue);
        return row == null ? bot.startRating() : row.rating;
    }

    /** What a rated game did to both ratings */
    public static final class Outcome {
        public final Glicko2.Rating playerBefore;
        public final Glicko2.Rating playerAfter;
        public final Glicko2.Rating botBefore;
        public final Glicko2.Rating botAfter;

        Outcome(Glicko2.Rating playerBefore, Glicko2.Rating playerAfter, Glicko2.Rating botBefore, Glicko2.Rating botAfter) {
            this.playerBefore = playerBefore;
            this.playerAfter = playerAfter;
            this.botBefore = botBefore;
            this.botAfter = botAfter;
        }
    }

    /**
     * Records a rated game between a player and a bot: `score` is the player's (1 won, 0.5 drawn, 0 lost). Each side is
     * updated against the other's rating from before the game, and both are stored together.
     */
    public Outcome record(String user, String botId, String queue, double score, long now) throws SQLException {
        BotRoster.Bot bot = roster.bot(botId);
        if (bot == null) {
            throw new IllegalArgumentException("There is no ladder bot " + botId);
        }
        if (score != 0 && score != 0.5 && score != 1) {
            throw new IllegalArgumentException("A score is 0, 0.5 or 1");
        }
        synchronized (RECORDING) {
            return recordLocked(user, bot, queue, score, now);
        }
    }

    private Outcome recordLocked(String user, BotRoster.Bot bot, String queue, double score, long now) throws SQLException {
        Glicko2.Rating player = player(user, bot.format, queue, now);
        Glicko2.Rating botRating = bot(bot, queue);
        Glicko2.Rating playerAfter = Glicko2.update(player,
                Collections.singletonList(new Glicko2.Result(botRating, score)), PLAYER_MIN_RD);
        Glicko2.Rating botAfter = Glicko2.update(botRating,
                Collections.singletonList(new Glicko2.Result(player, 1 - score)), BOT_MIN_RD);
        LadderStore.Row playerRow = store.rating(userSubject(user), bot.format, queue);
        LadderStore.Row botRow = store.rating(botSubject(bot.id), bot.format, queue);
        int botGames = botRow != null ? botRow.games + 1 : (bot.games == null ? 0 : bot.games) + 1;
        store.putBoth(bot.format, queue,
                userSubject(user), playerAfter, playerRow == null ? 1 : playerRow.games + 1,
                botSubject(bot.id), botAfter, botGames, now);
        return new Outcome(player, playerAfter, botRating, botAfter);
    }

    /** A bot matchmaking may pick, with its rating now */
    public static final class Candidate {
        public final BotRoster.Bot bot;
        public final Glicko2.Rating rating;

        Candidate(BotRoster.Bot bot, Glicko2.Rating rating) {
            this.bot = bot;
            this.rating = rating;
        }
    }

    /**
     * The bots a player at `rating` may meet in `format`: those rated from {@link #BELOW} under to {@link #ABOVE} over
     * the player, the window widening by {@link #WIDEN} each way until it holds {@link #MIN_CANDIDATES} (or every bot).
     * Weakest first.
     */
    public List<Candidate> candidates(String format, String queue, double rating) throws SQLException {
        List<Candidate> all = new ArrayList<>();
        for (BotRoster.Bot bot : roster.bots(format, queue)) {
            all.add(new Candidate(bot, bot(bot, queue)));
        }
        all.sort((a, b) -> Double.compare(a.rating.rating, b.rating.rating));
        double below = BELOW;
        double above = ABOVE;
        while (true) {
            List<Candidate> out = new ArrayList<>();
            for (Candidate c : all) {
                if (c.rating.rating >= rating - below && c.rating.rating <= rating + above) {
                    out.add(c);
                }
            }
            if (out.size() >= Math.min(MIN_CANDIDATES, all.size())) {
                return out;
            }
            below += WIDEN;
            above += WIDEN;
        }
    }

    /**
     * All eligible bots in `format` and `queue`, deduplicated by deck (picking the skill variant closest to `target`).
     * Weakest first.
     */
    public List<Candidate> allByDeck(String format, String queue, double target) throws SQLException {
        java.util.Map<String, Candidate> best = new java.util.LinkedHashMap<>();
        for (BotRoster.Bot bot : roster.bots(format, queue)) {
            Candidate c = new Candidate(bot, bot(bot, queue));
            String key = deckKey(bot);
            Candidate prev = best.get(key);
            if (prev == null || isCloser(c, prev, target)) {
                best.put(key, c);
            }
        }
        List<Candidate> out = new ArrayList<>(best.values());
        out.sort((a, b) -> Double.compare(a.rating.rating, b.rating.rating));
        return out;
    }

    /**
     * Distinct-deck candidates near `rating + targetShift`: widens the `[BELOW, ABOVE]` window by {@link #WIDEN}
     * until it holds at least `minDistinctDecks` unique decks (or every unique deck in the format). When multiple
     * skill copies of one deck fall in the window, keeps only the copy closest to the target rating.
     */
    public List<Candidate> candidatesByDeck(String format, String queue, double rating, double targetShift, int minDistinctDecks) throws SQLException {
        List<Candidate> all = new ArrayList<>();
        java.util.Set<String> totalDecks = new java.util.HashSet<>();
        for (BotRoster.Bot bot : roster.bots(format, queue)) {
            all.add(new Candidate(bot, bot(bot, queue)));
            totalDecks.add(deckKey(bot));
        }
        if (all.isEmpty()) {
            return Collections.emptyList();
        }
        double target = rating + targetShift;
        int want = Math.min(Math.max(1, minDistinctDecks), totalDecks.size());
        double below = BELOW;
        double above = ABOVE;
        while (true) {
            java.util.Map<String, Candidate> best = new java.util.LinkedHashMap<>();
            for (Candidate c : all) {
                if (c.rating.rating >= target - below && c.rating.rating <= target + above) {
                    String key = deckKey(c.bot);
                    Candidate prev = best.get(key);
                    if (prev == null || isCloser(c, prev, target)) {
                        best.put(key, c);
                    }
                }
            }
            if (best.size() >= want) {
                List<Candidate> out = new ArrayList<>(best.values());
                out.sort((a, b) -> Double.compare(a.rating.rating, b.rating.rating));
                return out;
            }
            below += WIDEN;
            above += WIDEN;
        }
    }

    static String deckKey(BotRoster.Bot bot) {
        return bot.deck != null && !bot.deck.isEmpty() ? bot.deck : bot.id;
    }

    private static boolean isCloser(Candidate a, Candidate b, double target) {
        double da = Math.abs(a.rating.rating - target);
        double db = Math.abs(b.rating.rating - target);
        if (Math.abs(da - db) > 1e-6) {
            return da < db;
        }
        return a.bot.id.compareTo(b.bot.id) < 0;
    }
}
