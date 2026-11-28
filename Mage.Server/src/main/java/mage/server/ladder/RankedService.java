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
 * Ranked (#431): the Open queue, Bo1, and the Bo3 queue (#435), against the ladder's bots. A player picks a format and a deck; the server picks a
 * bot near the player's hidden rating (and keeps that pick until it is played, so it can't be dodged), starts the
 * table, and when the match ends moves the rating, the visible rank and pays a rank's first reach. Ranked tables refuse
 * hints, take-backs, practice tools, saves and what-ifs; a game the server loses (a restart) is void.
 */
public final class RankedService {

    private static final Logger logger = Logger.getLogger(RankedService.class);

    /** the Open queue, best of one */
    public static final String OPEN = "open";
    /**
     * the Bo3 queue (#435): the Open queue's decks and bots, best of three with sideboarding, its own rank and rating.
     * Its ranks pay the same rewards as the Open queue's, once for both.
     */
    public static final String BO3 = "bo3";
    /** the Career League (#433): the same ladder, played with League decks built from the Career collection */
    public static final String LEAGUE = "league";
    /** 4-player Commander Free-For-All Ranked Pod queue with placement-based scoring */
    public static final String POD = "pod";
    /** the season key of a standing's season-end reward (League packs by peak) */
    public static final String SEASON_END = "season-end";
    /**
     * The Limited rank (#435): a format without bots, climbed by Career's sealed and draft runs, one move a run
     * ({@link #limitedRun}). It has the Open queue's ranks, seasons and boards but no queue to play and no rating.
     */
    public static final String LIMITED = "limited";
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
        // Modern (#435): the Open and Bo3 queues, with the ladder copies of its school decks (no League)
        types.put("modern", "Constructed - Modern");
        // Commander (Phase 3): 1v1 Duel (open), 4-Player Pod (pod), and Precon Upgrade League (league); stays on Java
        types.put("commander", "Variant Magic - Commander");
        // Pauper, Brawl & Legacy (Phase 4): full ladder coverage across Constructed and singleton formats
        types.put("pauper", "Constructed - Pauper");
        types.put("brawl", "Variant Magic - Brawl");
        types.put("legacy", "Constructed - Legacy");
        DECK_TYPES = Collections.unmodifiableMap(types);
    }

    private static volatile RankedService instance;

    private final LadderStore store;
    private final BotRoster roster;
    private final LadderRatings ratings;
    private final RankedMatchmaker matchmaker;
    /** tables being played, by table id */
    private final Map<UUID, RankedTable> tables = new ConcurrentHashMap<>();
    /** the ranked tables whose game the Rust host plays */
    private final Set<UUID> onRust = ConcurrentHashMap.newKeySet();
    /** cached opponent pick for a player's next game, by user|format|queue (persisted in ladder_picks) */
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
        this.matchmaker = new RankedMatchmaker(this.ratings, roster);
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
    /** each ranked table's latest game, by table id: the rewards screen asks by table, Career pays by game */
    private final Map<UUID, UUID> lastGames = Collections.synchronizedMap(new LinkedHashMap<UUID, UUID>() {
        @Override
        protected boolean removeEldestEntry(Map.Entry<UUID, UUID> eldest) {
            return size() > RESULTS_KEPT;
        }
    });

    /** a game at a ranked table ended; its payout is keyed by the game */
    public static void gameEnded(UUID tableId, UUID gameId) {
        RankedService current = instance;
        if (current != null && current.isRankedTable(tableId)) {
            current.lastGames.put(tableId, gameId);
            try {
                mage.server.career.CareerStore.get().putRankedGame(tableId.toString(), gameId.toString(), System.currentTimeMillis());
            } catch (java.sql.SQLException | RuntimeException e) {
                logger.warn("Ranked: table " + tableId + "'s latest game can't be kept: " + e.getMessage());
            }
        }
    }

    /** a ranked table's latest game (the client asks for rewards by table), or null for any other key */
    public static UUID lastGame(String tableId) {
        RankedService current = instance;
        if (current == null || tableId == null) {
            return null;
        }
        try {
            UUID game = current.lastGames.get(UUID.fromString(tableId));
            if (game != null) {
                return game;
            }
            // after a restart: the store keeps it (as the Rust platform reads it)
            String kept = mage.server.career.CareerStore.get().rankedGame(tableId);
            return kept == null ? null : UUID.fromString(kept);
        } catch (RuntimeException | java.sql.SQLException e) {
            return null;
        }
    }

    /** the search settings the ranked table's bot is pinned to (#435), for the Rust host; null at other tables */
    public static String pinOf(UUID tableId) {
        BotRoster.Bot bot = botOf(tableId);
        return bot == null ? null : bot.ai;
    }

    /** the AI playbook id of the ranked table's bot, for the Rust host and Bo3 sideboarder; null when none */
    public static String playbookOf(UUID tableId) {
        BotRoster.Bot bot = botOf(tableId);
        return bot == null ? null : bot.playbook;
    }

    /** the bot playing at a live ranked table, or null */
    public static BotRoster.Bot botOf(UUID tableId) {
        RankedService current = instance;
        RankedTable table = current == null ? null : current.tables.get(tableId);
        return table == null ? null : current.roster.bot(table.botId);
    }

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

    /** a format with a rank: the constructed ladders and Limited */
    public static boolean isFormat(String format) {
        return DECK_TYPES.containsKey(format) || LIMITED.equals(format);
    }

    public static boolean isQueue(String queue) {
        return OPEN.equals(queue) || BO3.equals(queue) || LEAGUE.equals(queue) || POD.equals(queue);
    }

    /** games a match needs won: two in the Bo3 queue, one elsewhere */
    public static int winsNeeded(String queue) {
        return BO3.equals(queue) ? 2 : 1;
    }

    /** the queue whose bots a queue plays: Bo3 and Pod meet the Open queue's bots */
    public static String botQueue(String queue) {
        return BO3.equals(queue) || POD.equals(queue) ? OPEN : queue;
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
        public final List<String> podBotIds;
        public final boolean pod;
        /** the season the game started in, which it counts for */
        public final int season;

        RankedTable(UUID tableId, String user, String format, String queue, String botId, int season) {
            this(tableId, user, format, queue, botId, Collections.emptyList(), false, season);
        }

        RankedTable(UUID tableId, String user, String format, String queue, String botId, List<String> podBotIds, boolean pod, int season) {
            this.tableId = tableId;
            this.user = user;
            this.format = format;
            this.queue = queue;
            this.botId = botId;
            this.podBotIds = podBotIds == null ? Collections.emptyList() : Collections.unmodifiableList(new ArrayList<>(podBotIds));
            this.pod = pod;
            this.season = season;
        }
    }

    public boolean isPodTable(UUID tableId) {
        RankedTable table = tableId == null ? null : tables.get(tableId);
        return table != null && table.pod;
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
     * The bot the player's next game in `format` is against: the one already picked (persisted in SQLite so it
     * survives restarts and can't be dodged), or one drawn now by {@link RankedMatchmaker} with deck deduplication
     * and guaranteed post-loss rotation.
     */
    public BotRoster.Bot pick(String user, String format, String queue, long now) throws SQLException {
        String userKey = key(user);
        String pickKey = userKey + "|" + format + "|" + queue;
        String picked = picks.get(pickKey);
        if (picked == null) {
            picked = store.pick(userKey, format, queue);
            if (picked != null) {
                picks.put(pickKey, picked);
            }
        }
        if (picked != null) {
            BotRoster.Bot kept = roster.bot(picked);
            if (kept != null && kept.plays(queue) && format.equals(kept.format)) {
                return kept;
            }
            picks.remove(pickKey);
            store.clearPick(userKey, format, queue);
        }
        Glicko2.Rating rating = ratings.player(user, format, queue, now);
        List<LadderStore.GameRow> recent = store.finishedGames(userKey, format, queue, RankedMatchmaker.HISTORY_LIMIT);
        BotRoster.Bot bot = pickWithBracketProtection(userKey, format, queue, rating.rating, recent, now);
        picks.put(pickKey, bot.id);
        store.putPick(userKey, format, queue, bot.id, now);
        return bot;
    }

    /**
     * For Commander in Bronze/Silver, avoids pairing players against high-power `meta` combo decks so casual and
     * precon decks face fair Precon/Upgraded/Focused pods until Gold+.
     */
    private BotRoster.Bot pickWithBracketProtection(String userKey, String format, String queue, double rating,
                                                    List<LadderStore.GameRow> recent, long now) throws SQLException {
        if ("commander".equals(format)) {
            int season = season(now);
            LadderStore.Standing standing = store.standing(userKey, format, queue, season);
            int rank = standing == null ? 0 : LadderRank.rank(standing.step);
            if (rank <= 1) {
                // Bronze & Silver: prefer starter / budget / rogue / tier2 decks (no high-power meta combos)
                List<BotRoster.Bot> friendly = new ArrayList<>();
                for (BotRoster.Bot candidate : roster.bots(format, queue)) {
                    if (!"meta".equals(candidate.effectiveTier())) {
                        friendly.add(candidate);
                    }
                }
                if (!friendly.isEmpty()) {
                    Set<String> recentDecks = new HashSet<>();
                    for (int i = 0; i < Math.min(3, recent.size()); i++) {
                        if (recent.get(i).botDeck != null) {
                            recentDecks.add(recent.get(i).botDeck);
                        }
                    }
                    Map<String, Double> botRatingMap = new java.util.HashMap<>();
                    for (BotRoster.Bot candidate : friendly) {
                        botRatingMap.put(candidate.id, ratings.bot(candidate, queue).rating);
                    }
                    friendly.sort((a, b) -> Double.compare(
                            Math.abs(botRatingMap.get(a.id) - rating),
                            Math.abs(botRatingMap.get(b.id) - rating)));
                    List<BotRoster.Bot> pool = new ArrayList<>();
                    for (BotRoster.Bot candidate : friendly) {
                        if (!recentDecks.contains(candidate.deck)) {
                            pool.add(candidate);
                        }
                        if (pool.size() >= 6) {
                            break;
                        }
                    }
                    if (pool.isEmpty()) {
                        pool = friendly.subList(0, Math.min(4, friendly.size()));
                    }
                    return pool.get(java.util.concurrent.ThreadLocalRandom.current().nextInt(pool.size()));
                }
            }
        }
        return matchmaker.pick(format, queue, rating, recent);
    }

    /** Picks 3 distinct-deck Commander bots for a 4-player Ranked or League pod, anchored on the player's primary pick. */
    public List<BotRoster.Bot> pickPod(String user, String format, String queue, long now) throws SQLException {
        BotRoster.Bot primary = pick(user, format, queue, now);
        List<BotRoster.Bot> out = new ArrayList<>();
        out.add(primary);
        Set<String> usedDecks = new HashSet<>();
        if (primary.deck != null) {
            usedDecks.add(primary.deck);
        }
        String userKey = key(user);
        LadderStore.Standing standing = store.standing(userKey, format, queue, season(now));
        boolean casualBracket = "commander".equals(format) && (standing == null || LadderRank.rank(standing.step) <= 1);
        double target = ratings.player(user, format, queue, now).rating;
        List<BotRoster.Bot> pool = new ArrayList<>();
        for (BotRoster.Bot candidate : roster.bots(format, queue)) {
            if (candidate.deck != null && usedDecks.contains(candidate.deck)) {
                continue;
            }
            if (casualBracket && "meta".equals(candidate.effectiveTier())) {
                continue;
            }
            pool.add(candidate);
        }
        if (pool.size() < 2) {
            pool = new ArrayList<>(roster.bots(format, queue));
            pool.removeIf(b -> b.deck != null && usedDecks.contains(b.deck));
        }
        Map<String, Double> poolRatings = new java.util.HashMap<>();
        for (BotRoster.Bot candidate : pool) {
            poolRatings.put(candidate.id, ratings.bot(candidate, queue).rating);
        }
        pool.sort((a, b) -> Double.compare(
                Math.abs(poolRatings.get(a.id) - target),
                Math.abs(poolRatings.get(b.id) - target)));
        List<BotRoster.Bot> window = new ArrayList<>(pool.subList(0, Math.min(10, pool.size())));
        Collections.shuffle(window);
        for (BotRoster.Bot candidate : window) {
            if (candidate.deck == null || usedDecks.add(candidate.deck)) {
                out.add(candidate);
                if (out.size() == 3) {
                    break;
                }
            }
        }
        return out;
    }

    /** the table is about to be played: called before anyone sits, so the game starts as ranked */
    public void register(UUID tableId, String user, String format, String queue, BotRoster.Bot bot, String deckName, long now) throws SQLException {
        registerInternal(tableId, user, format, queue, bot, Collections.emptyList(), false, deckName, now);
    }

    /** Registers a 4-player Commander Ranked or League pod table with its 3 AI bots. */
    public void registerPod(UUID tableId, String user, String format, String queue, List<BotRoster.Bot> bots, String deckName, long now) throws SQLException {
        BotRoster.Bot primary = bots.get(0);
        List<String> ids = new ArrayList<>();
        for (BotRoster.Bot b : bots) {
            ids.add(b.id);
        }
        registerInternal(tableId, user, format, queue, primary, ids, true, deckName, now);
    }

    private void registerInternal(UUID tableId, String user, String format, String queue, BotRoster.Bot bot,
                                  List<String> podBotIds, boolean pod, String deckName, long now) throws SQLException {
        int season = season(now);
        String userKey = key(user);
        RankedTable table = new RankedTable(tableId, userKey, format, queue, bot.id, podBotIds, pod, season);
        LadderStore.GameRow row = new LadderStore.GameRow();
        row.id = tableId.toString();
        row.user = table.user;
        row.format = format;
        row.queue = queue;
        row.season = season;
        row.bot = bot.id;
        row.botDeck = bot.deck;
        row.botArchetype = bot.effectiveArchetype();
        row.deck = deckName;
        row.started = now;
        store.startGame(row);
        tables.put(tableId, table);
        picks.remove(userKey + "|" + format + "|" + queue);
        store.clearPick(userKey, format, queue);
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
            store.putPick(table.user, table.format, table.queue, table.botId, now);
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
        /** all 3 pod bot IDs when played in a 4-player Commander pod */
        public List<String> podBotIds = new ArrayList<>();
        /** 1..4 placement in a 4-player Commander pod, or null for 1v1 */
        public Integer placement;
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

    public synchronized Result matchEnded(UUID tableId, double score, long now) throws SQLException {
        return matchEnded(tableId, score, now, null, null);
    }

    /**
     * The match at a ranked table ended: `score` is the player's (1 won, 0.5 drew, 0 lost). Moves the rating and the
     * rank, pays first reaches, and keeps the result for the client. Once per table.
     */
    public synchronized Result matchEnded(UUID tableId, double score, long now, Integer turns, String gameScore) throws SQLException {
        return matchEndedInternal(tableId, score, null, now, turns, gameScore);
    }

    /**
     * Records a finished 4-player Commander Ranked or League pod with exact placement (1 = winner, 2..4 = finish order).
     */
    public synchronized Result matchEndedPod(UUID tableId, int placement, long now, Integer turns) throws SQLException {
        int clamped = Math.max(1, Math.min(4, placement));
        double score = clamped == 1 ? 1.0 : clamped == 2 ? 0.5 : 0.0;
        return matchEndedInternal(tableId, score, clamped, now, turns, "#" + clamped);
    }

    private synchronized Result matchEndedInternal(UUID tableId, double score, Integer placement, long now, Integer turns, String gameScore) throws SQLException {
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
        LadderRank.Move move = placement != null
                ? LadderRank.afterPod(standing.step, standing.streak, placement)
                : LadderRank.after(standing.step, standing.streak, score);
        Result result = new Result();
        result.tableId = tableId.toString();
        result.format = table.format;
        result.queue = table.queue;
        result.status = placement != null
                ? (placement == 1 ? "won" : "lost")
                : (score >= 1 ? "won" : score <= 0 ? "lost" : "drew");
        result.botId = table.botId;
        result.podBotIds = new ArrayList<>(table.podBotIds);
        result.placement = placement;
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
        store.endGame(result.tableId, result.status, now, result.stepBefore, result.stepAfter, result.ratingBefore, result.ratingAfter, engine, turns, gameScore);
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
            // the Open queue's rank rewards pay once per account, whatever the season, and once for Open and Bo3
            if (store.payReward(table.user, table.format, OPEN, LadderStore.LIFETIME, rank, now)) {
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

    // ---- the Limited rank (#435)

    /** the pips a finished Limited run is worth: its wins less its losses, and a bad run costs one pip at most */
    static int limitedPips(int wins, int losses) {
        return Math.max(-1, wins - losses);
    }

    /**
     * A Career sealed or draft run ended at `wins`-`losses`: moves the player's Limited rank once, as that many games
     * would ({@link #limitedPips}; no streak bonus, a loss never drops below the floors of {@link LadderRank#after}),
     * records the run as a ladder game and pays first reaches as the Open queue does. Once per run.
     *
     * @return the move, or null when this run was counted before
     */
    public synchronized Result limitedRun(String user, String runId, String kind, String shape, String deckName,
                                          int wins, int losses, long now) throws SQLException {
        String k = key(user);
        String id = "limited-" + runId;
        if (store.game(id) != null) {
            return null;
        }
        int season = season(now);
        rollover(k, LIMITED, OPEN, season, now);
        LadderStore.Standing standing = store.standing(k, LIMITED, OPEN, season);
        boolean first = standing == null;
        if (first) {
            standing = new LadderStore.Standing();
            standing.user = k;
            standing.format = LIMITED;
            standing.queue = OPEN;
            standing.season = season;
        }
        int pips = limitedPips(wins, losses);
        int step = standing.step;
        if (pips > 0) {
            for (int i = 0; i < pips; i++) {
                step = LadderRank.after(step, 0, 1).step;
            }
        } else if (pips < 0) {
            step = LadderRank.after(step, 0, 0).step;
        }
        Result result = new Result();
        result.tableId = id;
        result.format = LIMITED;
        result.queue = OPEN;
        result.status = wins > losses ? "won" : wins < losses ? "lost" : "drew";
        result.botId = "limited:" + kind + "-" + shape;
        result.stepBefore = standing.step;
        result.stepAfter = step;
        result.delta = step - standing.step;
        int fromRank = first ? -1 : LadderRank.rank(standing.peak);
        int toRank = LadderRank.rank(Math.max(standing.peak, step));
        for (int r = fromRank + 1; r <= toRank; r++) {
            result.reached.add(LadderRank.RANKS[r]);
        }
        standing.step = step;
        standing.peak = Math.max(standing.peak, step);
        standing.streak = 0;
        if ("won".equals(result.status)) {
            standing.won++;
        } else if ("lost".equals(result.status)) {
            standing.lost++;
        } else {
            standing.drew++;
        }
        standing.updated = now;
        LadderStore.GameRow row = new LadderStore.GameRow();
        row.id = id;
        row.user = k;
        row.format = LIMITED;
        row.queue = OPEN;
        row.season = season;
        row.bot = result.botId;
        row.deck = deckName;
        row.started = now;
        store.startGame(row);
        store.putStanding(standing);
        store.endGame(id, result.status, now, result.stepBefore, result.stepAfter, null, null, null);
        payOpen(new RankedTable(null, k, LIMITED, OPEN, result.botId, season), result, now);
        logger.info("Ranked limited: " + k + " " + wins + "-" + losses + " in a " + kind + " " + shape + " run, "
                + LadderRank.label(result.stepBefore) + " -> " + LadderRank.label(result.stepAfter));
        return result;
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
