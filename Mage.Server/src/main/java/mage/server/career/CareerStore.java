package mage.server.career;

import org.apache.log4j.Logger;

import java.io.File;
import java.security.SecureRandom;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.Locale;
import java.util.Map;

/**
 * Career progress per account: the collection, coins, wildcards, XP and every payout. Kept in db/career.db (SQLite),
 * apart from the card database, with a versioned schema: each migration runs once, in order, inside a transaction.
 * Every change that pays or spends runs in one transaction, so a crash never leaves half a reward.
 */
public final class CareerStore implements AutoCloseable {

    private static final Logger logger = Logger.getLogger(CareerStore.class);
    private static final String DEFAULT_URL = "jdbc:sqlite:./db/career.db";

    /** migrations, oldest first; the schema version is the number of migrations applied */
    private static final String[][] MIGRATIONS = {
            {
                    "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
                    "CREATE TABLE profile (user TEXT PRIMARY KEY, created INTEGER NOT NULL, starter TEXT NOT NULL,"
                            + " coins INTEGER NOT NULL DEFAULT 0, xp INTEGER NOT NULL DEFAULT 0,"
                            + " wins INTEGER NOT NULL DEFAULT 0, losses INTEGER NOT NULL DEFAULT 0,"
                            + " wc_common INTEGER NOT NULL DEFAULT 0, wc_uncommon INTEGER NOT NULL DEFAULT 0,"
                            + " wc_rare INTEGER NOT NULL DEFAULT 0, wc_mythic INTEGER NOT NULL DEFAULT 0)",
                    "CREATE TABLE cards (user TEXT NOT NULL, set_code TEXT NOT NULL, card_number TEXT NOT NULL,"
                            + " name TEXT NOT NULL, rarity TEXT NOT NULL, count INTEGER NOT NULL,"
                            + " PRIMARY KEY (user, set_code, card_number))",
                    "CREATE INDEX cards_by_name ON cards (user, name)",
                    "CREATE TABLE payouts (match_key TEXT PRIMARY KEY, user TEXT NOT NULL, at INTEGER NOT NULL,"
                            + " opponent TEXT NOT NULL, won INTEGER NOT NULL, coins INTEGER NOT NULL, xp INTEGER NOT NULL,"
                            + " note TEXT)",
                    "CREATE INDEX payouts_by_user ON payouts (user, at)"
            },
            {
                    // M8: quests, levels, achievements, lifetime stats, cosmetic unlocks, the weekly goal
                    "ALTER TABLE profile ADD COLUMN pack_tokens INTEGER NOT NULL DEFAULT 0",
                    "ALTER TABLE profile ADD COLUMN level_paid INTEGER NOT NULL DEFAULT 1",
                    "ALTER TABLE profile ADD COLUMN quest_day TEXT",
                    "ALTER TABLE profile ADD COLUMN reroll_day TEXT",
                    "ALTER TABLE profile ADD COLUMN count_ai_games INTEGER NOT NULL DEFAULT 0",
                    "CREATE TABLE stats (user TEXT NOT NULL, counter TEXT NOT NULL, value INTEGER NOT NULL,"
                            + " PRIMARY KEY (user, counter))",
                    "CREATE TABLE quests (user TEXT NOT NULL, slot INTEGER NOT NULL, quest TEXT NOT NULL,"
                            + " progress INTEGER NOT NULL DEFAULT 0, assigned TEXT NOT NULL, PRIMARY KEY (user, slot))",
                    "CREATE TABLE achievements (user TEXT NOT NULL, id TEXT NOT NULL, at INTEGER NOT NULL,"
                            + " PRIMARY KEY (user, id))",
                    "CREATE TABLE unlocks (user TEXT NOT NULL, kind TEXT NOT NULL, item TEXT NOT NULL, at INTEGER NOT NULL,"
                            + " PRIMARY KEY (user, kind, item))",
                    "CREATE TABLE weekly (user TEXT NOT NULL, week TEXT NOT NULL, wins INTEGER NOT NULL DEFAULT 0,"
                            + " PRIMARY KEY (user, week))",
                    "CREATE TABLE progress_games (game_key TEXT PRIMARY KEY, user TEXT NOT NULL, at INTEGER NOT NULL,"
                            + " details TEXT)",
                    "CREATE INDEX progress_games_by_user ON progress_games (user, at)"
            },
            {
                    // M9: campaigns, shop sets opened by campaigns, gauntlet runs, puzzles, weekly challenges, limited runs
                    "CREATE TABLE campaign_nodes (user TEXT NOT NULL, campaign TEXT NOT NULL, node TEXT NOT NULL,"
                            + " done_at INTEGER NOT NULL, choice TEXT, PRIMARY KEY (user, campaign, node))",
                    "CREATE TABLE set_unlocks (user TEXT NOT NULL, set_code TEXT NOT NULL, at INTEGER NOT NULL,"
                            + " PRIMARY KEY (user, set_code))",
                    "CREATE TABLE runs (id TEXT PRIMARY KEY, user TEXT NOT NULL, kind TEXT NOT NULL, started INTEGER NOT NULL,"
                            + " ended INTEGER, state TEXT NOT NULL, wins INTEGER NOT NULL DEFAULT 0, losses INTEGER NOT NULL DEFAULT 0,"
                            + " data TEXT NOT NULL)",
                    "CREATE INDEX runs_by_user ON runs (user, kind, state)",
                    "CREATE TABLE puzzle_results (user TEXT NOT NULL, puzzle TEXT NOT NULL, attempts INTEGER NOT NULL DEFAULT 0,"
                            + " solved_at INTEGER, stars INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (user, puzzle))",
                    "CREATE TABLE challenge_results (user TEXT NOT NULL, week TEXT NOT NULL, attempts INTEGER NOT NULL DEFAULT 0,"
                            + " won INTEGER NOT NULL DEFAULT 0, best_turns INTEGER, best_life INTEGER, at INTEGER NOT NULL,"
                            + " PRIMARY KEY (user, week))",
                    "CREATE INDEX challenge_results_by_week ON challenge_results (week, won, best_turns)"
            },
            {
                    // M24: progression for every game. A profile without a starter ("") hasn't opened the collection yet
                    "ALTER TABLE profile ADD COLUMN show_progress INTEGER NOT NULL DEFAULT 1",
                    "ALTER TABLE progress_games ADD COLUMN kind TEXT",
                    "ALTER TABLE progress_games ADD COLUMN won INTEGER",
                    "ALTER TABLE progress_games ADD COLUMN coins INTEGER NOT NULL DEFAULT 0",
                    "ALTER TABLE progress_games ADD COLUMN xp INTEGER NOT NULL DEFAULT 0",
                    "ALTER TABLE progress_games ADD COLUMN reason TEXT",
                    "ALTER TABLE progress_games ADD COLUMN revoked INTEGER NOT NULL DEFAULT 0",
                    "CREATE TABLE pair_games (day TEXT NOT NULL, pair TEXT NOT NULL, games INTEGER NOT NULL DEFAULT 0,"
                            + " PRIMARY KEY (day, pair))",
                    "CREATE TABLE daily (user TEXT NOT NULL, day TEXT NOT NULL, wins INTEGER NOT NULL DEFAULT 0,"
                            + " games INTEGER NOT NULL DEFAULT 0, person_games INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (user, day))",
                    "CREATE TABLE career_tables (table_id TEXT PRIMARY KEY, data TEXT NOT NULL, created INTEGER NOT NULL)",
                    "CREATE TABLE decks (user TEXT PRIMARY KEY, deck TEXT NOT NULL, updated INTEGER NOT NULL)"
            }
    };

    /** every table that holds an account's rows, for deleting a Career */
    private static final String[] USER_TABLES = {"profile", "cards", "payouts", "stats", "quests", "achievements",
            "unlocks", "weekly", "progress_games", "campaign_nodes", "set_unlocks", "runs", "puzzle_results", "challenge_results",
            "daily", "decks"};

    private static volatile CareerStore instance;

    private final Connection connection;

    public CareerStore(String jdbcUrl) throws SQLException {
        connection = DriverManager.getConnection(jdbcUrl);
        try (Statement statement = connection.createStatement()) {
            statement.execute("PRAGMA foreign_keys = ON");
            statement.execute("PRAGMA busy_timeout = 5000");
        }
        migrate();
    }

    public static CareerStore get() {
        CareerStore current = instance;
        if (current == null) {
            synchronized (CareerStore.class) {
                current = instance;
                if (current == null) {
                    new File("db").mkdirs();
                    try {
                        current = new CareerStore(DEFAULT_URL);
                    } catch (SQLException | LinkageError e) {
                        logger.error("Career database can't be opened", e);
                        throw new IllegalStateException("Career progress can't be stored on this server right now");
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    /** for tests: use this store instead of db/career.db */
    public static void use(CareerStore store) {
        instance = store;
    }

    public synchronized int schemaVersion() throws SQLException {
        try (Statement statement = connection.createStatement()) {
            statement.execute("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)");
            try (ResultSet rows = statement.executeQuery("SELECT version FROM schema_version")) {
                return rows.next() ? rows.getInt(1) : 0;
            }
        }
    }

    private synchronized void migrate() throws SQLException {
        int version = schemaVersion();
        for (int next = version; next < MIGRATIONS.length; next++) {
            final int target = next + 1;
            transaction(() -> {
                try (Statement statement = connection.createStatement()) {
                    for (String sql : MIGRATIONS[target - 1]) {
                        statement.execute(sql);
                    }
                    statement.execute("DELETE FROM schema_version");
                    statement.execute("INSERT INTO schema_version (version) VALUES (" + target + ")");
                }
                return null;
            });
            logger.info("Career database schema is now version " + target);
        }
    }

    interface Work<T> {
        T run() throws SQLException;
    }

    /** runs the work in one transaction: all of it lands, or none. Inside another transaction it joins that one. */
    synchronized <T> T transaction(Work<T> work) throws SQLException {
        boolean auto = connection.getAutoCommit();
        if (!auto) {
            return work.run();
        }
        connection.setAutoCommit(false);
        try {
            T result = work.run();
            connection.commit();
            return result;
        } catch (SQLException | RuntimeException e) {
            connection.rollback();
            throw e;
        } finally {
            connection.setAutoCommit(auto);
        }
    }

    // ---- meta

    /** the secret export files are signed with; made once per server */
    public synchronized String secret() throws SQLException {
        String existing = meta("secret");
        if (existing != null) {
            return existing;
        }
        byte[] bytes = new byte[32];
        new SecureRandom().nextBytes(bytes);
        String secret = Base64.getEncoder().encodeToString(bytes);
        try (PreparedStatement insert = connection.prepareStatement("INSERT INTO meta (key, value) VALUES ('secret', ?)")) {
            insert.setString(1, secret);
            insert.executeUpdate();
        }
        return secret;
    }

    private String meta(String key) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT value FROM meta WHERE key = ?")) {
            select.setString(1, key);
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getString(1) : null;
            }
        }
    }

    // ---- profiles

    static String key(String user) {
        return user.toLowerCase(Locale.ENGLISH);
    }

    public synchronized CareerProfile profile(String user) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT * FROM profile WHERE user = ?")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                if (!rows.next()) {
                    return null;
                }
                CareerProfile profile = new CareerProfile();
                profile.created = rows.getLong("created");
                profile.starter = rows.getString("starter");
                profile.coins = rows.getInt("coins");
                profile.xp = rows.getInt("xp");
                profile.wins = rows.getInt("wins");
                profile.losses = rows.getInt("losses");
                profile.wildcards = new CareerProfile.Wildcards(rows.getInt("wc_common"), rows.getInt("wc_uncommon"),
                        rows.getInt("wc_rare"), rows.getInt("wc_mythic"));
                profile.level = CareerRules.levelFor(profile.xp);
                profile.packTokens = rows.getInt("pack_tokens");
                profile.countAiGames = true;
                profile.collection = !profile.starter.isEmpty();
                profile.showProgress = rows.getInt("show_progress") == 1;
                profile.cards = cardTotal(user);
                return profile;
            }
        }
    }

    /** levels by account name for the names that have a profile (B279) */
    public synchronized Map<String, Integer> levels(java.util.Collection<String> users) throws SQLException {
        Map<String, Integer> levels = new java.util.LinkedHashMap<>();
        try (PreparedStatement select = connection.prepareStatement("SELECT xp FROM profile WHERE user = ?")) {
            for (String user : users) {
                select.setString(1, key(user));
                try (ResultSet rows = select.executeQuery()) {
                    if (rows.next()) {
                        levels.put(user, CareerRules.levelFor(rows.getInt(1)));
                    }
                }
            }
        }
        return levels;
    }

    /** creates the profile; false when the account already has one */
    synchronized boolean createProfile(String user, String starter, int coins, long now) throws SQLException {
        try (PreparedStatement insert = connection.prepareStatement(
                "INSERT OR IGNORE INTO profile (user, created, starter, coins) VALUES (?, ?, ?, ?)")) {
            insert.setString(1, key(user));
            insert.setLong(2, now);
            insert.setString(3, starter);
            insert.setInt(4, coins);
            return insert.executeUpdate() == 1;
        }
    }

    /** opens the collection of a profile that has none yet: its starter and the coins it starts with */
    synchronized boolean openCollection(String user, String starter, int coins) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement(
                "UPDATE profile SET starter = ?, coins = coins + ? WHERE user = ? AND starter = ''")) {
            update.setString(1, starter);
            update.setInt(2, coins);
            update.setString(3, key(user));
            return update.executeUpdate() == 1;
        }
    }

    synchronized void deleteProfile(String user) throws SQLException {
        for (String table : USER_TABLES) {
            try (PreparedStatement delete = connection.prepareStatement("DELETE FROM " + table + " WHERE user = ?")) {
                delete.setString(1, key(user));
                delete.executeUpdate();
            }
        }
    }

    synchronized void addCoins(String user, int coins) throws SQLException {
        update("UPDATE profile SET coins = coins + ? WHERE user = ?", coins, user);
    }

    synchronized void addWildcard(String user, String rarity, int amount) throws SQLException {
        update("UPDATE profile SET " + wildcardColumn(rarity) + " = " + wildcardColumn(rarity) + " + ? WHERE user = ?", amount, user);
    }

    static String wildcardColumn(String rarity) {
        switch (rarity) {
            case CareerRules.COMMON:
                return "wc_common";
            case CareerRules.UNCOMMON:
                return "wc_uncommon";
            case CareerRules.RARE:
                return "wc_rare";
            case CareerRules.MYTHIC:
                return "wc_mythic";
            default:
                throw new IllegalArgumentException("No wildcard for rarity " + rarity);
        }
    }

    private void update(String sql, int amount, String user) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement(sql)) {
            update.setInt(1, amount);
            update.setString(2, key(user));
            if (update.executeUpdate() != 1) {
                throw new SQLException("No career for " + user);
            }
        }
    }

    /** sets the progress fields of an imported Career */
    synchronized void restoreProgress(String user, CareerProfile profile) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement(
                "UPDATE profile SET coins = ?, xp = ?, wins = ?, losses = ?, wc_common = ?, wc_uncommon = ?, wc_rare = ?, wc_mythic = ? WHERE user = ?")) {
            update.setInt(1, profile.coins);
            update.setInt(2, profile.xp);
            update.setInt(3, profile.wins);
            update.setInt(4, profile.losses);
            CareerProfile.Wildcards wildcards = profile.wildcards == null ? new CareerProfile.Wildcards(0, 0, 0, 0) : profile.wildcards;
            update.setInt(5, wildcards.common);
            update.setInt(6, wildcards.uncommon);
            update.setInt(7, wildcards.rare);
            update.setInt(8, wildcards.mythic);
            update.setString(9, key(user));
            update.executeUpdate();
        }
    }

    /** puts back a payout record of an imported Career, without paying it again */
    synchronized void restorePayout(String user, CareerPayout payout) throws SQLException {
        try (PreparedStatement insert = connection.prepareStatement(
                "INSERT OR IGNORE INTO payouts (match_key, user, at, opponent, won, coins, xp, note) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")) {
            insert.setString(1, payout.matchKey);
            insert.setString(2, key(user));
            insert.setLong(3, payout.at);
            insert.setString(4, payout.opponent);
            insert.setInt(5, payout.won ? 1 : 0);
            insert.setInt(6, payout.coins);
            insert.setInt(7, payout.xp);
            insert.setString(8, payout.note);
            insert.executeUpdate();
        }
    }

    // ---- cards

    /** copies of a card owned in any printing */
    public synchronized int owned(String user, String name) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT COALESCE(SUM(count), 0) FROM cards WHERE user = ? AND name = ?")) {
            select.setString(1, key(user));
            select.setString(2, name);
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getInt(1) : 0;
            }
        }
    }

    synchronized void addCard(String user, CareerCard card, int count) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement(
                "INSERT INTO cards (user, set_code, card_number, name, rarity, count) VALUES (?, ?, ?, ?, ?, ?)"
                        + " ON CONFLICT (user, set_code, card_number) DO UPDATE SET count = count + excluded.count")) {
            upsert.setString(1, key(user));
            upsert.setString(2, card.setCode);
            upsert.setString(3, card.cardNumber);
            upsert.setString(4, card.name);
            upsert.setString(5, card.rarity);
            upsert.setInt(6, count);
            upsert.executeUpdate();
        }
    }

    public synchronized List<CareerCard> collection(String user) throws SQLException {
        List<CareerCard> cards = new ArrayList<>();
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT set_code, card_number, name, rarity, count FROM cards WHERE user = ? ORDER BY name, set_code, card_number")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    CareerCard card = new CareerCard(rows.getString(3), rows.getString(1), rows.getString(2), rows.getString(4));
                    card.count = rows.getInt(5);
                    cards.add(card);
                }
            }
        }
        return cards;
    }

    private int cardTotal(String user) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT COALESCE(SUM(count), 0) FROM cards WHERE user = ?")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getInt(1) : 0;
            }
        }
    }

    // ---- payouts

    /**
     * Records a match's payout and applies it, once: a second call for the same match changes nothing.
     *
     * @return true when this call paid
     */
    synchronized boolean payout(String matchKey, String user, String opponent, boolean won, int coins, int xp, String note, long now) throws SQLException {
        return transaction(() -> payoutIn(matchKey, user, opponent, won, coins, xp, note, now));
    }

    /** {@link #payout} inside the caller's transaction */
    synchronized boolean payoutIn(String matchKey, String user, String opponent, boolean won, int coins, int xp, String note, long now) throws SQLException {
        try (PreparedStatement insert = connection.prepareStatement(
                "INSERT OR IGNORE INTO payouts (match_key, user, at, opponent, won, coins, xp, note) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")) {
            insert.setString(1, matchKey);
            insert.setString(2, key(user));
            insert.setLong(3, now);
            insert.setString(4, opponent);
            insert.setInt(5, won ? 1 : 0);
            insert.setInt(6, coins);
            insert.setInt(7, xp);
            insert.setString(8, note);
            if (insert.executeUpdate() != 1) {
                return false;
            }
        }
        try (PreparedStatement update = connection.prepareStatement(
                "UPDATE profile SET coins = coins + ?, xp = xp + ?, wins = wins + ?, losses = losses + ? WHERE user = ?")) {
            update.setInt(1, coins);
            update.setInt(2, xp);
            update.setInt(3, won ? 1 : 0);
            update.setInt(4, won ? 0 : 1);
            update.setString(5, key(user));
            if (update.executeUpdate() != 1) {
                throw new SQLException("No career for " + user);
            }
        }
        return true;
    }

    /** the latest payouts, newest first */
    public synchronized List<CareerPayout> payouts(String user, int limit) throws SQLException {
        List<CareerPayout> payouts = new ArrayList<>();
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT match_key, at, opponent, won, coins, xp, note FROM payouts WHERE user = ? ORDER BY at DESC LIMIT ?")) {
            select.setString(1, key(user));
            select.setInt(2, limit);
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    CareerPayout payout = new CareerPayout();
                    payout.matchKey = rows.getString(1);
                    payout.at = rows.getLong(2);
                    payout.opponent = rows.getString(3);
                    payout.won = rows.getInt(4) == 1;
                    payout.coins = rows.getInt(5);
                    payout.xp = rows.getInt(6);
                    payout.note = rows.getString(7);
                    payouts.add(payout);
                }
            }
        }
        return payouts;
    }

    /** wins against each opponent, for unlocking the next tier */
    public synchronized java.util.Map<String, Integer> winsByOpponent(String user) throws SQLException {
        java.util.Map<String, Integer> wins = new java.util.HashMap<>();
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT opponent, SUM(won) FROM payouts WHERE user = ? GROUP BY opponent")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    wins.put(rows.getString(1), rows.getInt(2));
                }
            }
        }
        return wins;
    }

    @Override
    public synchronized void close() throws SQLException {
        connection.close();
    }

    // ---- M8 progress: stats, quests, achievements, unlocks, the weekly goal

    synchronized Map<String, Integer> stats(String user) throws SQLException {
        Map<String, Integer> stats = new java.util.TreeMap<>();
        try (PreparedStatement select = connection.prepareStatement("SELECT counter, value FROM stats WHERE user = ?")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    stats.put(rows.getString(1), rows.getInt(2));
                }
            }
        }
        return stats;
    }

    synchronized void addStat(String user, String counter, int amount) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement(
                "INSERT INTO stats (user, counter, value) VALUES (?, ?, ?)"
                        + " ON CONFLICT (user, counter) DO UPDATE SET value = value + excluded.value")) {
            upsert.setString(1, key(user));
            upsert.setString(2, counter);
            upsert.setInt(3, amount);
            upsert.executeUpdate();
        }
    }

    synchronized void maxStat(String user, String counter, int value) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement(
                "INSERT INTO stats (user, counter, value) VALUES (?, ?, ?)"
                        + " ON CONFLICT (user, counter) DO UPDATE SET value = MAX(value, excluded.value)")) {
            upsert.setString(1, key(user));
            upsert.setString(2, counter);
            upsert.setInt(3, value);
            upsert.executeUpdate();
        }
    }

    /** a quest waiting in a slot */
    static class QuestRow {
        int slot;
        String quest;
        int progress;
        String assigned;
    }

    synchronized List<QuestRow> quests(String user) throws SQLException {
        List<QuestRow> quests = new ArrayList<>();
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT slot, quest, progress, assigned FROM quests WHERE user = ? ORDER BY slot")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    QuestRow row = new QuestRow();
                    row.slot = rows.getInt(1);
                    row.quest = rows.getString(2);
                    row.progress = rows.getInt(3);
                    row.assigned = rows.getString(4);
                    quests.add(row);
                }
            }
        }
        return quests;
    }

    synchronized void putQuest(String user, int slot, String quest, int progress, String day) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement(
                "INSERT OR REPLACE INTO quests (user, slot, quest, progress, assigned) VALUES (?, ?, ?, ?, ?)")) {
            upsert.setString(1, key(user));
            upsert.setInt(2, slot);
            upsert.setString(3, quest);
            upsert.setInt(4, progress);
            upsert.setString(5, day);
            upsert.executeUpdate();
        }
    }

    synchronized void deleteQuest(String user, int slot) throws SQLException {
        try (PreparedStatement delete = connection.prepareStatement("DELETE FROM quests WHERE user = ? AND slot = ?")) {
            delete.setString(1, key(user));
            delete.setInt(2, slot);
            delete.executeUpdate();
        }
    }

    /** a text column of the profile (quest_day, reroll_day) */
    synchronized String profileText(String user, String column) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT " + profileColumn(column) + " FROM profile WHERE user = ?")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getString(1) : null;
            }
        }
    }

    synchronized void setProfileText(String user, String column, String value) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement("UPDATE profile SET " + profileColumn(column) + " = ? WHERE user = ?")) {
            update.setString(1, value);
            update.setString(2, key(user));
            update.executeUpdate();
        }
    }

    synchronized int profileInt(String user, String column) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT " + profileColumn(column) + " FROM profile WHERE user = ?")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getInt(1) : 0;
            }
        }
    }

    synchronized void setProfileInt(String user, String column, int value) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement("UPDATE profile SET " + profileColumn(column) + " = ? WHERE user = ?")) {
            update.setInt(1, value);
            update.setString(2, key(user));
            update.executeUpdate();
        }
    }

    synchronized void addProfileInt(String user, String column, int amount) throws SQLException {
        update("UPDATE profile SET " + profileColumn(column) + " = " + profileColumn(column) + " + ? WHERE user = ?", amount, user);
    }

    /** only these columns are read or written by name */
    private static String profileColumn(String column) {
        switch (column) {
            case "quest_day":
            case "reroll_day":
            case "pack_tokens":
            case "level_paid":
            case "count_ai_games":
            case "show_progress":
            case "xp":
            case "coins":
                return column;
            default:
                throw new IllegalArgumentException("Not a profile column: " + column);
        }
    }

    synchronized Map<String, Long> achievements(String user) throws SQLException {
        Map<String, Long> achieved = new java.util.LinkedHashMap<>();
        try (PreparedStatement select = connection.prepareStatement("SELECT id, at FROM achievements WHERE user = ? ORDER BY at")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    achieved.put(rows.getString(1), rows.getLong(2));
                }
            }
        }
        return achieved;
    }

    /** @return true when this is new */
    synchronized boolean addAchievement(String user, String id, long now) throws SQLException {
        try (PreparedStatement insert = connection.prepareStatement("INSERT OR IGNORE INTO achievements (user, id, at) VALUES (?, ?, ?)")) {
            insert.setString(1, key(user));
            insert.setString(2, id);
            insert.setLong(3, now);
            return insert.executeUpdate() == 1;
        }
    }

    synchronized List<String[]> unlocks(String user) throws SQLException {
        List<String[]> unlocks = new ArrayList<>();
        try (PreparedStatement select = connection.prepareStatement("SELECT kind, item FROM unlocks WHERE user = ? ORDER BY at, kind, item")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    unlocks.add(new String[]{rows.getString(1), rows.getString(2)});
                }
            }
        }
        return unlocks;
    }

    /** The account's cosmetic unlocks as {kind, item}, for checking the look it plays with. */
    public List<String[]> unlocksOf(String user) throws SQLException {
        return unlocks(user);
    }

    /** @return true when this is new */
    synchronized boolean addUnlock(String user, String kind, String item, long now) throws SQLException {
        try (PreparedStatement insert = connection.prepareStatement("INSERT OR IGNORE INTO unlocks (user, kind, item, at) VALUES (?, ?, ?, ?)")) {
            insert.setString(1, key(user));
            insert.setString(2, kind);
            insert.setString(3, item);
            insert.setLong(4, now);
            return insert.executeUpdate() == 1;
        }
    }

    synchronized int weeklyWins(String user, String week) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT wins FROM weekly WHERE user = ? AND week = ?")) {
            select.setString(1, key(user));
            select.setString(2, week);
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getInt(1) : 0;
            }
        }
    }

    synchronized void addWeeklyWin(String user, String week) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement(
                "INSERT INTO weekly (user, week, wins) VALUES (?, ?, 1) ON CONFLICT (user, week) DO UPDATE SET wins = wins + 1")) {
            upsert.setString(1, key(user));
            upsert.setString(2, week);
            upsert.executeUpdate();
        }
    }

    /**
     * Claims a game for progress, once.
     *
     * @return true when this call claimed it
     */
    synchronized boolean claimGame(String gameKey, String user, long now) throws SQLException {
        try (PreparedStatement insert = connection.prepareStatement("INSERT OR IGNORE INTO progress_games (game_key, user, at) VALUES (?, ?, ?)")) {
            insert.setString(1, gameKey);
            insert.setString(2, key(user));
            insert.setLong(3, now);
            return insert.executeUpdate() == 1;
        }
    }

    synchronized void setGameDetails(String gameKey, String details) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement("UPDATE progress_games SET details = ? WHERE game_key = ?")) {
            update.setString(1, details);
            update.setString(2, gameKey);
            update.executeUpdate();
        }
    }

    /** what a game paid, as stored JSON; null when the game isn't the account's or paid nothing */
    synchronized String gameDetails(String gameKey, String user) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT details FROM progress_games WHERE game_key = ? AND user = ?")) {
            select.setString(1, gameKey);
            select.setString(2, key(user));
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getString(1) : null;
            }
        }
    }

    /** a game's stored details for the account: by its exact key, or by the game id it was claimed under */
    synchronized String gameDetailsFor(String gameKey, String user) throws SQLException {
        String exact = gameDetails(gameKey, user);
        return exact != null ? exact : gameDetails(gameKey + "/" + key(user), user);
    }

    /** what a paid game was, for the admin's list: its kind, result, pay and why it paid what it did */
    synchronized void setGameRecord(String gameKey, String kind, boolean won, int coins, int xp, String reason) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement(
                "UPDATE progress_games SET kind = ?, won = ?, coins = ?, xp = ?, reason = ? WHERE game_key = ?")) {
            update.setString(1, kind);
            update.setInt(2, won ? 1 : 0);
            update.setInt(3, coins);
            update.setInt(4, xp);
            update.setString(5, reason);
            update.setString(6, gameKey);
            update.executeUpdate();
        }
    }

    /** One game progression counted, for /admin. */
    public static class PaidGame {
        public String gameKey;
        public long at;
        public String kind;
        public boolean won;
        public int coins;
        public int xp;
        public String reason;
        public boolean revoked;
    }

    /** the account's latest counted games, newest first */
    public synchronized List<PaidGame> paidGames(String user, int limit) throws SQLException {
        List<PaidGame> games = new ArrayList<>();
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT game_key, at, kind, won, coins, xp, reason, revoked FROM progress_games WHERE user = ? ORDER BY at DESC LIMIT ?")) {
            select.setString(1, key(user));
            select.setInt(2, limit);
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    PaidGame game = new PaidGame();
                    game.gameKey = rows.getString(1);
                    game.at = rows.getLong(2);
                    game.kind = rows.getString(3) == null ? CareerRewards.CAREER : rows.getString(3);
                    game.won = rows.getInt(4) == 1;
                    game.coins = rows.getInt(5);
                    game.xp = rows.getInt(6);
                    game.reason = rows.getString(7);
                    game.revoked = rows.getInt(8) == 1;
                    games.add(game);
                }
            }
        }
        return games;
    }

    /** a counted game, or null when it isn't the account's */
    synchronized PaidGame paidGame(String user, String gameKey) throws SQLException {
        for (PaidGame game : paidGames(user, Integer.MAX_VALUE)) {
            if (game.gameKey.equals(gameKey)) {
                return game;
            }
        }
        return null;
    }

    /** @return true when this call revoked it */
    synchronized boolean revokeGame(String gameKey) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement("UPDATE progress_games SET revoked = 1 WHERE game_key = ? AND revoked = 0")) {
            update.setString(1, gameKey);
            return update.executeUpdate() == 1;
        }
    }

    // ---- M24: the pair cap, the day's games, Career tables, the Career deck

    /** paid games a pair of people played together on a day */
    synchronized int pairGames(String day, String pair) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT games FROM pair_games WHERE day = ? AND pair = ?")) {
            select.setString(1, day);
            select.setString(2, pair);
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getInt(1) : 0;
            }
        }
    }

    synchronized void addPairGame(String day, String pair) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement(
                "INSERT INTO pair_games (day, pair, games) VALUES (?, ?, 1) ON CONFLICT (day, pair) DO UPDATE SET games = games + 1")) {
            upsert.setString(1, day);
            upsert.setString(2, pair);
            upsert.executeUpdate();
        }
    }

    /** pairs with the account in them that reached the cap, and on how many days, over the last days given */
    public synchronized Map<String, Integer> cappedPairs(String user, String sinceDay, int cap) throws SQLException {
        Map<String, Integer> pairs = new java.util.TreeMap<>();
        String me = key(user);
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT pair, COUNT(*) FROM pair_games WHERE day >= ? AND games >= ? AND (pair LIKE ? OR pair LIKE ?) GROUP BY pair")) {
            select.setString(1, sinceDay);
            select.setInt(2, cap);
            select.setString(3, me + "|%");
            select.setString(4, "%|" + me);
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    pairs.put(rows.getString(1), rows.getInt(2));
                }
            }
        }
        return pairs;
    }

    /** {wins, games, games against people} on a day */
    synchronized int[] daily(String user, String day) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT wins, games, person_games FROM daily WHERE user = ? AND day = ?")) {
            select.setString(1, key(user));
            select.setString(2, day);
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? new int[]{rows.getInt(1), rows.getInt(2), rows.getInt(3)} : new int[]{0, 0, 0};
            }
        }
    }

    synchronized void addDaily(String user, String day, boolean won, boolean person) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement(
                "INSERT INTO daily (user, day, wins, games, person_games) VALUES (?, ?, ?, 1, ?)"
                        + " ON CONFLICT (user, day) DO UPDATE SET wins = wins + excluded.wins, games = games + 1,"
                        + " person_games = person_games + excluded.person_games")) {
            upsert.setString(1, key(user));
            upsert.setString(2, day);
            upsert.setInt(3, won ? 1 : 0);
            upsert.setInt(4, person ? 1 : 0);
            upsert.executeUpdate();
        }
    }

    /** a Career table, so it survives a restart */
    synchronized void putCareerTable(String tableId, String data, long now) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement(
                "INSERT OR REPLACE INTO career_tables (table_id, data, created) VALUES (?, ?, ?)")) {
            upsert.setString(1, tableId);
            upsert.setString(2, data);
            upsert.setLong(3, now);
            upsert.executeUpdate();
        }
    }

    synchronized void deleteCareerTable(String tableId) throws SQLException {
        try (PreparedStatement delete = connection.prepareStatement("DELETE FROM career_tables WHERE table_id = ?")) {
            delete.setString(1, tableId);
            delete.executeUpdate();
        }
    }

    /** Career tables kept from before a restart, as {table id, data}; ones older than the cutoff are dropped */
    synchronized List<String[]> careerTables(long olderThan) throws SQLException {
        try (PreparedStatement delete = connection.prepareStatement("DELETE FROM career_tables WHERE created < ?")) {
            delete.setLong(1, olderThan);
            delete.executeUpdate();
        }
        List<String[]> tables = new ArrayList<>();
        try (PreparedStatement select = connection.prepareStatement("SELECT table_id, data FROM career_tables")) {
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    tables.add(new String[]{rows.getString(1), rows.getString(2)});
                }
            }
        }
        return tables;
    }

    /** the account's Career deck as JSON, or null */
    public synchronized String deck(String user) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT deck FROM decks WHERE user = ?")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getString(1) : null;
            }
        }
    }

    synchronized void putDeck(String user, String deck, long now) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement("INSERT OR REPLACE INTO decks (user, deck, updated) VALUES (?, ?, ?)")) {
            upsert.setString(1, key(user));
            upsert.setString(2, deck);
            upsert.setLong(3, now);
            upsert.executeUpdate();
        }
    }

    /** every account with a profile, for counting how many players have an achievement */
    synchronized int profileCount() throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT COUNT(*) FROM profile")) {
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getInt(1) : 0;
            }
        }
    }

    /** how many accounts have each achievement */
    synchronized Map<String, Integer> achievementCounts() throws SQLException {
        Map<String, Integer> counts = new java.util.HashMap<>();
        try (PreparedStatement select = connection.prepareStatement("SELECT id, COUNT(*) FROM achievements GROUP BY id")) {
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    counts.put(rows.getString(1), rows.getInt(2));
                }
            }
        }
        return counts;
    }

    /** the account's latest game with stored details */
    synchronized String latestGameDetails(String user) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT details FROM progress_games WHERE user = ? AND details IS NOT NULL ORDER BY at DESC LIMIT 1")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getString(1) : null;
            }
        }
    }

    // ---- M9: campaigns, set unlocks, runs, puzzles, challenges

    /** finished campaign nodes: node id to the option chosen (empty for a duel) */
    synchronized Map<String, String> campaignNodes(String user, String campaign) throws SQLException {
        Map<String, String> done = new java.util.LinkedHashMap<>();
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT node, choice FROM campaign_nodes WHERE user = ? AND campaign = ? ORDER BY done_at")) {
            select.setString(1, key(user));
            select.setString(2, campaign);
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    done.put(rows.getString(1), rows.getString(2) == null ? "" : rows.getString(2));
                }
            }
        }
        return done;
    }

    /** @return true when the node wasn't finished before */
    synchronized boolean finishCampaignNode(String user, String campaign, String node, String choice, long now) throws SQLException {
        try (PreparedStatement insert = connection.prepareStatement(
                "INSERT OR IGNORE INTO campaign_nodes (user, campaign, node, done_at, choice) VALUES (?, ?, ?, ?, ?)")) {
            insert.setString(1, key(user));
            insert.setString(2, campaign);
            insert.setString(3, node);
            insert.setLong(4, now);
            insert.setString(5, choice);
            return insert.executeUpdate() == 1;
        }
    }

    synchronized java.util.Set<String> setUnlocks(String user) throws SQLException {
        java.util.Set<String> sets = new java.util.LinkedHashSet<>();
        try (PreparedStatement select = connection.prepareStatement("SELECT set_code FROM set_unlocks WHERE user = ? ORDER BY at")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    sets.add(rows.getString(1));
                }
            }
        }
        return sets;
    }

    synchronized boolean addSetUnlock(String user, String setCode, long now) throws SQLException {
        try (PreparedStatement insert = connection.prepareStatement("INSERT OR IGNORE INTO set_unlocks (user, set_code, at) VALUES (?, ?, ?)")) {
            insert.setString(1, key(user));
            insert.setString(2, setCode);
            insert.setLong(3, now);
            return insert.executeUpdate() == 1;
        }
    }

    /** a gauntlet or limited run; data is the mode's own JSON */
    static class RunRow {
        String id;
        String kind;
        long started;
        Long ended;
        String state;
        int wins;
        int losses;
        String data;
    }

    synchronized void putRun(String user, RunRow run) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement(
                "INSERT OR REPLACE INTO runs (id, user, kind, started, ended, state, wins, losses, data) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")) {
            upsert.setString(1, run.id);
            upsert.setString(2, key(user));
            upsert.setString(3, run.kind);
            upsert.setLong(4, run.started);
            if (run.ended == null) {
                upsert.setNull(5, java.sql.Types.INTEGER);
            } else {
                upsert.setLong(5, run.ended);
            }
            upsert.setString(6, run.state);
            upsert.setInt(7, run.wins);
            upsert.setInt(8, run.losses);
            upsert.setString(9, run.data);
            upsert.executeUpdate();
        }
    }

    /** the account's runs of a kind, newest first; only active ones when asked */
    synchronized List<RunRow> runs(String user, String kind, boolean activeOnly, int limit) throws SQLException {
        List<RunRow> runs = new ArrayList<>();
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT id, kind, started, ended, state, wins, losses, data FROM runs WHERE user = ? AND kind = ?"
                        + (activeOnly ? " AND state = 'active'" : "") + " ORDER BY started DESC LIMIT ?")) {
            select.setString(1, key(user));
            select.setString(2, kind);
            select.setInt(3, limit);
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    RunRow run = new RunRow();
                    run.id = rows.getString(1);
                    run.kind = rows.getString(2);
                    run.started = rows.getLong(3);
                    long ended = rows.getLong(4);
                    run.ended = rows.wasNull() ? null : ended;
                    run.state = rows.getString(5);
                    run.wins = rows.getInt(6);
                    run.losses = rows.getInt(7);
                    run.data = rows.getString(8);
                    runs.add(run);
                }
            }
        }
        return runs;
    }

    synchronized RunRow run(String user, String id) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT id, kind, started, ended, state, wins, losses, data FROM runs WHERE user = ? AND id = ?")) {
            select.setString(1, key(user));
            select.setString(2, id);
            try (ResultSet rows = select.executeQuery()) {
                if (!rows.next()) {
                    return null;
                }
                RunRow run = new RunRow();
                run.id = rows.getString(1);
                run.kind = rows.getString(2);
                run.started = rows.getLong(3);
                long ended = rows.getLong(4);
                run.ended = rows.wasNull() ? null : ended;
                run.state = rows.getString(5);
                run.wins = rows.getInt(6);
                run.losses = rows.getInt(7);
                run.data = rows.getString(8);
                return run;
            }
        }
    }

    /** {attempts, stars, solvedAt or 0} per puzzle */
    synchronized Map<String, long[]> puzzleResults(String user) throws SQLException {
        Map<String, long[]> results = new java.util.HashMap<>();
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT puzzle, attempts, stars, solved_at FROM puzzle_results WHERE user = ?")) {
            select.setString(1, key(user));
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    results.put(rows.getString(1), new long[]{rows.getInt(2), rows.getInt(3), rows.getLong(4)});
                }
            }
        }
        return results;
    }

    synchronized void puzzleAttempt(String user, String puzzle) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement(
                "INSERT INTO puzzle_results (user, puzzle, attempts) VALUES (?, ?, 1)"
                        + " ON CONFLICT (user, puzzle) DO UPDATE SET attempts = attempts + 1")) {
            upsert.setString(1, key(user));
            upsert.setString(2, puzzle);
            upsert.executeUpdate();
        }
    }

    /** keeps the best stars; the first solve's time stays */
    synchronized void puzzleSolved(String user, String puzzle, int stars, long now) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement(
                "UPDATE puzzle_results SET stars = MAX(stars, ?), solved_at = COALESCE(solved_at, ?) WHERE user = ? AND puzzle = ?")) {
            update.setInt(1, stars);
            update.setLong(2, now);
            update.setString(3, key(user));
            update.setString(4, puzzle);
            update.executeUpdate();
        }
    }

    synchronized void challengeAttempt(String user, String week, long now) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement(
                "INSERT INTO challenge_results (user, week, attempts, at) VALUES (?, ?, 1, ?)"
                        + " ON CONFLICT (user, week) DO UPDATE SET attempts = attempts + 1")) {
            upsert.setString(1, key(user));
            upsert.setString(2, week);
            upsert.setLong(3, now);
            upsert.executeUpdate();
        }
    }

    /** records a win; the best is the fewest turns, then the most life left */
    synchronized void challengeWon(String user, String week, int turns, int life, long now) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement(
                "UPDATE challenge_results SET won = 1, at = ?,"
                        + " best_life = CASE WHEN best_turns IS NULL OR ? < best_turns OR (? = best_turns AND ? > best_life) THEN ? ELSE best_life END,"
                        + " best_turns = CASE WHEN best_turns IS NULL OR ? < best_turns OR (? = best_turns AND ? > best_life) THEN ? ELSE best_turns END"
                        + " WHERE user = ? AND week = ?")) {
            update.setLong(1, now);
            update.setInt(2, turns);
            update.setInt(3, turns);
            update.setInt(4, life);
            update.setInt(5, life);
            update.setInt(6, turns);
            update.setInt(7, turns);
            update.setInt(8, life);
            update.setInt(9, turns);
            update.setString(10, key(user));
            update.setString(11, week);
            update.executeUpdate();
        }
    }

    /** every weekly challenge result: user, week, attempts, won, best turns, best life, at (moving them to the leaderboards) */
    public synchronized List<Object[]> challengeRows() throws SQLException {
        List<Object[]> rows = new ArrayList<>();
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT user, week, attempts, won, best_turns, best_life, at FROM challenge_results")) {
            try (ResultSet result = select.executeQuery()) {
                while (result.next()) {
                    int turns = result.getInt(5);
                    Integer bestTurns = result.wasNull() ? null : turns;
                    int life = result.getInt(6);
                    Integer bestLife = result.wasNull() ? null : life;
                    rows.add(new Object[]{result.getString(1), result.getString(2), result.getInt(3), result.getInt(4) == 1,
                            bestTurns, bestLife, result.getLong(7)});
                }
            }
        }
        return rows;
    }

    /** a week's results, best first: winners by fewest turns then most life, then everyone else by attempts */
    synchronized List<Object[]> challengeBoard(String week, int limit) throws SQLException {
        List<Object[]> board = new ArrayList<>();
        try (PreparedStatement select = connection.prepareStatement(
                "SELECT user, attempts, won, best_turns, best_life FROM challenge_results WHERE week = ?"
                        + " ORDER BY won DESC, best_turns ASC, best_life DESC, attempts ASC LIMIT ?")) {
            select.setString(1, week);
            select.setInt(2, limit);
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    int turns = rows.getInt(4);
                    Integer bestTurns = rows.wasNull() ? null : turns;
                    int life = rows.getInt(5);
                    Integer bestLife = rows.wasNull() ? null : life;
                    board.add(new Object[]{rows.getString(1), rows.getInt(2), rows.getInt(3) == 1, bestTurns, bestLife});
                }
            }
        }
        return board;
    }
}
