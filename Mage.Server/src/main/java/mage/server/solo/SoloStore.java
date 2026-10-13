package mage.server.solo;

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
 * Single-player data that isn't Career progress: saved solo games, batch AI jobs and leaderboard scores. Kept in
 * db/solo.db (SQLite) with a versioned schema, like Career's store; losing it loses saves and scores, never a
 * collection.
 */
public final class SoloStore implements AutoCloseable {

    private static final Logger logger = Logger.getLogger(SoloStore.class);
    private static final String DEFAULT_URL = "jdbc:sqlite:./db/solo.db";

    /** migrations, oldest first; the schema version is the number of migrations applied */
    private static final String[][] MIGRATIONS = {
            {
                    // S006: a solo game saved at the start of its latest turn. "auto" saves go when the game ends;
                    // "kept" ones stay until resumed or deleted
                    "CREATE TABLE saved_games (id TEXT PRIMARY KEY, user TEXT NOT NULL, created INTEGER NOT NULL,"
                            + " updated INTEGER NOT NULL, kind TEXT NOT NULL, career INTEGER NOT NULL DEFAULT 0,"
                            + " turn INTEGER NOT NULL, title TEXT NOT NULL, opponents TEXT NOT NULL, game_type TEXT NOT NULL,"
                            + " table_info BLOB NOT NULL, snapshot BLOB NOT NULL)",
                    "CREATE INDEX saved_games_by_user ON saved_games (user, updated)",
                    // S004: AI-against-AI jobs
                    "CREATE TABLE jobs (id TEXT PRIMARY KEY, user TEXT NOT NULL, kind TEXT NOT NULL, created INTEGER NOT NULL,"
                            + " started INTEGER, finished INTEGER, status TEXT NOT NULL, spec TEXT NOT NULL,"
                            + " games INTEGER NOT NULL, done INTEGER NOT NULL DEFAULT 0, result TEXT, error TEXT)",
                    "CREATE INDEX jobs_by_user ON jobs (user, created)",
                    "CREATE INDEX jobs_by_status ON jobs (status, created)",
                    // S005: one row per player and board, the best score kept
                    "CREATE TABLE scores (mode TEXT NOT NULL, board TEXT NOT NULL, user TEXT NOT NULL, v1 INTEGER,"
                            + " v2 INTEGER, attempts INTEGER NOT NULL DEFAULT 1, detail TEXT, at INTEGER NOT NULL,"
                            + " PRIMARY KEY (mode, board, user))"
            },
            {
                    // S013, S014: every turn of a solo game, to play it again from any turn (a what-if line), live or
                    // from its replay
                    "CREATE TABLE game_lines (game_id TEXT PRIMARY KEY, user TEXT NOT NULL, created INTEGER NOT NULL,"
                            + " updated INTEGER NOT NULL, table_info BLOB NOT NULL)",
                    "CREATE INDEX game_lines_by_user ON game_lines (user, updated)",
                    "CREATE TABLE turn_snapshots (game_id TEXT NOT NULL, turn INTEGER NOT NULL, active TEXT NOT NULL,"
                            + " at INTEGER NOT NULL, snapshot BLOB NOT NULL, PRIMARY KEY (game_id, turn))"
            }
    };

    private static volatile SoloStore instance;

    private final Connection connection;

    public SoloStore(String jdbcUrl) throws SQLException {
        connection = DriverManager.getConnection(jdbcUrl);
        try (Statement statement = connection.createStatement()) {
            statement.execute("PRAGMA busy_timeout = 5000");
        }
        migrate();
    }

    public static SoloStore get() {
        SoloStore current = instance;
        if (current == null) {
            synchronized (SoloStore.class) {
                current = instance;
                if (current == null) {
                    new File("db").mkdirs();
                    try {
                        current = new SoloStore(DEFAULT_URL);
                    } catch (SQLException | LinkageError e) {
                        logger.error("Solo database can't be opened", e);
                        throw new IllegalStateException("Saved games and scores can't be stored on this server right now");
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    /** for tests: use this store instead of db/solo.db */
    public static void use(SoloStore store) {
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

    // ---- saved games

    public static class SavedRow {
        public String id;
        public String user;
        public long created;
        public long updated;
        public String kind;
        public boolean career;
        public int turn;
        public String title;
        public String opponents;
        public String gameType;
    }

    public synchronized void saveGame(String id, String user, String kind, boolean career, int turn, String title,
                                      String opponents, String gameType, byte[] tableInfo, byte[] snapshot) throws SQLException {
        long now = System.currentTimeMillis();
        try (PreparedStatement statement = connection.prepareStatement(
                "INSERT INTO saved_games (id, user, created, updated, kind, career, turn, title, opponents, game_type, table_info, snapshot)"
                        + " VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                        + " ON CONFLICT (id) DO UPDATE SET updated = excluded.updated, kind = excluded.kind, turn = excluded.turn,"
                        + " title = excluded.title, opponents = excluded.opponents, table_info = excluded.table_info, snapshot = excluded.snapshot")) {
            statement.setString(1, id);
            statement.setString(2, user);
            statement.setLong(3, now);
            statement.setLong(4, now);
            statement.setString(5, kind);
            statement.setInt(6, career ? 1 : 0);
            statement.setInt(7, turn);
            statement.setString(8, title);
            statement.setString(9, opponents);
            statement.setString(10, gameType);
            statement.setBytes(11, tableInfo);
            statement.setBytes(12, snapshot);
            statement.executeUpdate();
        }
    }

    public synchronized void setKind(String id, String kind) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement("UPDATE saved_games SET kind = ? WHERE id = ?")) {
            statement.setString(1, kind);
            statement.setString(2, id);
            statement.executeUpdate();
        }
    }

    public synchronized List<SavedRow> savedGames(String user) throws SQLException {
        List<SavedRow> rows = new ArrayList<>();
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT id, user, created, updated, kind, career, turn, title, opponents, game_type FROM saved_games"
                        + " WHERE user = ? ORDER BY updated DESC")) {
            statement.setString(1, user);
            try (ResultSet result = statement.executeQuery()) {
                while (result.next()) {
                    SavedRow row = new SavedRow();
                    row.id = result.getString(1);
                    row.user = result.getString(2);
                    row.created = result.getLong(3);
                    row.updated = result.getLong(4);
                    row.kind = result.getString(5);
                    row.career = result.getInt(6) != 0;
                    row.turn = result.getInt(7);
                    row.title = result.getString(8);
                    row.opponents = result.getString(9);
                    row.gameType = result.getString(10);
                    rows.add(row);
                }
            }
        }
        return rows;
    }

    /** @return {table info, snapshot}, or null when there's no such save for the user */
    public synchronized byte[][] loadGame(String id, String user) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT table_info, snapshot FROM saved_games WHERE id = ? AND user = ?")) {
            statement.setString(1, id);
            statement.setString(2, user);
            try (ResultSet result = statement.executeQuery()) {
                return result.next() ? new byte[][]{result.getBytes(1), result.getBytes(2)} : null;
            }
        }
    }

    public synchronized boolean deleteGame(String id) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement("DELETE FROM saved_games WHERE id = ?")) {
            statement.setString(1, id);
            return statement.executeUpdate() > 0;
        }
    }

    public synchronized boolean deleteGame(String id, String user) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement("DELETE FROM saved_games WHERE id = ? AND user = ?")) {
            statement.setString(1, id);
            statement.setString(2, user);
            return statement.executeUpdate() > 0;
        }
    }

    /** drops saves not touched since {@code before}, and all but each player's newest {@code keepPerUser} */
    public synchronized int pruneGames(long before, int keepPerUser) throws SQLException {
        int removed;
        try (PreparedStatement statement = connection.prepareStatement("DELETE FROM saved_games WHERE updated < ?")) {
            statement.setLong(1, before);
            removed = statement.executeUpdate();
        }
        try (PreparedStatement statement = connection.prepareStatement(
                "DELETE FROM saved_games WHERE id IN (SELECT id FROM (SELECT id, ROW_NUMBER() OVER"
                        + " (PARTITION BY user ORDER BY updated DESC) AS n FROM saved_games) WHERE n > ?)")) {
            statement.setInt(1, keepPerUser);
            removed += statement.executeUpdate();
        }
        return removed;
    }

    // ---- turns of solo games (what-if lines)

    public static class TurnRow {
        public int turn;
        /** whose turn it is */
        public String active;
        public long at;
    }

    /** a solo game's table, to set it up again from any of its turns */
    public synchronized void putLine(String gameId, String user, byte[] tableInfo) throws SQLException {
        long now = System.currentTimeMillis();
        try (PreparedStatement statement = connection.prepareStatement(
                "INSERT INTO game_lines (game_id, user, created, updated, table_info) VALUES (?, ?, ?, ?, ?)"
                        + " ON CONFLICT (game_id) DO UPDATE SET updated = excluded.updated, table_info = excluded.table_info")) {
            statement.setString(1, gameId);
            statement.setString(2, user);
            statement.setLong(3, now);
            statement.setLong(4, now);
            statement.setBytes(5, tableInfo);
            statement.executeUpdate();
        }
    }

    public synchronized void putTurn(String gameId, int turn, String active, byte[] snapshot) throws SQLException {
        long now = System.currentTimeMillis();
        try (PreparedStatement statement = connection.prepareStatement(
                "INSERT OR REPLACE INTO turn_snapshots (game_id, turn, active, at, snapshot) VALUES (?, ?, ?, ?, ?)")) {
            statement.setString(1, gameId);
            statement.setInt(2, turn);
            statement.setString(3, active == null ? "" : active);
            statement.setLong(4, now);
            statement.setBytes(5, snapshot);
            statement.executeUpdate();
        }
        try (PreparedStatement statement = connection.prepareStatement("UPDATE game_lines SET updated = ? WHERE game_id = ?")) {
            statement.setLong(1, now);
            statement.setString(2, gameId);
            statement.executeUpdate();
        }
    }

    /** the owner of a game's line, or null when there's none */
    public synchronized String lineOwner(String gameId) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement("SELECT user FROM game_lines WHERE game_id = ?")) {
            statement.setString(1, gameId);
            try (ResultSet rows = statement.executeQuery()) {
                return rows.next() ? rows.getString(1) : null;
            }
        }
    }

    public synchronized List<TurnRow> turns(String gameId) throws SQLException {
        List<TurnRow> turns = new ArrayList<>();
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT turn, active, at FROM turn_snapshots WHERE game_id = ? ORDER BY turn")) {
            statement.setString(1, gameId);
            try (ResultSet rows = statement.executeQuery()) {
                while (rows.next()) {
                    TurnRow row = new TurnRow();
                    row.turn = rows.getInt(1);
                    row.active = rows.getString(2);
                    row.at = rows.getLong(3);
                    turns.add(row);
                }
            }
        }
        return turns;
    }

    /** [table info, snapshot] of a game at the start of a turn, for its owner; null when there's no such turn */
    public synchronized byte[][] loadTurn(String gameId, int turn, String user) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT l.table_info, t.snapshot FROM game_lines l JOIN turn_snapshots t ON t.game_id = l.game_id"
                        + " WHERE l.game_id = ? AND l.user = ? AND t.turn = ?")) {
            statement.setString(1, gameId);
            statement.setString(2, user);
            statement.setInt(3, turn);
            try (ResultSet rows = statement.executeQuery()) {
                return rows.next() ? new byte[][]{rows.getBytes(1), rows.getBytes(2)} : null;
            }
        }
    }

    /** lines older than {@code before}, and past a player's newest {@code keepPerUser}, go with their turns */
    public synchronized int pruneLines(long before, int keepPerUser) throws SQLException {
        int removed;
        try (PreparedStatement statement = connection.prepareStatement("DELETE FROM game_lines WHERE updated < ?")) {
            statement.setLong(1, before);
            removed = statement.executeUpdate();
        }
        try (PreparedStatement statement = connection.prepareStatement(
                "DELETE FROM game_lines WHERE game_id IN (SELECT game_id FROM (SELECT game_id, ROW_NUMBER() OVER"
                        + " (PARTITION BY user ORDER BY updated DESC) AS n FROM game_lines) WHERE n > ?)")) {
            statement.setInt(1, keepPerUser);
            removed += statement.executeUpdate();
        }
        try (Statement statement = connection.createStatement()) {
            statement.executeUpdate("DELETE FROM turn_snapshots WHERE game_id NOT IN (SELECT game_id FROM game_lines)");
        }
        return removed;
    }

    // ---- jobs

    public static class JobRow {
        public String id;
        public String user;
        public String kind;
        public long created;
        public Long started;
        public Long finished;
        public String status;
        public String spec;
        public int games;
        public int done;
        public String result;
        public String error;
    }

    public synchronized void insertJob(JobRow job) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "INSERT INTO jobs (id, user, kind, created, status, spec, games, done) VALUES (?, ?, ?, ?, ?, ?, ?, 0)")) {
            statement.setString(1, job.id);
            statement.setString(2, job.user);
            statement.setString(3, job.kind);
            statement.setLong(4, job.created);
            statement.setString(5, job.status);
            statement.setString(6, job.spec);
            statement.setInt(7, job.games);
            statement.executeUpdate();
        }
    }

    public synchronized void updateJob(String id, String status, Long started, Long finished, int done, String result, String error) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "UPDATE jobs SET status = ?, started = COALESCE(?, started), finished = ?, done = ?, result = ?, error = ? WHERE id = ?")) {
            statement.setString(1, status);
            if (started == null) {
                statement.setNull(2, java.sql.Types.INTEGER);
            } else {
                statement.setLong(2, started);
            }
            if (finished == null) {
                statement.setNull(3, java.sql.Types.INTEGER);
            } else {
                statement.setLong(3, finished);
            }
            statement.setInt(4, done);
            statement.setString(5, result);
            statement.setString(6, error);
            statement.setString(7, id);
            statement.executeUpdate();
        }
    }

    public synchronized JobRow job(String id) throws SQLException {
        List<JobRow> rows = jobs("SELECT * FROM jobs WHERE id = ? LIMIT ?", id, 1);
        return rows.isEmpty() ? null : rows.get(0);
    }

    /** a user's jobs, newest first; null user for everyone's */
    public synchronized List<JobRow> jobs(String user, int limit) throws SQLException {
        if (user == null) {
            return jobs("SELECT * FROM jobs ORDER BY created DESC LIMIT ?", null, limit);
        }
        return jobs("SELECT * FROM jobs WHERE user = ? ORDER BY created DESC LIMIT ?", user, limit);
    }

    public synchronized List<JobRow> jobsWithStatus(String status) throws SQLException {
        List<JobRow> rows = new ArrayList<>();
        try (PreparedStatement statement = connection.prepareStatement("SELECT * FROM jobs WHERE status = ? ORDER BY created")) {
            statement.setString(1, status);
            try (ResultSet result = statement.executeQuery()) {
                while (result.next()) {
                    rows.add(job(result));
                }
            }
        }
        return rows;
    }

    /** games played in jobs a user submitted since {@code since} */
    public synchronized int gamesSince(String user, long since) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT COALESCE(SUM(games), 0) FROM jobs WHERE user = ? AND created >= ? AND status <> 'cancelled'")) {
            statement.setString(1, user);
            statement.setLong(2, since);
            try (ResultSet result = statement.executeQuery()) {
                return result.next() ? result.getInt(1) : 0;
            }
        }
    }

    private List<JobRow> jobs(String sql, String parameter, int limit) throws SQLException {
        List<JobRow> rows = new ArrayList<>();
        try (PreparedStatement statement = connection.prepareStatement(sql)) {
            int i = 1;
            if (parameter != null) {
                statement.setString(i++, parameter);
            }
            statement.setInt(i, limit);
            try (ResultSet result = statement.executeQuery()) {
                while (result.next()) {
                    rows.add(job(result));
                }
            }
        }
        return rows;
    }

    private static JobRow job(ResultSet result) throws SQLException {
        JobRow row = new JobRow();
        row.id = result.getString("id");
        row.user = result.getString("user");
        row.kind = result.getString("kind");
        row.created = result.getLong("created");
        long started = result.getLong("started");
        row.started = result.wasNull() ? null : started;
        long finished = result.getLong("finished");
        row.finished = result.wasNull() ? null : finished;
        row.status = result.getString("status");
        row.spec = result.getString("spec");
        row.games = result.getInt("games");
        row.done = result.getInt("done");
        row.result = result.getString("result");
        row.error = result.getString("error");
        return row;
    }

    // ---- scores

    public static class ScoreRow {
        public String mode;
        public String board;
        public String user;
        /** null until the player has a score (attempts only) */
        public Long v1;
        public Long v2;
        public int attempts;
        public String detail;
        public long at;
    }

    public synchronized ScoreRow score(String mode, String board, String user) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT mode, board, user, v1, v2, attempts, detail, at FROM scores WHERE mode = ? AND board = ? AND user = ?")) {
            statement.setString(1, mode);
            statement.setString(2, board);
            statement.setString(3, user);
            try (ResultSet result = statement.executeQuery()) {
                return result.next() ? score(result) : null;
            }
        }
    }

    public synchronized void putScore(ScoreRow row) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement(
                "INSERT INTO scores (mode, board, user, v1, v2, attempts, detail, at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
                        + " ON CONFLICT (mode, board, user) DO UPDATE SET v1 = excluded.v1, v2 = excluded.v2,"
                        + " attempts = excluded.attempts, detail = excluded.detail, at = excluded.at")) {
            statement.setString(1, row.mode);
            statement.setString(2, row.board);
            statement.setString(3, row.user);
            statement.setObject(4, row.v1);
            statement.setObject(5, row.v2);
            statement.setInt(6, row.attempts);
            statement.setString(7, row.detail);
            statement.setLong(8, row.at);
            statement.executeUpdate();
        }
    }

    /** every score on a board; the caller sorts by the board's rule */
    public synchronized List<ScoreRow> scores(String mode, String board) throws SQLException {
        List<ScoreRow> rows = new ArrayList<>();
        try (PreparedStatement statement = connection.prepareStatement(
                "SELECT mode, board, user, v1, v2, attempts, detail, at FROM scores WHERE mode = ? AND board = ?")) {
            statement.setString(1, mode);
            statement.setString(2, board);
            try (ResultSet result = statement.executeQuery()) {
                while (result.next()) {
                    rows.add(score(result));
                }
            }
        }
        return rows;
    }

    public synchronized int countScores(String mode) throws SQLException {
        try (PreparedStatement statement = connection.prepareStatement("SELECT COUNT(*) FROM scores WHERE mode = ?")) {
            statement.setString(1, mode);
            try (ResultSet result = statement.executeQuery()) {
                return result.next() ? result.getInt(1) : 0;
            }
        }
    }

    /** removes every row an account owns (when the account is deleted) */
    public synchronized void deleteUser(String user) throws SQLException {
        for (String table : new String[]{"saved_games", "jobs", "scores", "game_lines"}) {
            try (PreparedStatement statement = connection.prepareStatement("DELETE FROM " + table + " WHERE user = ?")) {
                // leaderboards key players by their lower-case name
                statement.setString(1, table.equals("scores") ? user.toLowerCase(java.util.Locale.ENGLISH) : user);
                statement.executeUpdate();
            }
        }
        try (Statement statement = connection.createStatement()) {
            statement.executeUpdate("DELETE FROM turn_snapshots WHERE game_id NOT IN (SELECT game_id FROM game_lines)");
        }
    }

    private static ScoreRow score(ResultSet result) throws SQLException {
        ScoreRow row = new ScoreRow();
        row.mode = result.getString(1);
        row.board = result.getString(2);
        row.user = result.getString(3);
        long v1 = result.getLong(4);
        row.v1 = result.wasNull() ? null : v1;
        long v2 = result.getLong(5);
        row.v2 = result.wasNull() ? null : v2;
        row.attempts = result.getInt(6);
        row.detail = result.getString(7);
        row.at = result.getLong(8);
        return row;
    }

    @Override
    public synchronized void close() throws SQLException {
        connection.close();
    }
}
