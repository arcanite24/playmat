package mage.server.ladder;

import mage.server.career.CareerLeague;
import mage.server.career.RankedRewards;
import org.apache.log4j.Logger;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Ranked (#431): the Open queue, Bo1, against the ladder's bots. A player picks a format and a deck; the server picks a
 * bot near the player's hidden rating (and keeps that pick until it is played, so it can't be dodged), starts the
 * table, and when the match ends moves the rating, the visible rank and pays a rank's first reach. Ranked tables refuse
 * hints, take-backs, practice tools, saves and what-ifs; a game the server loses (a restart) is void.
 */
public final class RankedService {

    private static final Logger logger = Logger.getLogger(RankedService.class);

    /** the Open queue, best of one (Bo3 comes later, #435) */
    public static final String OPEN = "open";
    /** the Career League (#433): the same ladder, played with League decks built from the Career collection */
    public static final String LEAGUE = "league";
    /** the season key of a standing's season-end reward (League packs by peak) */
    public static final String SEASON_END = "season-end";
    /** a pick avoids the bots met in this many last games, when there are others */
    static final int AVOID_RECENT = 3;
    /** the results kept for the client to read after a game */
    static final int RESULTS_KEPT = 500;

    /** the ladder's formats and their XMage deck types */
    static final Map<String, String> DECK_TYPES;

    static {
        Map<String, String> types = new LinkedHashMap<>();
        types.put("standard", "Constructed - Standard");
        types.put("pioneer", "Constructed - Pioneer");
        DECK_TYPES = Collections.unmodifiableMap(types);
    }

    private static volatile RankedService instance;

    private final LadderStore store;
    private final BotRoster roster;
    private final LadderRatings ratings;
    /** tables being played, by table id */
    private final Map<UUID, RankedTable> tables = new ConcurrentHashMap<>();
    /** the ranked tables whose game the Rust host plays */
    private final Set<UUID> onRust = ConcurrentHashMap.newKeySet();
    /** the bot picked for a player's next game, by user|format|queue, until it is played */
    private final Map<String, String> picks = new ConcurrentHashMap<>();
    /** each finished table's result, for the client to show */
    private final Map<UUID, Result> results = Collections.synchronizedMap(new LinkedHashMap<UUID, Result>() {
        @Override
        protected boolean removeEldestEntry(Map.Entry<UUID, Result> eldest) {
            return size() > RESULTS_KEPT;
        }
    });

    RankedService(LadderStore store, BotRoster roster) {
        this.store = store;
        this.roster = roster;
        this.ratings = new LadderRatings(store, roster);
    }

    public static RankedService get() {
        RankedService current = instance;
        if (current == null) {
            synchronized (RankedService.class) {
                current = instance;
                if (current == null) {
                    current = new RankedService(LadderStore.get(), BotRoster.get());
                    // a game still marked playing was lost with the last server run
                    current.voidInterrupted(System.currentTimeMillis());
                    instance = current;
                }
            }
        }
        return current;
    }

    /**
     * Whether the table is ranked, without opening the ladder: a server that never started a ranked game has none.
     * For the checks every game makes (assists, the end of a match).
     */
    public static boolean isRanked(UUID tableId) {
        RankedService current = instance;
        return current != null && current.isRankedTable(tableId);
    }

    /** the Rust host took the table's game */
    public static void hostedOnRust(UUID tableId) {
        RankedService current = instance;
        if (current != null && current.isRankedTable(tableId)) {
            current.onRust.add(tableId);
        }
    }

    /** for tests */
    public static void use(RankedService service) {
        instance = service;
    }

    public static RankedService forTests(LadderStore store, BotRoster roster) {
        return new RankedService(store, roster);
    }

    static String key(String user) {
        return user.toLowerCase(Locale.ENGLISH);
    }

    public static boolean isFormat(String format) {
        return DECK_TYPES.containsKey(format);
    }

    public static boolean isQueue(String queue) {
        return OPEN.equals(queue) || LEAGUE.equals(queue);
    }

    public static String deckType(String format) {
        return DECK_TYPES.get(format);
    }

    public List<String> formats() {
        List<String> out = new ArrayList<>();
        for (String format : DECK_TYPES.keySet()) {
            if (!roster.bots(format).isEmpty()) {
                out.add(format);
            }
        }
        return out;
    }

    public BotRoster roster() {
        return roster;
    }

    public LadderRatings ratings() {
        return ratings;
    }

    public LadderStore store() {
        return store;
    }

    // ---- tables

    /** A ranked table: whose, which format and bot */
    public static final class RankedTable {
        public final UUID tableId;
        public final String user;
        public final String format;
        public final String queue;
        public final String botId;
        /** the season the game started in, which it counts for */
        public final int season;

        RankedTable(UUID tableId, String user, String format, String queue, String botId, int season) {
            this.tableId = tableId;
            this.user = user;
            this.format = format;
            this.queue = queue;
            this.botId = botId;
            this.season = season;
        }
    }

    public boolean isRankedTable(UUID tableId) {
        return tableId != null && tables.containsKey(tableId);
    }

    /** whether the player is in a ranked game now (one at a time) */
    public boolean playing(String user) {
        String k = key(user);
        for (RankedTable table : tables.values()) {
            if (table.user.equals(k)) {
                return true;
            }
        }
        return false;
    }

    /**
     * The bot the player's next game in `format` is against: the one already picked, or one drawn now from the bots
     * near the player's rating, avoiding the last few met when it can.
     */
    public BotRoster.Bot pick(String user, String format, String queue, long now) throws SQLException {
        String pickKey = key(user) + "|" + format + "|" + queue;
        String picked = picks.get(pickKey);
        if (picked != null && roster.bot(picked) != null) {
            return roster.bot(picked);
        }
        Glicko2.Rating rating = ratings.player(user, format, queue, now);
        List<LadderRatings.Candidate> candidates = ratings.candidates(format, queue, rating.rating);
        if (candidates.isEmpty()) {
            throw new IllegalStateException("There are no ladder bots for " + format);
        }
        Set<String> recent = new HashSet<>();
        for (LadderStore.GameRow game : store.games(key(user), format, queue, AVOID_RECENT)) {
            recent.add(game.bot);
        }
        List<LadderRatings.Candidate> fresh = new ArrayList<>();
        for (LadderRatings.Candidate c : candidates) {
            if (!recent.contains(c.bot.id)) {
                fresh.add(c);
            }
        }
        List<LadderRatings.Candidate> from = fresh.isEmpty() ? candidates : fresh;
        // RandomUtil, so a test can seed the draw
        BotRoster.Bot bot = from.get(mage.util.RandomUtil.nextInt(from.size())).bot;
        picks.put(pickKey, bot.id);
        return bot;
    }

    /** the table is about to be played: called before anyone sits, so the game starts as ranked */
    public void register(UUID tableId, String user, String format, String queue, BotRoster.Bot bot, String deckName, long now) throws SQLException {
        int season = season(now);
        RankedTable table = new RankedTable(tableId, key(user), format, queue, bot.id, season);
        LadderStore.GameRow row = new LadderStore.GameRow();
        row.id = tableId.toString();
        row.user = table.user;
        row.format = format;
        row.queue = queue;
        row.season = season;
        row.bot = bot.id;
        row.deck = deckName;
        row.started = now;
        store.startGame(row);
        tables.put(tableId, table);
        picks.remove(key(user) + "|" + format + "|" + queue);
    }

    /** the table never got going (a seat refused, the start failed): nothing counts; the pick stays */
    public void forget(UUID tableId, long now) {
        RankedTable table = tables.remove(tableId);
        onRust.remove(tableId);
        if (table == null) {
            return;
        }
        picks.put(table.user + "|" + table.format + "|" + table.queue, table.botId);
        try {
            store.endGame(tableId.toString(), "void", now, null, null, null, null, null);
        } catch (SQLException e) {
            logger.warn("Ranked: an unstarted game couldn't be voided", e);
        }
    }

    /** games marked playing from before this server started: void */
    void voidInterrupted(long now) {
        try {
            for (LadderStore.GameRow game : store.playing()) {
                if (!tables.containsKey(UUID.fromString(game.id))) {
                    store.endGame(game.id, "void", now, null, null, null, null, null);
                }
            }
        } catch (SQLException | IllegalArgumentException e) {
            logger.warn("Ranked: interrupted games couldn't be voided", e);
        }
    }

    // ---- results

    /** What a ranked game did */
    public static final class Result {
        public String tableId;
        public String format;
        public String queue;
        /** won, lost or drew */
        public String status;
        public String botId;
        public int stepBefore;
        public int stepAfter;
        public int delta;
        public boolean bonus;
        public int streak;
        /** the ranks reached for the first time this game (bronze to mythic) */
        public List<String> reached = new ArrayList<>();
        public RankedRewards.Paid paid;
        public double ratingBefore;
        public double ratingAfter;
    }

    /**
     * The match at a ranked table ended: `score` is the player's (1 won, 0.5 drew, 0 lost). Moves the rating and the
     * rank, pays first reaches, and keeps the result for the client. Once per table.
     */
    public synchronized Result matchEnded(UUID tableId, double score, long now) throws SQLException {
        RankedTable table = tables.remove(tableId);
        if (table == null) {
            return null;
        }
        String engine = onRust.remove(tableId) ? "rust" : "java";
        rollover(table.user, table.format, table.queue, table.season, now);
        LadderStore.Standing standing = store.standing(table.user, table.format, table.queue, table.season);
        boolean first = standing == null;
        if (first) {
            standing = new LadderStore.Standing();
            standing.user = table.user;
            standing.format = table.format;
            standing.queue = table.queue;
            standing.season = table.season;
        }
        int peakBefore = standing.peak;
        LadderRatings.Outcome outcome = ratings.record(table.user, table.botId, table.queue, score, now);
        LadderRank.Move move = LadderRank.after(standing.step, standing.streak, score);
        Result result = new Result();
        result.tableId = tableId.toString();
        result.format = table.format;
        result.queue = table.queue;
        result.status = score >= 1 ? "won" : score <= 0 ? "lost" : "drew";
        result.botId = table.botId;
        result.stepBefore = standing.step;
        result.stepAfter = move.step;
        result.delta = move.delta;
        result.bonus = move.bonus;
        result.streak = move.streak;
        result.ratingBefore = outcome.playerBefore.rating;
        result.ratingAfter = outcome.playerAfter.rating;
        // a rank is reached the first time the peak enters it; Bronze with the first game
        int fromRank = first ? -1 : LadderRank.rank(standing.peak);
        int toRank = LadderRank.rank(Math.max(standing.peak, move.step));
        for (int r = fromRank + 1; r <= toRank; r++) {
            result.reached.add(LadderRank.RANKS[r]);
        }
        standing.step = move.step;
        standing.peak = Math.max(standing.peak, move.step);
        standing.streak = move.streak;
        if (score >= 1) {
            standing.won++;
        } else if (score <= 0) {
            standing.lost++;
        } else {
            standing.drew++;
        }
        standing.updated = now;
        store.putStanding(standing);
        store.endGame(result.tableId, result.status, now, result.stepBefore, result.stepAfter, result.ratingBefore, result.ratingAfter, engine);
        if (LEAGUE.equals(table.queue)) {
            payLeague(table, result, first, peakBefore, standing, score >= 1, now);
        } else {
            payOpen(table, result, now);
        }
        {
            logger.info("Ranked " + table.format + ": " + table.user + " " + result.status + " against " + table.botId
                    + " on " + engine + ", " + LadderRank.label(result.stepBefore) + " -> " + LadderRank.label(result.stepAfter));
        }
        results.put(tableId, result);
        return result;
    }

    private void payOpen(RankedTable table, Result result, long now) throws SQLException {
        for (String rank : result.reached) {
            // the Open queue's rank rewards pay once per account, whatever the season
            if (store.payReward(table.user, table.format, table.queue, LadderStore.LIFETIME, rank, now)) {
                try {
                    RankedRewards.Paid paid = RankedRewards.pay(table.user, rank, table.format, now);
                    if (result.paid == null) {
                        result.paid = paid;
                    } else {
                        result.paid.cosmetics.addAll(paid.cosmetics);
                        result.paid.tokens += paid.tokens;
                    }
                } catch (SQLException | RuntimeException e) {
                    logger.error("Ranked: reaching " + rank + " couldn't be paid to " + table.user, e);
                }
            }
        }
    }

    /** a League game: tiers gained pay the format's packs, ranks pay cosmetics and wildcards, wins pay wildcards */
    private void payLeague(RankedTable table, Result result, boolean first, int peakBefore, LadderStore.Standing standing, boolean won, long now) throws SQLException {
        List<Integer> tiers = new ArrayList<>();
        for (int tier : CareerLeague.tiersGained(peakBefore, standing.peak, first)) {
            if (store.payReward(table.user, table.format, table.queue, table.season, "tier-" + tier, now)) {
                tiers.add(tier);
            }
        }
        List<String> ranks = new ArrayList<>();
        for (String rank : result.reached) {
            if (store.payReward(table.user, table.format, table.queue, table.season, rank, now)) {
                ranks.add(rank);
            }
        }
        try {
            result.paid = CareerLeague.pay(table.user, table.format, tiers, ranks, standing.won, won, now);
        } catch (SQLException | RuntimeException e) {
            logger.error("League: a game's rewards couldn't be paid to " + table.user, e);
        }
    }

    // ---- seasons (#434)

    /** the season now: the calendar's, never below a season already played (a changed calendar can't take one back) */
    public int season(long now) throws SQLException {
        return Math.max(Seasons.get().index(now), store.maxSeason());
    }

    /**
     * The player's first touch of a format and queue in `season`: when they had a standing in an earlier one, the new
     * season places them from where that one ended ({@link LadderRank#placement}), with its peak at the placement so
     * the ranks and tiers below it don't pay again, and a League pays the ended season's packs by its peak. Safe to
     * call any number of times: it does this once.
     */
    public synchronized void rollover(String user, String format, String queue, int season, long now) throws SQLException {
        String k = key(user);
        if (store.standing(k, format, queue, season) != null) {
            return;
        }
        LadderStore.Standing last = store.before(k, format, queue, season);
        if (last == null) {
            return;
        }
        LadderStore.Standing placed = new LadderStore.Standing();
        placed.user = k;
        placed.format = format;
        placed.queue = queue;
        placed.season = season;
        placed.step = LadderRank.placement(last.step);
        placed.peak = placed.step;
        placed.updated = now;
        store.putStanding(placed);
        logger.info("Ladder " + format + " " + queue + ": " + k + " starts " + Seasons.name(season) + " at "
                + LadderRank.label(placed.step) + " (ended " + Seasons.name(last.season) + " at " + LadderRank.label(last.step) + ")");
        if (LEAGUE.equals(queue) && store.payReward(k, format, queue, last.season, SEASON_END, now)) {
            try {
                CareerLeague.paySeasonEnd(k, format, last.peak, now);
            } catch (SQLException | RuntimeException e) {
                logger.error("League: " + Seasons.name(last.season) + "'s end couldn't be paid to " + k, e);
            }
        }
    }

    /** the result of a finished ranked table, if it is still kept */
    public Result result(UUID tableId) {
        return results.get(tableId);
    }
}
