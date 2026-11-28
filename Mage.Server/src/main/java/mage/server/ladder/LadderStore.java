package mage.server.ladder;

import org.apache.log4j.Logger;

import java.io.File;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.List;

/**
 * db/ladder.db (SQLite), the ladder's ratings (#430): one row per player or bot, format and queue. Kept apart from
 * the human ratings of rated matches (UserStatsRepository) and from Career's store, so the ladder can move to Rust on
 * its own (#186). Losing it resets ratings to where new players and the calibrated bots start.
 */
public final class LadderStore implements AutoCloseable {

    private static final Logger logger = Logger.getLogger(LadderStore.class);
    private static final String DEFAULT_URL = "jdbc:sqlite:./db/ladder.db";

    private static final String[][] MIGRATIONS = {
            {
                    "CREATE TABLE ladder_rating (subject TEXT NOT NULL, format TEXT NOT NULL, queue TEXT NOT NULL,"
                            + " rating REAL NOT NULL, rd REAL NOT NULL, volatility REAL NOT NULL, games INTEGER NOT NULL,"
                            + " updated INTEGER NOT NULL, PRIMARY KEY (subject, format, queue))",
            },
            {
                    // the visible rank (#431): a step per pip, LadderRank; season 0 until seasons (#434)
                    "CREATE TABLE ladder_standing (user TEXT NOT NULL, format TEXT NOT NULL, queue TEXT NOT NULL,"
                            + " season INTEGER NOT NULL, step INTEGER NOT NULL, peak INTEGER NOT NULL, streak INTEGER NOT NULL,"
                            + " won INTEGER NOT NULL, lost INTEGER NOT NULL, drew INTEGER NOT NULL, updated INTEGER NOT NULL,"
                            + " PRIMARY KEY (user, format, queue, season))",
                    // every ranked game: 'playing' from the start, then won, lost, drew or void
                    "CREATE TABLE ladder_games (id TEXT PRIMARY KEY, user TEXT NOT NULL, format TEXT NOT NULL,"
                            + " queue TEXT NOT NULL, season INTEGER NOT NULL, bot TEXT NOT NULL, deck TEXT,"
                            + " status TEXT NOT NULL, started INTEGER NOT NULL, ended INTEGER,"
                            + " step_before INTEGER, step_after INTEGER, rating_before REAL, rating_after REAL, engine TEXT)",
                    "CREATE INDEX ladder_games_by_user ON ladder_games (user, started)",
                    "CREATE INDEX ladder_games_by_status ON ladder_games (status)",
                    // a rank's first-reach reward, paid once per user, format, queue and season
                    "CREATE TABLE ladder_rewards (user TEXT NOT NULL, format TEXT NOT NULL, queue TEXT NOT NULL,"
                            + " season INTEGER NOT NULL, rank TEXT NOT NULL, paid INTEGER NOT NULL,"
                            + " PRIMARY KEY (user, format, queue, season, rank))",
            },
            {
                    // seasons (#434): the Open queue's rank rewards pay once per account, kept under season -1
                    "INSERT OR IGNORE INTO ladder_rewards (user, format, queue, season, rank, paid)"
                            + " SELECT user, format, queue, -1, rank, paid FROM ladder_rewards WHERE queue = 'open'",
            },
            {
                    // season boards (#434) read a format, queue and season at a time
                    "CREATE INDEX ladder_games_by_season ON ladder_games (format, queue, season)",
                    "CREATE INDEX ladder_standing_by_season ON ladder_standing (format, queue, season)",
            },
            {
                    // persisted anti-dodge opponent picks and richer game metadata for rotation and analytics
                    "CREATE TABLE ladder_picks (user TEXT NOT NULL, format TEXT NOT NULL, queue TEXT NOT NULL,"
                            + " bot_id TEXT NOT NULL, picked_at INTEGER NOT NULL, PRIMARY KEY (user, format, queue))",
                    "ALTER TABLE ladder_games ADD COLUMN bot_deck TEXT",
                    "ALTER TABLE ladder_games ADD COLUMN bot_archetype TEXT",
                    "ALTER TABLE ladder_games ADD COLUMN turns INTEGER",
                    "ALTER TABLE ladder_games ADD COLUMN game_score TEXT",
            },
    };

    /** the season key of rewards paid once per account, not per season */
    public static final int LIFETIME = -1;

    private static volatile LadderStore instance;

    private final Connection connection;

    public LadderStore(String jdbcUrl) throws SQLException {
        connection = DriverManager.getConnection(jdbcUrl);
        try (Statement statement = connection.createStatement()) {
            statement.execute("PRAGMA busy_timeout = 5000");
        }
        migrate();
    }

    public static LadderStore get() {
        LadderStore current = instance;
        if (current == null) {
            synchronized (LadderStore.class) {
                current = instance;
                if (current == null) {
                    new File("db").mkdirs();
                    try {
                        current = new LadderStore(DEFAULT_URL);
                    } catch (SQLException | LinkageError e) {
                        logger.error("Ladder database can't be opened", e);
                        throw new IllegalStateException("Ladder ratings can't be stored on this server right now");
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    /** for tests: use this store instead of db/ladder.db */
    public static void use(LadderStore store) {
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
            boolean auto = connection.getAutoCommit();
            connection.setAutoCommit(false);
            try (Statement statement = connection.createStatement()) {
                for (String sql : MIGRATIONS[next]) {
                    statement.execute(sql);
                }
                statement.execute("DELETE FROM schema_version");
                statement.execute("INSERT INTO schema_version (version) VALUES (" + (next + 1) + ")");
                connection.commit();
            } catch (SQLException e) {
                connection.rollback();
                throw e;
            } finally {
                connection.setAutoCommit(auto);
            }
        }
    }

    /** A stored rating and how many rated games are behind it */
    public static final class Row {
        public final Glicko2.Rating rating;
        public final int games;
        public final long updated;

        Row(Glicko2.Rating rating, int games, long updated) {
            this.rating = rating;
            this.games = games;
            this.updated = updated;
        }
    }

    /** the stored rating, or null before the subject's first rated game */
    public synchronized Row rating(String subject, String format, String queue) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT rating, rd, volatility, games, updated FROM ladder_rating WHERE subject = ? AND format = ? AND queue = ?")) {
            statement.setString(1, subject);
            statement.setString(2, format);
            statement.setString(3, queue);
            try (ResultSet rows = statement.executeQuery()) {
                if (!rows.next()) {
                    return null;
                }
                return new Row(new Glicko2.Rating(rows.getDouble(1), rows.getDouble(2), rows.getDouble(3)), rows.getInt(4), rows.getLong(5));
            }
        }
    }

    /** stores both sides of one rated game at once */
    public synchronized void putBoth(String format, String queue, String subjectA, Glicko2.Rating a, int gamesA,
                                     String subjectB, Glicko2.Rating b, int gamesB, long now) throws SQLException {
        boolean auto = connection.getAutoCommit();
        connection.setAutoCommit(false);
        try {
            put(subjectA, format, queue, a, gamesA, now);
            put(subjectB, format, queue, b, gamesB, now);
            connection.commit();
        } catch (SQLException e) {
            connection.rollback();
            throw e;
        } finally {
            connection.setAutoCommit(auto);
        }
    }

    private void put(String subject, String format, String queue, Glicko2.Rating rating, int games, long now) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "INSERT INTO ladder_rating (subject, format, queue, rating, rd, volatility, games, updated) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
                        + " ON CONFLICT (subject, format, queue) DO UPDATE SET rating = excluded.rating, rd = excluded.rd,"
                        + " volatility = excluded.volatility, games = excluded.games, updated = excluded.updated")) {
            statement.setString(1, subject);
            statement.setString(2, format);
            statement.setString(3, queue);
            statement.setDouble(4, rating.rating);
            statement.setDouble(5, rating.rd);
            statement.setDouble(6, rating.volatility);
            statement.setInt(7, games);
            statement.setLong(8, now);
            statement.executeUpdate();
        }
    }

    // ---- standings

    /** A player's standing in one format and queue, for a season */
    public static final class Standing {
        public String user;
        public String format;
        public String queue;
        public int season;
        public int step;
        public int peak;
        public int streak;
        public int won;
        public int lost;
        public int drew;
        public long updated;
    }

    /** the standing, or null before the player's first ranked game there */
    public synchronized Standing standing(String user, String format, String queue, int season) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT step, peak, streak, won, lost, drew, updated FROM ladder_standing"
                        + " WHERE user = ? AND format = ? AND queue = ? AND season = ?")) {
            statement.setString(1, user);
            statement.setString(2, format);
            statement.setString(3, queue);
            statement.setInt(4, season);
            try (ResultSet rows = statement.executeQuery()) {
                if (!rows.next()) {
                    return null;
                }
                Standing s = new Standing();
                s.user = user;
                s.format = format;
                s.queue = queue;
                s.season = season;
                s.step = rows.getInt(1);
                s.peak = rows.getInt(2);
                s.streak = rows.getInt(3);
                s.won = rows.getInt(4);
                s.lost = rows.getInt(5);
                s.drew = rows.getInt(6);
                s.updated = rows.getLong(7);
                return s;
            }
        }
    }

    /** the highest season anyone has a standing or a game in; -1 when none */
    public synchronized int maxSeason() throws SQLException {
        try (Statement statement = connection.createStatement();
             ResultSet rows = statement.executeQuery("SELECT MAX(m) FROM (SELECT MAX(season) AS m FROM ladder_standing"
                     + " UNION ALL SELECT MAX(season) FROM ladder_games)")) {
            if (rows.next()) {
                int max = rows.getInt(1);
                return rows.wasNull() ? -1 : max;
            }
            return -1;
        }
    }

    /** the player's standings in a format and queue, every season, newest first */
    public synchronized java.util.List<Standing> standings(String user, String format, String queue) throws SQLException {
        java.util.List<Standing> out = new java.util.ArrayList<>();
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT season FROM ladder_standing WHERE user = ? AND format = ? AND queue = ? ORDER BY season DESC")) {
            statement.setString(1, user);
            statement.setString(2, format);
            statement.setString(3, queue);
            java.util.List<Integer> seasons = new java.util.ArrayList<>();
            try (ResultSet rows = statement.executeQuery()) {
                while (rows.next()) {
                    seasons.add(rows.getInt(1));
                }
            }
            for (int season : seasons) {
                out.add(standing(user, format, queue, season));
            }
        }
        return out;
    }

    /** the player's latest standing before `season`, or null */
    public synchronized Standing before(String user, String format, String queue, int season) throws SQLException {
        for (Standing s : standings(user, format, queue)) {
            if (s.season < season) {
                return s;
            }
        }
        return null;
    }

    public synchronized void putStanding(Standing s) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "INSERT INTO ladder_standing (user, format, queue, season, step, peak, streak, won, lost, drew, updated)"
                        + " VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT (user, format, queue, season) DO UPDATE SET"
                        + " step = excluded.step, peak = excluded.peak, streak = excluded.streak, won = excluded.won,"
                        + " lost = excluded.lost, drew = excluded.drew, updated = excluded.updated")) {
            statement.setString(1, s.user);
            statement.setString(2, s.format);
            statement.setString(3, s.queue);
            statement.setInt(4, s.season);
            statement.setInt(5, s.step);
            statement.setInt(6, s.peak);
            statement.setInt(7, s.streak);
            statement.setInt(8, s.won);
            statement.setInt(9, s.lost);
            statement.setInt(10, s.drew);
            statement.setLong(11, s.updated);
            statement.executeUpdate();
        }
    }

    // ---- ranked games

    /** A ranked game: who, against which bot, and how it went */
    public static final class GameRow {
        public String id;
        public String user;
        public String format;
        public String queue;
        public int season;
        public String bot;
        public String deck;
        /** playing, won, lost, drew or void */
        public String status;
        public long started;
        public Long ended;
        public Integer stepBefore;
        public Integer stepAfter;
        public Double ratingBefore;
        public Double ratingAfter;
        public String engine;
        public String botDeck;
        public String botArchetype;
        public Integer turns;
        public String gameScore;
    }

    public synchronized void startGame(GameRow g) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "INSERT INTO ladder_games (id, user, format, queue, season, bot, deck, status, started, engine, bot_deck, bot_archetype)"
                        + " VALUES (?, ?, ?, ?, ?, ?, ?, 'playing', ?, ?, ?, ?)")) {
            statement.setString(1, g.id);
            statement.setString(2, g.user);
            statement.setString(3, g.format);
            statement.setString(4, g.queue);
            statement.setInt(5, g.season);
            statement.setString(6, g.bot);
            statement.setString(7, g.deck);
            statement.setLong(8, g.started);
            statement.setString(9, g.engine);
            statement.setString(10, g.botDeck);
            statement.setString(11, g.botArchetype);
            statement.executeUpdate();
        }
    }

    /**
     * Ends a game that is still playing; false when it already ended (a game pays once). A void game keeps no
     * rank or rating change.
     */
    public synchronized boolean endGame(String id, String status, long ended, Integer stepBefore, Integer stepAfter,
                                        Double ratingBefore, Double ratingAfter, String engine) throws SQLException {
        return endGame(id, status, ended, stepBefore, stepAfter, ratingBefore, ratingAfter, engine, null, null);
    }

    public synchronized boolean endGame(String id, String status, long ended, Integer stepBefore, Integer stepAfter,
                                        Double ratingBefore, Double ratingAfter, String engine,
                                        Integer turns, String gameScore) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "UPDATE ladder_games SET status = ?, ended = ?, step_before = ?, step_after = ?, rating_before = ?,"
                        + " rating_after = ?, engine = COALESCE(?, engine), turns = COALESCE(?, turns),"
                        + " game_score = COALESCE(?, game_score) WHERE id = ? AND status = 'playing'")) {
            statement.setString(1, status);
            statement.setLong(2, ended);
            setInt(statement, 3, stepBefore);
            setInt(statement, 4, stepAfter);
            setDouble(statement, 5, ratingBefore);
            setDouble(statement, 6, ratingAfter);
            statement.setString(7, engine);
            setInt(statement, 8, turns);
            statement.setString(9, gameScore);
            statement.setString(10, id);
            return statement.executeUpdate() == 1;
        }
    }

    // ---- persisted opponent picks

    public synchronized String pick(String user, String format, String queue) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT bot_id FROM ladder_picks WHERE user = ? AND format = ? AND queue = ?")) {
            statement.setString(1, user);
            statement.setString(2, format);
            statement.setString(3, queue);
            try (ResultSet rows = statement.executeQuery()) {
                return rows.next() ? rows.getString(1) : null;
            }
        }
    }

    public synchronized void putPick(String user, String format, String queue, String botId, long now) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "INSERT INTO ladder_picks (user, format, queue, bot_id, picked_at) VALUES (?, ?, ?, ?, ?)"
                        + " ON CONFLICT (user, format, queue) DO UPDATE SET bot_id = excluded.bot_id, picked_at = excluded.picked_at")) {
            statement.setString(1, user);
            statement.setString(2, format);
            statement.setString(3, queue);
            statement.setString(4, botId);
            statement.setLong(5, now);
            statement.executeUpdate();
        }
    }

    public synchronized void clearPick(String user, String format, String queue) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "DELETE FROM ladder_picks WHERE user = ? AND format = ? AND queue = ?")) {
            statement.setString(1, user);
            statement.setString(2, format);
            statement.setString(3, queue);
            statement.executeUpdate();
        }
    }

    /** the player's finished (non-void, non-playing) games in a format and queue, newest first */
    public synchronized java.util.List<GameRow> finishedGames(String user, String format, String queue, int limit) throws SQLException {
        java.util.List<GameRow> out = new java.util.ArrayList<>();
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT * FROM ladder_games WHERE user = ? AND format = ? AND queue = ?"
                        + " AND status IN ('won', 'lost', 'drew') ORDER BY started DESC LIMIT ?")) {
            statement.setString(1, user);
            statement.setString(2, format);
            statement.setString(3, queue);
            statement.setInt(4, limit);
            try (ResultSet rows = statement.executeQuery()) {
                while (rows.next()) {
                    out.add(gameRow(rows));
                }
            }
        }
        return out;
    }

    /** all finished games in one season for a player, format and queue, newest first */
    public synchronized java.util.List<GameRow> seasonGames(String user, String format, String queue, int season) throws SQLException {
        java.util.List<GameRow> out = new java.util.ArrayList<>();
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT * FROM ladder_games WHERE user = ? AND format = ? AND queue = ? AND season = ?"
                        + " AND status IN ('won', 'lost', 'drew') ORDER BY started DESC")) {
            statement.setString(1, user);
            statement.setString(2, format);
            statement.setString(3, queue);
            statement.setInt(4, season);
            try (ResultSet rows = statement.executeQuery()) {
                while (rows.next()) {
                    out.add(gameRow(rows));
                }
            }
        }
        return out;
    }

    public synchronized GameRow game(String id) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement("SELECT * FROM ladder_games WHERE id = ?")) {
            statement.setString(1, id);
            try (ResultSet rows = statement.executeQuery()) {
                return rows.next() ? gameRow(rows) : null;
            }
        }
    }

    /** the player's ranked games, newest first */
    public synchronized java.util.List<GameRow> games(String user, String format, String queue, int limit) throws SQLException {
        java.util.List<GameRow> out = new java.util.ArrayList<>();
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT * FROM ladder_games WHERE user = ? AND format = ? AND queue = ? ORDER BY started DESC LIMIT ?")) {
            statement.setString(1, user);
            statement.setString(2, format);
            statement.setString(3, queue);
            statement.setInt(4, limit);
            try (ResultSet rows = statement.executeQuery()) {
                while (rows.next()) {
                    out.add(gameRow(rows));
                }
            }
        }
        return out;
    }

    /** games still marked playing: after a restart, these were interrupted */
    public synchronized java.util.List<GameRow> playing() throws SQLException {
        java.util.List<GameRow> out = new java.util.ArrayList<>();
        try (PreparedStatement statement = connection.prepareStatement("SELECT * FROM ladder_games WHERE status = 'playing'")) {
            try (ResultSet rows = statement.executeQuery()) {
                while (rows.next()) {
                    out.add(gameRow(rows));
                }
            }
        }
        return out;
    }

    /** a player's account is deleted: their standings, games, rewards and ratings go */
    /** everything kept about the account, table by table, for the account's data export */
    public synchronized java.util.Map<String, Object> export(String user) throws SQLException {
        String lower = user.toLowerCase(java.util.Locale.ENGLISH);
        java.util.Map<String, Object> out = new java.util.LinkedHashMap<>();
        out.put("standings", rows("SELECT * FROM ladder_standing WHERE lower(user) = ? ORDER BY format, queue, season", lower));
        out.put("games", rows("SELECT * FROM ladder_games WHERE lower(user) = ? ORDER BY started", lower));
        out.put("rewards", rows("SELECT * FROM ladder_rewards WHERE lower(user) = ? ORDER BY paid", lower));
        out.put("ratings", rows("SELECT * FROM ladder_rating WHERE subject = ? ORDER BY format, queue", LadderRatings.userSubject(user)));
        return out;
    }

    private java.util.List<java.util.Map<String, Object>> rows(String sql, String arg) throws SQLException {
        java.util.List<java.util.Map<String, Object>> out = new java.util.ArrayList<>();
        try (PreparedStatement statement = connection.prepareStatement(sql)) {
            statement.setString(1, arg);
            try (java.sql.ResultSet rs = statement.executeQuery()) {
                java.sql.ResultSetMetaData meta = rs.getMetaData();
                while (rs.next()) {
                    java.util.Map<String, Object> row = new java.util.LinkedHashMap<>();
                    for (int i = 1; i <= meta.getColumnCount(); i++) {
                        row.put(meta.getColumnLabel(i), rs.getObject(i));
                    }
                    out.add(row);
                }
            }
        }
        return out;
    }

    public synchronized void deleteUser(String user) throws SQLException {
        String lower = user.toLowerCase(java.util.Locale.ENGLISH);
        for (String sql : new String[]{
                "DELETE FROM ladder_standing WHERE lower(user) = ?",
                "DELETE FROM ladder_games WHERE lower(user) = ?",
                "DELETE FROM ladder_rewards WHERE lower(user) = ?",
                "DELETE FROM ladder_picks WHERE lower(user) = ?",
        }) {
            try (PreparedStatement statement = connection.prepareStatement(sql)) {
                statement.setString(1, lower);
                statement.executeUpdate();
            }
        }
        try (PreparedStatement statement = connection.prepareStatement("DELETE FROM ladder_rating WHERE subject = ?")) {
            statement.setString(1, LadderRatings.userSubject(user));
            statement.executeUpdate();
        }
    }

    private static GameRow gameRow(ResultSet rows) throws SQLException {
        GameRow g = new GameRow();
        g.id = rows.getString("id");
        g.user = rows.getString("user");
        g.format = rows.getString("format");
        g.queue = rows.getString("queue");
        g.season = rows.getInt("season");
        g.bot = rows.getString("bot");
        g.deck = rows.getString("deck");
        g.status = rows.getString("status");
        g.started = rows.getLong("started");
        long ended = rows.getLong("ended");
        g.ended = rows.wasNull() ? null : ended;
        int before = rows.getInt("step_before");
        g.stepBefore = rows.wasNull() ? null : before;
        int after = rows.getInt("step_after");
        g.stepAfter = rows.wasNull() ? null : after;
        double rb = rows.getDouble("rating_before");
        g.ratingBefore = rows.wasNull() ? null : rb;
        double ra = rows.getDouble("rating_after");
        g.ratingAfter = rows.wasNull() ? null : ra;
        g.engine = rows.getString("engine");
        g.botDeck = rows.getString("bot_deck");
        g.botArchetype = rows.getString("bot_archetype");
        int turns = rows.getInt("turns");
        g.turns = rows.wasNull() ? null : turns;
        g.gameScore = rows.getString("game_score");
        return g;
    }

    private static void setInt(PreparedStatement statement, int index, Integer value) throws SQLException {
        if (value == null) {
            statement.setNull(index, java.sql.Types.INTEGER);
        } else {
            statement.setInt(index, value);
        }
    }

    private static void setDouble(PreparedStatement statement, int index, Double value) throws SQLException {
        if (value == null) {
            statement.setNull(index, java.sql.Types.REAL);
        } else {
            statement.setDouble(index, value);
        }
    }

    // ---- rank rewards

    /** marks a rank's first-reach reward paid; false when it was already */
    public synchronized boolean payReward(String user, String format, String queue, int season, String rank, long now) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "INSERT OR IGNORE INTO ladder_rewards (user, format, queue, season, rank, paid) VALUES (?, ?, ?, ?, ?, ?)")) {
            statement.setString(1, user);
            statement.setString(2, format);
            statement.setString(3, queue);
            statement.setInt(4, season);
            statement.setString(5, rank);
            statement.setLong(6, now);
            return statement.executeUpdate() == 1;
        }
    }

    /** One player's line on a season board (#434) */
    public static final class BoardRow {
        public String user;
        public long value;
        /** games played in the season (peak and Mythic boards) */
        public int games;
        public long at;
    }

    private static final String PLAYED = "status IN ('won', 'lost', 'drew')";

    /** each player's peak rank in a season, players with at least one game in it */
    public synchronized List<BoardRow> peakBoard(String format, String queue, int season) throws SQLException {
        return board("SELECT user, peak, won + lost + drew, updated FROM ladder_standing"
                + " WHERE format = ? AND queue = ? AND season = ? AND won + lost + drew > 0", format, queue, season);
    }

    /** each Mythic player's rating after their last game of a season, rounded */
    public synchronized List<BoardRow> mythicBoard(String format, String queue, int season) throws SQLException {
        List<BoardRow> rows = board("SELECT s.user, (SELECT ROUND(g.rating_after) FROM ladder_games g WHERE g.user = s.user"
                + " AND g.format = s.format AND g.queue = s.queue AND g.season = s.season AND g.rating_after IS NOT NULL AND g." + PLAYED
                + " ORDER BY g.ended DESC LIMIT 1), s.won + s.lost + s.drew, s.updated FROM ladder_standing s"
                + " WHERE s.format = ? AND s.queue = ? AND s.season = ? AND s.peak >= " + LadderRank.MYTHIC_STEP
                + " AND s.won + s.lost + s.drew > 0", format, queue, season);
        rows.removeIf(row -> row.value <= 0);
        return rows;
    }

    /** how many games each player took to reach Mythic in a season, and when */
    public synchronized List<BoardRow> climbBoard(String format, String queue, int season) throws SQLException {
        String sql = "SELECT r.user, (SELECT COUNT(*) FROM ladder_games x WHERE x.user = r.user AND x.format = r.format"
                + " AND x.queue = r.queue AND x.season = r.season AND x." + PLAYED + " AND x.ended <= r.reached), 0, r.reached"
                + " FROM (SELECT user, format, queue, season, MIN(ended) AS reached FROM ladder_games"
                + " WHERE format = ? AND queue = ? AND season = ? AND " + PLAYED + " AND step_after >= " + LadderRank.MYTHIC_STEP
                + " GROUP BY user, format, queue, season) r";
        List<BoardRow> rows = board(sql, format, queue, season);
        for (BoardRow row : rows) {
            row.games = (int) row.value;
        }
        return rows;
    }

    private List<BoardRow> board(String sql, String format, String queue, int season) throws SQLException {
        List<BoardRow> out = new ArrayList<>();
        try (PreparedStatement statement = connection.prepareStatement(sql)) {
            statement.setString(1, format);
            statement.setString(2, queue);
            statement.setInt(3, season);
            try (ResultSet rows = statement.executeQuery()) {
                while (rows.next()) {
                    BoardRow row = new BoardRow();
                    row.user = rows.getString(1);
                    row.value = rows.getLong(2);
                    row.games = rows.getInt(3);
                    row.at = rows.getLong(4);
                    out.add(row);
                }
            }
        }
        return out;
    }

    @Override
    public synchronized void close() throws SQLException {
        connection.close();
    }
}
