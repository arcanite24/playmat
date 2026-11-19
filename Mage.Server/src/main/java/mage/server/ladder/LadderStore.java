package mage.server.ladder;

import org.apache.log4j.Logger;

import java.io.File;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;

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
    };

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

    @Override
    public synchronized void close() throws SQLException {
        connection.close();
    }
}
