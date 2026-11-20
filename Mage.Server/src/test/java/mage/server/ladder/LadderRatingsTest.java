package mage.server.ladder;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.StringReader;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * The ladder's ratings (#430): Glicko-2 against Glickman's worked example, the shipped bot roster, and how a rated
 * game and matchmaking use them.
 */
public class LadderRatingsTest {

    private LadderStore store;

    @BeforeEach
    void open() throws Exception {
        store = new LadderStore("jdbc:sqlite::memory:");
    }

    @AfterEach
    void close() throws Exception {
        store.close();
    }

    @Test
    void glicko2MatchesGlickmansExample() {
        // "Example of the Glicko-2 system", section 3: 1500 ± 200 beats 1400 ± 30, loses to 1550 ± 100 and 1700 ± 300
        Glicko2.Rating player = new Glicko2.Rating(1500, 200, 0.06);
        List<Glicko2.Result> results = Arrays.asList(
                new Glicko2.Result(new Glicko2.Rating(1400, 30, 0.06), 1),
                new Glicko2.Result(new Glicko2.Rating(1550, 100, 0.06), 0),
                new Glicko2.Result(new Glicko2.Rating(1700, 300, 0.06), 0));
        Glicko2.Rating after = Glicko2.update(player, results, 0);
        assertEquals(1464.06, after.rating, 0.01);
        assertEquals(151.52, after.rd, 0.01);
        assertEquals(0.05999, after.volatility, 0.00001);
    }

    @Test
    void anIdlePeriodOnlyWidensTheDeviation() {
        Glicko2.Rating player = new Glicko2.Rating(1500, 50, 0.06);
        Glicko2.Rating after = Glicko2.update(player, java.util.Collections.<Glicko2.Result>emptyList(), 0);
        assertEquals(1500, after.rating, 1e-9);
        assertEquals(Math.sqrt(Math.pow(50 / Glicko2.SCALE, 2) + 0.06 * 0.06) * Glicko2.SCALE, after.rd, 1e-9);
    }

    @Test
    void theShippedRosterIsCalibratedAndComplete() throws Exception {
        BotRoster roster = BotRoster.get();
        assertEquals(new HashSet<>(Arrays.asList("standard", "pioneer", "modern")), roster.formats());
        assertNotNull(roster.calibration(), "bots.json says how it was measured");
        Set<String> cosmetics = cosmeticIds();
        Set<String> names = new HashSet<>();
        for (String format : roster.formats()) {
            List<BotRoster.Bot> bots = roster.bots(format);
            // Modern fields only the school decks the Rust host nearly plays (#435): at least eight, at two or three skills
            int least = "modern".equals(format) ? 20 : 40;
            assertTrue(bots.size() >= least, format + " has at least " + least + " bots, has " + bots.size());
            BotRoster.Bot anchor = roster.bot(roster.anchor(format));
            assertNotNull(anchor, format + "'s anchor is a bot");
            assertEquals(1000, (int) anchor.rating, format + "'s anchor is 1000 by definition");
            for (BotRoster.Bot bot : bots) {
                assertTrue(bot.calibrated(), bot.id + " is calibrated");
                assertTrue(bot.rd != null && bot.rd >= 30 && bot.rd <= 350, bot.id + " has a deviation");
                assertTrue(Arrays.asList(1, 3, 5, 7).contains(bot.skill), bot.id + " plays a Rust host skill");
                // #435: pinned to the search it was rated with, so a stronger AI can't move it
                assertNotNull(bot.ai, bot.id + " is pinned to its generation's search");
                assertTrue(bot.generation >= 1, bot.id + " has a generation");
                assertTrue(bot.generation == 1 || bot.id.endsWith("-g" + bot.generation), bot.id + "'s id says its generation");
                assertNotNull(LadderRatingsTest.class.getResource("/career/campaigns/decks/" + bot.deck), bot.id + "'s deck ships");
                BotRoster.Persona p = bot.persona;
                assertTrue(names.add(p.name), "persona names are unique: " + p.name);
                for (String id : Arrays.asList(p.avatar, p.sleeve, p.playmat, p.frame, p.title)) {
                    assertTrue(cosmetics.contains(id), bot.id + "'s " + id + " is in the cosmetics catalog");
                }
            }
            // the climb has room: the strongest bot is well above the weakest
            double low = bots.get(0).startRating().rating;
            double high = bots.get(bots.size() - 1).startRating().rating;
            assertTrue(high - low >= 300, format + " spans " + low + " to " + high);
        }
    }

    @Test
    void aRatedGameMovesBothSidesTowardTheResult() throws Exception {
        LadderRatings ratings = new LadderRatings(store, roster());
        long now = 1_800_000_000_000L;
        assertNull(store.rating("user:neri", "standard", "open"));
        LadderRatings.Outcome won = ratings.record("Neri", "strong", "open", 1, now);
        assertEquals(LadderRatings.START, won.playerBefore.rating, 1e-9);
        assertTrue(won.playerAfter.rating > won.playerBefore.rating, "beating a stronger bot raises the player");
        assertTrue(won.playerAfter.rd < won.playerBefore.rd, "and makes the rating surer");
        assertTrue(won.botAfter.rating < won.botBefore.rating, "the bot loses rating");
        // a calibrated bot moves far less than an unknown player
        assertTrue(won.botBefore.rating - won.botAfter.rating < won.playerAfter.rating - won.playerBefore.rating);
        assertEquals(1, store.rating("user:neri", "standard", "open").games);
        assertEquals(41, store.rating("bot:strong", "standard", "open").games, "the calibration's games count");

        LadderRatings.Outcome lost = ratings.record("neri", "strong", "open", 0, now);
        assertEquals(won.playerAfter.rating, lost.playerBefore.rating, 1e-9, "the stored rating is the next start");
        assertTrue(lost.playerAfter.rating < lost.playerBefore.rating);
        // another queue is another rating
        assertNull(store.rating("user:neri", "standard", "bo3"));
    }

    @Test
    void anIdlePlayerGrowsLessSure() throws Exception {
        LadderRatings ratings = new LadderRatings(store, roster());
        long now = 1_800_000_000_000L;
        ratings.record("neri", "strong", "open", 1, now);
        Glicko2.Rating today = ratings.player("neri", "standard", "open", now);
        Glicko2.Rating month = ratings.player("neri", "standard", "open", now + 30 * LadderRatings.PERIOD_MS);
        assertEquals(today.rating, month.rating, 1e-9);
        assertTrue(month.rd > today.rd);
    }

    @Test
    void matchmakingDrawsFromAroundThePlayer() throws Exception {
        LadderRatings ratings = new LadderRatings(store, roster());
        List<LadderRatings.Candidate> near = ratings.candidates("standard", "open", 1450);
        // 1350 to 1500 holds two; widened to 1300 to 1550 it holds four; never 1700
        assertEquals(Arrays.asList("low", "mid-low", "mid", "mid-high"), ids(near));
        // far above every bot: the window widens until it holds three
        List<LadderRatings.Candidate> top = ratings.candidates("standard", "open", 2100);
        assertEquals(3, top.size());
        assertEquals("strong", top.get(top.size() - 1).bot.id);
        assertTrue(ratings.candidates("modern", "open", 1500).isEmpty(), "a format without bots has none");
        assertFalse(ids(ratings.candidates("standard", "open", 1000)).isEmpty());
    }

    private static List<String> ids(List<LadderRatings.Candidate> candidates) {
        String[] out = new String[candidates.size()];
        for (int i = 0; i < out.length; i++) {
            out[i] = candidates.get(i).bot.id;
        }
        return Arrays.asList(out);
    }

    private static BotRoster roster() {
        return new BotRoster(new StringReader("{\"anchors\":{\"standard\":\"weak\"},\"bots\":["
                + bot("weak", 1000, 40) + "," + bot("low", 1300, 60) + "," + bot("mid-low", 1400, 60) + ","
                + bot("mid", 1500, 60) + "," + bot("mid-high", 1550, 60) + "," + bot("strong", 1700, 40) + "]}"));
    }

    private static String bot(String id, int rating, int rd) {
        return "{\"id\":\"" + id + "\",\"format\":\"standard\",\"deck\":\"standard/starter.dck\",\"skill\":5,\"prior\":1200,"
                + "\"rating\":" + rating + ",\"rd\":" + rd + ",\"games\":40,\"persona\":{\"name\":\"" + id + "\"}}";
    }

    private static Set<String> cosmeticIds() throws Exception {
        Set<String> ids = new HashSet<>();
        try (InputStream in = LadderRatingsTest.class.getResourceAsStream("/career/cosmetics.json")) {
            JsonArray items = JsonParser.parseReader(new InputStreamReader(in, StandardCharsets.UTF_8)).getAsJsonArray();
            for (JsonElement item : items) {
                JsonObject o = item.getAsJsonObject();
                ids.add(o.get("id").getAsString());
            }
        }
        return ids;
    }
}
