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

    @Test
    public void payPendingSkipsBadEventAndPaysRemaining() throws Exception {
        Path file = dir.resolve("career-pending.db");
        try (CareerStore store = new CareerStore("jdbc:sqlite:" + file)) {
            CareerStore.use(store);
            CareerPayouts.useRoute(CareerPayouts.JAVA);
            try {
                long now = System.currentTimeMillis();
                store.createProfile("Ana", "test", CareerRules.START_COINS, now);

                // First pending event is corrupt JSON; second is a valid pending Career match payout
                assertTrue(store.addPayoutEvent("aaa-bad", now - 10, "{not valid json", "pending", null, null, null));

                CareerPayoutEvent.Career table = new CareerPayoutEvent.Career();
                table.kind = "duel";
                table.user = "Ana";
                table.opponent = "wren";
                table.tier = 1;
                CareerPayoutEvent good = new CareerPayoutEvent();
                good.event = CareerPayoutEvent.MATCH_ENDED;
                good.key = "zzz-good";
                good.at = now;
                good.day = CareerProgress.get().day();
                good.career = table;
                good.won = true;
                good.turns = 6;
                assertTrue(store.addPayoutEvent(good.key, now, new com.google.gson.Gson().toJson(good), "pending", null, null, null));

                int paid = CareerPayouts.payPending();
                assertEquals(1, paid);
                assertEquals("paid", store.payoutEvent(good.key)[0]);
                assertEquals("pending", store.payoutEvent("aaa-bad")[0]);
                assertEquals(1, store.profile("Ana").wins);
            } finally {
                CareerPayouts.useRoute(null);
                CareerStore.use(null);
            }
        }
    }
}
