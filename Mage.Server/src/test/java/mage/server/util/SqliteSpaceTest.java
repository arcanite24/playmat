package mage.server.util;

import mage.server.solo.SoloStore;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.File;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.Statement;

import static org.junit.jupiter.api.Assertions.*;

public class SqliteSpaceTest {

    @TempDir
    Path dir;

    private static void fill(Connection connection, int rows) throws Exception {
        try (PreparedStatement insert = connection.prepareStatement("INSERT INTO blobs (data) VALUES (?)")) {
            for (int i = 0; i < rows; i++) {
                insert.setBytes(1, new byte[64 * 1024]);
                insert.executeUpdate();
            }
        }
    }

    @Test
    public void anOldFileIsCompactedOnceThenTrimmedIncrementally() throws Exception {
        File file = dir.resolve("old.db").toFile();
        try (Connection connection = DriverManager.getConnection("jdbc:sqlite:" + file)) {
            try (Statement statement = connection.createStatement()) {
                statement.execute("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB NOT NULL)");
            }
            assertEquals(SqliteSpace.NONE, SqliteSpace.pragma(connection, "auto_vacuum"));
            fill(connection, 80);
            try (Statement statement = connection.createStatement()) {
                statement.execute("DELETE FROM blobs WHERE id > 10");
            }
            long full = file.length();
            assertTrue(full > 5_000_000, "the deleted rows' pages stay in the file");

            SqliteSpace.Result first = SqliteSpace.reclaim(connection, file, "old.db");
            assertTrue(first.vacuumed);
            assertEquals(SqliteSpace.INCREMENTAL, SqliteSpace.pragma(connection, "auto_vacuum"));
            assertTrue(file.length() < full / 4, "compacted: " + file.length());

            // from now on deleted rows' pages go back with an incremental vacuum, no full VACUUM
            fill(connection, 40);
            long grown = file.length();
            try (Statement statement = connection.createStatement()) {
                statement.execute("DELETE FROM blobs WHERE id > 10");
            }
            SqliteSpace.Result next = SqliteSpace.reclaim(connection, file, "old.db");
            assertFalse(next.vacuumed);
            assertEquals(0, SqliteSpace.pragma(connection, "freelist_count"));
            assertTrue(file.length() < grown / 2, "trimmed: " + file.length());
        }
    }

    @Test
    public void aNewStoreIsIncrementalFromTheStart() throws Exception {
        File file = dir.resolve("solo.db").toFile();
        try (SoloStore store = new SoloStore("jdbc:sqlite:" + file)) {
            assertFalse(store.reclaimSpace().vacuumed);
        }
        try (Connection connection = DriverManager.getConnection("jdbc:sqlite:" + file)) {
            assertEquals(SqliteSpace.INCREMENTAL, SqliteSpace.pragma(connection, "auto_vacuum"));
        }
    }

    @Test
    public void inMemoryUrlsHaveNoFile() {
        assertNull(SoloStore.fileOf("jdbc:sqlite::memory:"));
        assertNull(SoloStore.fileOf("jdbc:sqlite:file::memory:?cache=shared"));
        assertEquals(new File("./db/solo.db"), SoloStore.fileOf("jdbc:sqlite:./db/solo.db"));
    }
}
