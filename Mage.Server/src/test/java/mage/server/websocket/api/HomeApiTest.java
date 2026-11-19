package mage.server.websocket.api;

import mage.server.career.CareerStore;
import mage.server.websocket.rpc.RpcException;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.util.Arrays;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

/** Home (#432): saved layouts are checked against the widget list; the activity summary counts days and streaks. */
public class HomeApiTest {

    private Path file;
    private CareerStore store;

    @BeforeEach
    void open() throws Exception {
        file = Files.createTempFile("home-test", ".db");
        store = new CareerStore("jdbc:sqlite:" + file);
        CareerStore.use(store);
    }

    @AfterEach
    void close() throws Exception {
        CareerStore.use(null);
        store.close();
        Files.deleteIfExists(file);
    }

    @Test
    void aLayoutIsCheckedAndWrittenBackClean() throws Exception {
        String clean = HomeApi.clean("{\"v\":1,\"preset\":\"casual\",\"extra\":true,\"items\":["
                + "{\"key\":\"rank-0\",\"id\":\"rank\",\"size\":\"m\",\"settings\":{\"format\":\"pioneer\"},\"junk\":1},"
                + "{\"key\":\"weekly-1\",\"id\":\"weekly\",\"size\":\"s\"}]}");
        assertEquals("{\"v\":1,\"preset\":\"casual\",\"items\":[{\"key\":\"rank-0\",\"id\":\"rank\",\"size\":\"m\",\"settings\":{\"format\":\"pioneer\"}},"
                + "{\"key\":\"weekly-1\",\"id\":\"weekly\",\"size\":\"s\"}]}", clean);
        assertTrue(HomeApi.clean("{\"v\":1,\"preset\":\"nope\",\"items\":[]}").contains("\"preset\":null"));
        for (String bad : Arrays.asList(
                "not json", "{\"v\":2,\"items\":[]}", "{\"v\":1}",
                "{\"v\":1,\"items\":[{\"key\":\"a\",\"id\":\"nope\",\"size\":\"s\"}]}",
                "{\"v\":1,\"items\":[{\"key\":\"a\",\"id\":\"weekly\",\"size\":\"l\"}]}",
                "{\"v\":1,\"items\":[{\"key\":\"a\",\"id\":\"weekly\",\"size\":\"s\"},{\"key\":\"a\",\"id\":\"level\",\"size\":\"s\"}]}")) {
            assertThrows(RpcException.class, () -> HomeApi.clean(bad), bad);
        }
        StringBuilder many = new StringBuilder("{\"v\":1,\"items\":[");
        for (int i = 0; i < 25; i++) {
            many.append(i == 0 ? "" : ",").append("{\"key\":\"k").append(i).append("\",\"id\":\"weekly\",\"size\":\"s\"}");
        }
        assertThrows(RpcException.class, () -> HomeApi.clean(many + "]}"), "at most 24");
    }

    @Test
    void activityCountsDaysInThePlayersTimeZone() throws Exception {
        long day = HomeApi.DAY_MS;
        // 2027-01-10 03:00 UTC is still the 9th at UTC-6
        long now = 1_799_000_000_000L - Math.floorMod(1_799_000_000_000L, day) + 3 * 3_600_000L;
        game("a", now - 60_000, true, "Wren");
        game("b", now - day, false, "Oriel");
        game("c", now - 2 * day, true, "Brann");
        game("d", now - 4 * day, true, "Vessa");
        // 23:00 UTC yesterday: still yesterday in UTC, but today at UTC-6
        game("e", now - 4 * 3_600_000L, false, "Kade");
        HomeApi.HomeActivity utc = HomeApi.activity("Neri", 0, now);
        assertEquals(HomeApi.DAYS, utc.days.size());
        assertEquals(Integer.valueOf(1), utc.days.get(HomeApi.DAYS - 1));
        assertEquals(3, utc.streak, "today, yesterday and the day before");
        assertEquals("Wren", utc.games.get(0).opponent);
        assertEquals(Boolean.TRUE, utc.games.get(0).won);
        assertEquals(Integer.valueOf(2), utc.days.get(HomeApi.DAYS - 2));
        assertEquals(5, utc.games.size());
        HomeApi.HomeActivity mexico = HomeApi.activity("neri", -360, now);
        assertEquals(Integer.valueOf(2), mexico.days.get(HomeApi.DAYS - 1));
        assertEquals(3, mexico.streak);
    }

    private void game(String key, long at, boolean won, String opponent) throws Exception {
        try (Connection connection = DriverManager.getConnection("jdbc:sqlite:" + file);
             PreparedStatement insert = connection.prepareStatement("INSERT INTO progress_games (game_key, user, at, details) VALUES (?, ?, ?, ?)")) {
            insert.setString(1, key);
            insert.setString(2, "neri");
            insert.setLong(3, at);
            insert.setString(4, "{\"won\":" + won + ",\"opponent\":\"" + opponent + "\",\"career\":false}");
            insert.executeUpdate();
        }
    }
}
