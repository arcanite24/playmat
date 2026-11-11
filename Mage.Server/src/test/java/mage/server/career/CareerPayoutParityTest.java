package mage.server.career;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.UUID;
import java.util.stream.Collectors;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * The payouts Rust must make the same (docs/rust/Payouts.md): each scenario is paid by Java on the shadow route, which
 * keeps the event with the rows before and after. With {@code -Dplaymat.payoutFixtures=write} the cases are written to
 * rust/crates/playmat-platform/tests/payouts, where playmat-platform's {@code payouts_match_java} replays each one and
 * must end with the same rows. Rerun it so whenever a payout rule changes in Java.
 */
public class CareerPayoutParityTest {

    private static final Path FIXTURES = Paths.get("..", "rust", "crates", "playmat-platform", "tests", "payouts");
    private static final String SECRET = "parity-secret";

    private Path file;
    private Path soloFile;
    private CareerStore store;
    private mage.server.solo.SoloStore solo;
    private final CareerService service = CareerService.get();
    private final LocalDate day = LocalDate.of(2027, 5, 5); // a Wednesday
    private final List<Map<String, Object>> cases = new ArrayList<>();

    @BeforeEach
    public void setUp() throws Exception {
        file = Files.createTempFile("career-parity", ".db");
        soloFile = Files.createTempFile("solo-parity", ".db");
        store = new CareerStore("jdbc:sqlite:" + file);
        CareerStore.use(store);
        solo = new mage.server.solo.SoloStore("jdbc:sqlite:" + soloFile);
        mage.server.solo.SoloStore.use(solo);
        store.setMeta("secret", SECRET);
        // no fixed draws: the quests come from the seeded dice, as in production
        CareerProgress.get().useClock(() -> day, null);
        CareerPayouts.useRoute(CareerPayouts.SHADOW);
    }

    @AfterEach
    public void tearDown() throws Exception {
        CareerPayouts.useRoute(null);
        CareerProgress.get().useClock(() -> LocalDate.now(java.time.ZoneOffset.UTC), new Random());
        store.close();
        CareerStore.use(null);
        solo.close();
        mage.server.solo.SoloStore.use(null);
        Files.deleteIfExists(file);
        Files.deleteIfExists(soloFile);
    }

    private static CareerTally tally(Object... counters) {
        CareerTally tally = CareerTally.forTest(UUID.randomUUID());
        for (int i = 0; i < counters.length; i += 2) {
            tally.add((String) counters[i], (Integer) counters[i + 1]);
        }
        return tally;
    }

    private void keep(String name, String key) throws Exception {
        String[] row = store.payoutEvent(key);
        assertNotNull(row, name + ": the event is kept");
        assertEquals("paid", row[0], name);
        assertEquals("java", row[1], name);
        assertNotNull(row[4], name + ": the rows before");
        assertNotNull(row[5], name + ": the rows after");
        Map<String, Object> one = new LinkedHashMap<>();
        one.put("name", name);
        one.put("event", JsonParser.parseString(row[3]));
        one.put("before", JsonParser.parseString(row[4]));
        one.put("after", JsonParser.parseString(row[5]));
        cases.add(one);
    }

    private String duel(String name, String opponent, int tier, boolean won, int turns, CareerTally tally, String assisted) throws Exception {
        UUID table = UUID.randomUUID();
        service.register(table, "Ana", opponent, tier);
        if (assisted != null) {
            service.tableInfo(table).assisted = assisted;
        }
        service.matchEnded(table, null, won, turns, tally);
        keep(name, table.toString());
        return table.toString();
    }

    private String game(String name, CareerService.FinishedGame game) throws Exception {
        service.gameEnded(game);
        keep(name, game.gameId.toString());
        return game.gameId.toString();
    }

    private static CareerService.Seat seat(String user, boolean won, CareerTally tally, String... colors) {
        CareerService.Seat seat = new CareerService.Seat(user, won, tally);
        for (String color : colors) {
            seat.colors.add(color);
        }
        return seat;
    }

    @Test
    public void payoutsRustReplays() throws Exception {
        store.createProfile("Ana", "test", CareerRules.START_COINS, 1L);

        // a first win of the day: three quests and the weekly one are picked, the daily ladder and first win pay
        CareerTally first = tally("spells", 9, "damage_to_opponents", 23, "creatures_entered", 6, "biggest_hit", 7);
        first.moment("big_hit", 7, 6);
        String key = duel("first win", "wren", 1, true, 9, first, null);
        CareerProgress.CareerGameResult result = service.gameResult("Ana", key);
        assertTrue(result.quests.size() >= 0 && result.weekly != null, "the weekly goal is told");

        // a loss before turn 3 pays nothing; a loss makes the opponent the nemesis
        duel("early loss", "odo", 1, false, 2, tally(), null);
        duel("nemesis", "pip", 1, false, 8, tally("spells", 4), null);
        // beating the nemesis pays the bounty
        duel("bounty", "pip", 1, true, 10, tally("spells", 12, "attacks", 9), null);
        // help halves the pay
        duel("assisted", "marisol", 1, true, 7, tally("spells", 3), "a hint");
        // a rival chapter's line
        duel("rival", "rival-1", 1, true, 11, tally("cards_drawn", 14), null);
        // a raid sets no nemesis
        duel("raid", "leviathan", 5, false, 12, tally("life_gained", 30), null);

        // near a level with capsule tokens, and the week's fifth win
        store.addProfileInt("Ana", "xp", 1300 - store.profile("Ana").xp - 50);
        for (int i = 0; i < 4 - store.weeklyWins("Ana", CareerProgress.get().week()); i++) {
            store.addWeeklyWin("Ana", CareerProgress.get().week());
        }
        duel("level and weekly goal", "ilse", 2, true, 9, tally("spells", 40, "damage_to_opponents", 60, "tokens_created", 12), null);

        // a game against the AI: XP by skill, a best of three won, a color mastery's first rank
        store.addStat("Ana", "games_color_R", 2);
        CareerService.FinishedGame ai = new CareerService.FinishedGame(UUID.randomUUID(), CareerRewards.AI, 2, 9);
        ai.aiSkill = 6;
        ai.bo3 = true;
        ai.format = "pioneer";
        CareerService.Seat ana = seat("Ana", true, tally("spells", 8, "lands_played", 5), "R");
        ana.matchWon = true;
        ai.seats.add(ana);
        game("ai bo3", ai);

        // two people: the first game against a person today, a seat without a tally gets nothing
        CareerService.FinishedGame people = new CareerService.FinishedGame(UUID.randomUUID(), CareerRewards.PERSON, 2, 8);
        people.format = "standard";
        people.seats.add(seat("Ana", false, tally("blocks", 4), "W", "U"));
        people.seats.add(seat("Bea", true, tally("attacks", 7), "G"));
        game("person", people);
        CareerService.FinishedGame untallied = new CareerService.FinishedGame(UUID.randomUUID(), CareerRewards.PERSON, 2, 8);
        untallied.seats.add(seat("Ana", true, tally("attacks", 3), "B"));
        untallied.seats.add(seat("Cyd", false, null));
        game("person without a tally", untallied);

        // the pair cap: the day's games together are used up
        for (int i = 0; i < CareerRewards.get().pairCapPerDay(); i++) {
            store.addPairGame(day.toString(), CareerService.pair("Ana", "Bea"));
        }
        CareerService.FinishedGame capped = new CareerService.FinishedGame(UUID.randomUUID(), CareerRewards.POD, 4, 10);
        capped.seats.add(seat("Ana", true, tally("spells", 2), "R"));
        capped.seats.add(seat("Bea", false, tally("spells", 5), "G"));
        game("pair cap", capped);

        // a game before the turn floor, and an event
        CareerService.FinishedGame short_ = new CareerService.FinishedGame(UUID.randomUUID(), CareerRewards.AI, 2, 2);
        short_.seats.add(seat("Dee", true, tally("spells", 1), "colorless"));
        game("before the floor", short_);
        CareerService.FinishedGame event = new CareerService.FinishedGame(UUID.randomUUID(), CareerRewards.EVENT, 2, 9);
        event.format = "limited";
        event.seats.add(seat("Ana", true, tally("spells", 6, "games", 0), "U", "R"));
        game("event game", event);

        // a payout made twice pays once: the event is already there
        assertNull(service.matchEnded(UUID.randomUUID(), null, true, 9, tally()), "not a Career table");

        // the rewards screen reads what Java paid
        assertEquals(14, cases.size());
        if ("write".equals(System.getProperty("playmat.payoutFixtures"))) {
            write();
        }
    }

    private void write() throws Exception {
        Gson gson = new GsonBuilder().setPrettyPrinting().disableHtmlEscaping().create();
        Files.createDirectories(FIXTURES);
        Map<String, Object> book = new LinkedHashMap<>();
        book.put("secret", SECRET);
        book.put("schema", store.schema());
        book.put("quests", new ArrayList<>(CareerProgress.get().questTemplates().values()));
        book.put("achievements", CareerProgress.get().achievementExport());
        book.put("opponents", mage.server.platform.CareerViewsExport.opponents(CareerContent.get()));
        // the catalog's mastery rewards (the rest of it doesn't pay)
        book.put("cosmetics", CareerCosmetics.get().items().stream().filter(item -> item.mastery != null).collect(Collectors.toList()));
        JsonObject all = new JsonObject();
        all.add("book", JsonParser.parseString(gson.toJson(book)));
        all.add("cases", gson.toJsonTree(cases));
        String text = gson.toJson((JsonElement) all) + "\n";
        Files.write(FIXTURES.resolve("cases.json"), text.getBytes(StandardCharsets.UTF_8));
    }
}
