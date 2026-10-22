package mage.server.career;

import com.google.gson.JsonParser;
import mage.game.result.ResultProtos;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.Statement;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicReference;
import java.util.stream.Collectors;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * The wardrobe and the chase (M27, M28): the cosmetics catalog, tiered achievements with points and the hall, mastery
 * cosmetics, badges, moments, days played and credit for games played before progression counted them.
 */
public class CareerWardrobeTest {

    private Path file;
    private CareerStore store;
    private final CareerService service = CareerService.get();
    private final AtomicReference<LocalDate> day = new AtomicReference<>(LocalDate.of(2027, 5, 3));

    @BeforeEach
    public void setUp() throws Exception {
        file = Files.createTempFile("career-wardrobe-test", ".db");
        store = new CareerStore("jdbc:sqlite:" + file);
        CareerStore.use(store);
        CareerProgress.get().useClock(day::get, new Random(7));
    }

    @AfterEach
    public void tearDown() throws Exception {
        store.close();
        CareerStore.use(null);
        CareerProgress.get().useClock(() -> LocalDate.now(java.time.ZoneOffset.UTC), new Random());
        Files.deleteIfExists(file);
    }

    private static CareerService.FinishedGame duel(String winner, CareerTally winnerTally, String loser) {
        CareerService.FinishedGame game = new CareerService.FinishedGame(UUID.randomUUID(), CareerRewards.PERSON, 2, 9);
        game.seats.add(new CareerService.Seat(winner, true, winnerTally));
        game.seats.add(new CareerService.Seat(loser, false, CareerTally.forTest(UUID.randomUUID())));
        return game;
    }

    private static List<CareerProgress.CareerCosmetic> contentCosmetics() throws Exception {
        List<CareerProgress.CareerCosmetic> found = new ArrayList<>();
        for (String name : Arrays.asList("levels.json", "achievements.json")) {
            found.addAll(CareerCosmetics.cosmeticsIn(JsonParser.parseReader(new InputStreamReader(
                    CareerWardrobeTest.class.getResourceAsStream("/career/" + name), StandardCharsets.UTF_8))));
        }
        for (String campaign : CareerCampaigns.get().campaigns().keySet()) {
            found.addAll(CareerCosmetics.cosmeticsIn(JsonParser.parseReader(new InputStreamReader(
                    CareerWardrobeTest.class.getResourceAsStream("/career/campaigns/" + campaign + ".json"), StandardCharsets.UTF_8))));
        }
        return found;
    }

    @Test
    public void theCatalogHoldsEveryCosmeticAndSaysWhereEachComesFrom() throws Exception {
        CareerCosmetics catalog = CareerCosmetics.get();
        for (CareerProgress.CareerCosmetic cosmetic : contentCosmetics()) {
            assertNotNull(catalog.find(cosmetic.kind, cosmetic.id), cosmetic.kind + ":" + cosmetic.id + " is in the catalog");
        }
        Set<String> sleeveColors = new HashSet<>();
        Set<String> rarities = new HashSet<>(Arrays.asList(CareerRules.COMMON, CareerRules.UNCOMMON, CareerRules.RARE, CareerRules.MYTHIC));
        for (CareerCosmetics.CareerCatalogItem item : catalog.items()) {
            String key = item.kind + ":" + item.id;
            assertTrue(CareerCosmetics.KINDS.contains(item.kind), key);
            assertTrue(item.id.matches("[a-z0-9-]+"), key + " passes the look's id check");
            assertTrue(rarities.contains(item.rarity), key);
            assertNotNull(item.name, key);
            assertNotNull(item.es, key);
            assertFalse(item.sources.isEmpty(), key + " comes from somewhere");
            if ("sleeve".equals(item.kind)) {
                assertTrue(sleeveColors.add(item.color), key + " has its own dye");
            }
            if ("playmat".equals(item.kind)) {
                assertEquals(7, item.shades.size(), key);
            }
            if ("emote".equals(item.kind) && !item.free) {
                assertEquals(4, item.stickers.size(), key + " comes as a set of four");
            }
        }
        assertTrue(catalog.items().size() >= 240, "about 250 cosmetics");
        assertTrue(catalog.items().stream().filter(item -> "title".equals(item.kind)).count() >= 80, "at least 80 titles");
        assertEquals(15, catalog.items().stream().filter(item -> "frame".equals(item.kind)).count());
        assertEquals(10, catalog.items().stream().filter(item -> "pet".equals(item.kind)).count());
        for (String track : CareerProgress.masteryTracks()) {
            assertNotNull(catalog.masteryReward(track, 5), track + " pays a cosmetic at rank 5");
            assertNotNull(catalog.masteryReward(track, 10), track + " pays its master title at rank 10");
        }
        assertTrue(catalog.isFree("emote", "phrases"));
        assertEquals("critters", catalog.emoteSetOf("owl-sleepy").id);
    }

    @Test
    public void aboutTwoHundredFiftyAchievementsInEightFamilies() {
        Map<String, CareerProgress.AchievementTemplate> all = CareerProgress.get().achievementTemplates();
        int count = 0;
        Set<String> families = new HashSet<>();
        for (CareerProgress.AchievementTemplate template : all.values()) {
            count += template.tiers == null ? 1 : template.tiers.size();
            families.add(template.family);
            if (template.tiers != null) {
                assertEquals(4, template.tiers.size(), template.id + " has bronze, silver, gold and mythic");
                for (int tier = 1; tier < 4; tier++) {
                    assertTrue(template.tiers.get(tier).target > template.tiers.get(tier - 1).target, template.id + " climbs");
                }
                assertTrue(template.text.contains("{n}"), template.id);
            }
        }
        assertTrue(count >= 250, "achievements counted by tier: " + count);
        assertEquals(new HashSet<>(Arrays.asList("play", "social", "formats", "colors", "mechanics", "moments", "career", "long")), families);
    }

    @Test
    public void eachTierPaysOnceWithItsPointsAndTheHallCountsHolders() throws Exception {
        CareerTally tally = CareerTally.forTest(UUID.randomUUID());
        tally.add(CareerTally.SPELLS, 1000);
        CareerProgress.CareerGameResult result = service.gameEnded(duel("Ana", tally, "Bea")).get("Ana");
        List<Integer> tiers = result.achievements.stream().filter(a -> "t-spells".equals(a.id)).map(a -> a.tier).collect(Collectors.toList());
        assertEquals(Arrays.asList(1, 2), tiers, "bronze and silver in one go");
        CareerProgress.CareerAchievement silver = result.achievements.stream().filter(a -> "t-spells".equals(a.id) && a.tier == 2).findFirst().get();
        assertEquals("Cast 1,000 spells.", silver.text);

        // the next game doesn't pay them again
        CareerTally again = CareerTally.forTest(UUID.randomUUID());
        again.add(CareerTally.SPELLS, 1);
        CareerProgress.CareerGameResult second = service.gameEnded(duel("Ana", again, "Cid")).get("Ana");
        assertTrue(second.achievements.stream().noneMatch(a -> "t-spells".equals(a.id)));

        CareerTally other = CareerTally.forTest(UUID.randomUUID());
        other.add(CareerTally.SPELLS, 100);
        service.gameEnded(duel("Bea", other, "Dee"));

        CareerProgress.CareerHall hall = CareerProgress.get().hall("Ana");
        CareerProgress.CareerAchievement spells = hall.achievements.stream().filter(a -> "t-spells".equals(a.id)).findFirst().get();
        assertEquals(2, spells.tier);
        assertEquals(5000, spells.target, "the next tier's target");
        assertEquals(2, spells.tiers.get(0).holders, "Ana and Bea have bronze");
        assertEquals(1, spells.tiers.get(1).holders);
        assertEquals("Cast 100 spells.", spells.tiers.get(0).text, "each tier says what it asked");
        assertEquals(15, spells.points);
        assertTrue(hall.points >= 15);
        assertEquals(hall.points, CareerProgress.get().points("Ana"));
        assertFalse(hall.recent.isEmpty());
        assertEquals(CareerProgress.POINT_RANKS.size(), hall.rankNames.size());
        assertTrue(CareerProgress.get().badges("Ana").containsAll(Arrays.asList("t-spells-1", "t-spells-2")));
        assertEquals(hall.points, CareerProgress.get().publicProfile("Ana").points);
    }

    @Test
    public void masteryRanksPayTheCatalogsCosmetics() throws Exception {
        service.gameEnded(duel("Ana", CareerTally.forTest(UUID.randomUUID()), "Bea"));
        store.maxStat("Ana", "games_color_W", 40);
        CareerProgress.CareerGameResult result = new CareerProgress.CareerGameResult();
        CareerProgress.get().masteriesIn(store, "Ana", result, 1L);
        CareerProgress.CareerCosmetic five = CareerCosmetics.get().masteryReward("color_W", 5);
        assertTrue(result.masteries.stream().anyMatch(rank -> rank.rank == 5 && rank.cosmetic != null && rank.cosmetic.id.equals(five.id)));
        assertTrue(store.unlocksOf("Ana").stream().anyMatch(unlock -> unlock[0].equals(five.kind) && unlock[1].equals(five.id)));
        assertTrue(CareerProgress.get().badges("Ana").contains("mastery-color-w-5"));
    }

    @Test
    public void momentsAreToldWithTheTurnTheyHappenedOn() throws Exception {
        CareerTally tally = CareerTally.forTest(UUID.randomUUID());
        tally.moment("big_hit", 4, 3);
        tally.moment("big_hit", 12, 7);
        tally.moment("big_hit", 11, 8);
        tally.moment("storm", 2, 5);
        CareerProgress.CareerGameResult result = service.gameEnded(duel("Ana", tally, "Bea")).get("Ana");
        assertEquals(1, result.moments.size(), "the small storm isn't worth telling");
        CareerProgress.CareerMoment hit = result.moments.get(0);
        assertEquals("big_hit", hit.id);
        assertEquals(12, hit.value);
        assertEquals(7, hit.turn);
        assertEquals("Hit for 12 in one blow", hit.text);
    }

    @Test
    public void daysPlayedCountOnceADayNeverAsAStreak() throws Exception {
        service.gameEnded(duel("Ana", CareerTally.forTest(UUID.randomUUID()), "Bea"));
        service.gameEnded(duel("Ana", CareerTally.forTest(UUID.randomUUID()), "Cid"));
        assertEquals(1, (int) store.stats("Ana").get("days_played"));
        day.set(day.get().plusDays(5));
        service.gameEnded(duel("Ana", CareerTally.forTest(UUID.randomUUID()), "Dee"));
        assertEquals(2, (int) store.stats("Ana").get("days_played"));
    }

    private static ResultProtos.TableProto match(String controller, long end, int winsNeeded, Object... players) {
        ResultProtos.MatchProto.Builder match = ResultProtos.MatchProto.newBuilder()
                .setDeckType("Constructed - Modern").setGameType("Two Player Duel")
                .setMatchOptions(ResultProtos.MatchOptionsProto.newBuilder().setWinsNeeded(winsNeeded));
        for (int index = 0; index < players.length; index += 3) {
            match.addPlayers(ResultProtos.MatchPlayerProto.newBuilder().setName((String) players[index])
                    .setWins((Integer) players[index + 1]).setHuman((Boolean) players[index + 2]));
        }
        return ResultProtos.TableProto.newBuilder().setControllerName(controller).setEndTimeMs(end).setMatch(match).build();
    }

    @Test
    public void pastGamesAreCreditedOnceFromTheTableRecords() throws Exception {
        long monday = java.time.LocalDate.of(2026, 3, 2).atStartOfDay(java.time.ZoneOffset.UTC).toInstant().toEpochMilli();
        List<ResultProtos.TableProto> tables = Arrays.asList(
                match("Ana", monday, 2, "Ana", 2, true, "Bea", 1, true),
                match("Ana", monday + 86_400_000L, 1, "Ana", 1, true, "Computer - mad", 0, false),
                match("System", monday + 2 * 86_400_000L, 2, "Ana", 0, true, "Bea", 2, true));
        assertEquals(2, CareerHistoryCredit.credit(store, tables));
        Map<String, Integer> ana = store.stats("Ana");
        assertEquals(3 + 1 + 2, (int) ana.get("games"));
        assertEquals(2 + 1, (int) ana.get("wins"));
        assertEquals(3, (int) ana.get("games_kind_person"));
        assertEquals(1, (int) ana.get("wins_kind_ai"));
        assertEquals(2, (int) ana.get("games_kind_event"));
        assertEquals(6, (int) ana.get("games_format_modern"));
        assertEquals(1, (int) ana.get("bo3_match_wins"));
        assertEquals(3, (int) ana.get("days_played"));
        assertNotNull(store.metaValue(CareerHistoryCredit.DONE));

        // a second run raises nothing that's already there
        CareerHistoryCredit.credit(store, tables);
        assertEquals(6, (int) store.stats("Ana").get("games"));
        assertTrue(store.stats("Computer - mad").isEmpty(), "the AI gets no credit");
    }

    @Test
    public void titlesWithSpacesAreHyphenatedByTheMigration() throws Exception {
        store.addUnlock("Ana", "title", "questing knight", 1L);
        store.close();
        try (Connection raw = DriverManager.getConnection("jdbc:sqlite:" + file); Statement sql = raw.createStatement()) {
            sql.execute("UPDATE schema_version SET version = 4");
        }
        store = new CareerStore("jdbc:sqlite:" + file);
        CareerStore.use(store);
        assertEquals(5, store.schemaVersion());
        assertTrue(store.unlocksOf("Ana").stream().anyMatch(unlock -> unlock[1].equals("questing-knight")));
        assertTrue(store.unlocksOf("Ana").stream().noneMatch(unlock -> unlock[1].contains(" ")));
    }
}
