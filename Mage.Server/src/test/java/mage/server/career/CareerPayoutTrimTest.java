package mage.server.career;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.Statement;

import static org.junit.jupiter.api.Assertions.*;

/** checked shadow events lose their row copies; differences keep them (career.db grew 100 MB a day without this) */
public class CareerPayoutTrimTest {

    @TempDir
    Path dir;

    @Test
    public void checkedEventsLoseTheirRowsDifferencesKeepThem() throws Exception {
        Path file = dir.resolve("career.db");
        try (CareerStore store = new CareerStore("jdbc:sqlite:" + file)) {
            long now = System.currentTimeMillis();
            for (String key : new String[]{"same", "skipped", "different", "unchecked"}) {
                assertTrue(store.addPayoutEvent(key, now, "{}", "paid", "java", "{\"before\":1}", "{\"after\":1}"));
            }
            // as Rust's shadow replay marks them
            try (Connection rust = DriverManager.getConnection("jdbc:sqlite:" + file);
                 Statement statement = rust.createStatement()) {
                statement.executeUpdate("UPDATE payout_events SET checked = 'same' WHERE key = 'same'");
                statement.executeUpdate("UPDATE payout_events SET checked = 'skipped' WHERE key = 'skipped'");
                statement.executeUpdate("UPDATE payout_events SET checked = 'different: coins 5 vs 6' WHERE key = 'different'");
            }
            assertEquals(2, store.trimCheckedPayoutEvents());
            assertEquals(0, store.trimCheckedPayoutEvents(), "once");

            for (String key : new String[]{"same", "skipped"}) {
                String[] row = store.payoutEvent(key);
                assertEquals("paid", row[0]);
                assertNotNull(row[2], "still checked");
                assertNull(row[4], key + ": no rows before");
                assertNull(row[5], key + ": no rows after");
            }
            assertNotNull(store.payoutEvent("different")[4], "a difference keeps its rows");
            assertNotNull(store.payoutEvent("unchecked")[4], "Rust hasn't replayed it yet");

            // paid events past their keep go whole
            assertEquals(4, store.prunePayoutEvents(now + 1));
        }
    }
}
