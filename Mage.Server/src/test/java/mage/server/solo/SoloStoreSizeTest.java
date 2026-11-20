package mage.server.solo;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.Statement;
import java.util.Comparator;
import java.util.Random;
import java.util.stream.Stream;

import static org.junit.jupiter.api.Assertions.*;

/**
 * solo.db can't grow without bound: what-if lines stay under a size budget, and the space they free goes back to the
 * disk (2026-11-20: 1.65 GB of turns and 0.93 GB of free pages filled a 75 GB host along with the backups).
 */
public class SoloStoreSizeTest {

    private Path dir;

    @BeforeEach
    public void setUp() throws Exception {
        dir = Files.createTempDirectory("solo-size-test");
    }

    @AfterEach
    public void tearDown() throws Exception {
        try (Stream<Path> paths = Files.walk(dir)) {
            paths.sorted(Comparator.reverseOrder()).forEach(path -> path.toFile().delete());
        }
    }

    private static byte[] turn(Random random) {
        // snapshots are compressed: random bytes are a fair stand-in
        byte[] bytes = new byte[50_000];
        random.nextBytes(bytes);
        return bytes;
    }

    /** 20 games of 10 turns (about 10 MB), the oldest played first */
    private static void play(SoloStore store) throws Exception {
        Random random = new Random(1);
        for (int game = 0; game < 20; game++) {
            store.putLine("g" + game, "user" + (game % 3), new byte[]{1});
            for (int turn = 1; turn <= 10; turn++) {
                store.putTurn("g" + game, turn, "Ana", turn(random));
            }
            Thread.sleep(2);
        }
    }

    @Test
    public void linesStayUnderTheBudgetLeastRecentFirst() throws Exception {
        try (SoloStore store = new SoloStore("jdbc:sqlite:" + dir.resolve("solo.db"))) {
            play(store);
            assertTrue(store.usedBytes() > 9_000_000, "about 10 MB of turns: " + store.usedBytes());

            int removed = store.pruneLinesToSize(4_000_000);
            assertTrue(removed >= 12, "most lines go: " + removed);
            assertTrue(store.usedBytes() <= 4_000_000, "under the budget: " + store.usedBytes());
            assertTrue(store.turns("g0").isEmpty(), "the least recently played line went, with its turns");
            assertEquals(10, store.turns("g19").size(), "the newest line stays whole");
            assertEquals(0, store.pruneLinesToSize(4_000_000), "nothing more to do");
        }
    }

    @Test
    public void aNewFileGivesFreePagesBackBitByBit() throws Exception {
        Path file = dir.resolve("solo.db");
        try (SoloStore store = new SoloStore("jdbc:sqlite:" + file)) {
            assertEquals(2, store.autoVacuum(), "a new file is incremental");
            play(store);
            store.pruneLinesToSize(2_000_000);
            assertTrue(store.freeBytes() > 5_000_000, "the turns' pages are free, still in the file");
            long before = Files.size(file);

            long freed = store.compact(0);
            assertTrue(freed > 5_000_000, "freed " + freed);
            assertEquals(0, store.freeBytes(), "every free page went back");
            assertTrue(Files.size(file) < before - 5_000_000, "the file shrank");
        }
    }

    @Test
    public void anOlderFileIsVacuumedOnceWhenTheDiskHasRoom() throws Exception {
        Path file = dir.resolve("solo.db");
        // a file from before: auto_vacuum none
        try (Connection connection = DriverManager.getConnection("jdbc:sqlite:" + file);
             Statement statement = connection.createStatement()) {
            statement.execute("PRAGMA auto_vacuum = NONE");
            statement.execute("CREATE TABLE schema_version (version INTEGER NOT NULL)");
            statement.execute("INSERT INTO schema_version VALUES (0)");
        }
        try (SoloStore store = new SoloStore("jdbc:sqlite:" + file)) {
            assertEquals(0, store.autoVacuum(), "the pragma alone doesn't change a file that has tables");
            play(store);
            store.pruneLinesToSize(2_000_000);
            long free = store.freeBytes();
            assertTrue(free > 5_000_000);

            assertEquals(0, store.compact(store.fileBytes()), "a VACUUM needs twice the file free on the disk: not now");
            assertEquals(free, store.freeBytes());

            assertTrue(store.compact(Long.MAX_VALUE) > 5_000_000, "with room it runs");
            assertEquals(0, store.freeBytes());
            assertEquals(2, store.autoVacuum(), "and from then on it's incremental");
            assertEquals(3, store.schemaVersion(), "the data survived");
        }
    }
}
