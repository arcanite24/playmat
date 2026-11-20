package mage.server.util;

import org.apache.log4j.Logger;

import java.io.File;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;

/**
 * Gives a SQLite file's free pages back to the disk. SQLite keeps the pages of deleted rows inside the file, so pruning
 * alone never makes it smaller. A file with auto_vacuum NONE (every database made before Nov 2026) is converted once,
 * with a full VACUUM, to auto_vacuum INCREMENTAL; from then on {@code PRAGMA incremental_vacuum} trims it cheaply.
 * The VACUUM needs about the compacted size free in the temp directory and up to the file's size again for its
 * journal, so it waits when the disk is short.
 */
public final class SqliteSpace {

    private static final Logger logger = Logger.getLogger(SqliteSpace.class);

    /** auto_vacuum values, as PRAGMA auto_vacuum reports them */
    static final int NONE = 0;
    static final int INCREMENTAL = 2;

    private SqliteSpace() {
    }

    /** What one pass did. */
    public static final class Result {
        public final long bytesBefore;
        public final long bytesAfter;
        public final boolean vacuumed;

        Result(long bytesBefore, long bytesAfter, boolean vacuumed) {
            this.bytesBefore = bytesBefore;
            this.bytesAfter = bytesAfter;
            this.vacuumed = vacuumed;
        }
    }

    /** to be set (before any table) on a new database, so it never needs the full VACUUM */
    public static void preferIncremental(Connection connection) throws SQLException {
        try (Statement statement = connection.createStatement()) {
            statement.execute("PRAGMA auto_vacuum = INCREMENTAL");
        }
    }

    /**
     * Frees the file's unused pages: a full VACUUM the first time (converting it to incremental), an incremental one
     * after. {@code file} is the database's path, for the free-space check; null skips the check (tests, in memory).
     * The caller holds the store's lock, so its own writes wait; the Rust gateway's wait up to their busy timeout.
     */
    public static Result reclaim(Connection connection, File file, String name) throws SQLException {
        long pageSize = pragma(connection, "page_size");
        long before = pragma(connection, "page_count") * pageSize;
        long free = pragma(connection, "freelist_count") * pageSize;
        boolean vacuumed = false;
        try (Statement statement = connection.createStatement()) {
            if (pragma(connection, "auto_vacuum") == INCREMENTAL) {
                if (free > 0) {
                    incremental(connection, free / pageSize);
                }
            } else {
                long room = file == null ? Long.MAX_VALUE : Math.min(usable(file.getAbsoluteFile().getParentFile()),
                        usable(new File(System.getProperty("java.io.tmpdir"))));
                // the copy (the used pages) plus the journal (up to the whole file), with room to spare
                long need = 2 * before + 256L * 1024 * 1024;
                if (room < need) {
                    logger.warn(name + ": " + mb(free) + " MB unused but only " + mb(room) + " MB free on the disk, "
                            + mb(need) + " MB needed to compact it; trying again later");
                    return new Result(before, before, false);
                }
                long started = System.currentTimeMillis();
                statement.execute("PRAGMA auto_vacuum = INCREMENTAL");
                statement.execute("VACUUM");
                vacuumed = true;
                logger.info(name + ": compacted in " + (System.currentTimeMillis() - started) + " ms, "
                        + mb(before) + " MB -> " + mb(pragma(connection, "page_count") * pageSize) + " MB (incremental from now on)");
            }
        }
        return new Result(before, pragma(connection, "page_count") * pageSize, vacuumed);
    }

    /**
     * PRAGMA incremental_vacuum gives back one page per step of its statement, and the JDBC driver steps a statement
     * once per execute: so one execute per page, all in one transaction (one sync, not one per page).
     */
    private static void incremental(Connection connection, long pages) throws SQLException {
        boolean auto = connection.getAutoCommit();
        connection.setAutoCommit(false);
        try {
            // closed (its last step finished) before the commit, which a statement in progress would block
            try (PreparedStatement step = connection.prepareStatement("PRAGMA incremental_vacuum")) {
                for (long page = 0; page < pages; page++) {
                    step.execute();
                }
            }
            connection.commit();
        } catch (SQLException | RuntimeException e) {
            connection.rollback();
            throw e;
        } finally {
            connection.setAutoCommit(auto);
        }
    }

    static long pragma(Connection connection, String name) throws SQLException {
        try (Statement statement = connection.createStatement();
             ResultSet rows = statement.executeQuery("PRAGMA " + name)) {
            return rows.next() ? rows.getLong(1) : 0;
        }
    }

    private static long usable(File directory) {
        return directory == null ? Long.MAX_VALUE : directory.getUsableSpace();
    }

    private static long mb(long bytes) {
        return bytes / (1024 * 1024);
    }
}
