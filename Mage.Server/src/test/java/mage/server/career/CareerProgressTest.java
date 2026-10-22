package mage.server.career;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.DriverManager;
import java.sql.Statement;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicReference;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Career progress contract tests: quests, levels, achievements and the weekly goal are durable state, paid once per
 * game, so each rule is pinned here.
 */
public class CareerProgressTest {

    private Path file;
    private CareerStore store;
    private final CareerService service = CareerService.get();
    private final CareerProgress progress = CareerProgress.get();
    private final AtomicReference<LocalDate> day = new AtomicReference<>(LocalDate.of(2027, 5, 3)); // a Monday

    @BeforeEach
    public void setUp() throws Exception {
        file = Files.createTempFile("career-progress-test", ".db");
        store = new CareerStore("jdbc:sqlite:" + file);
        CareerStore.use(store);
        store.createProfile("Ana", "test", CareerRules.START_COINS, 1L);
        progress.useClock(day::get, new Random(7));
    }

    @AfterEach
    public void tearDown() throws Exception {
        store.close();
        CareerStore.use(null);
        progress.useClock(() -> LocalDate.now(java.time.ZoneOffset.UTC), new Random());
        Files.deleteIfExists(file);
    }

    private static CareerTally tally(Object... counters) {
        CareerTally tally = CareerTally.forTest(UUID.randomUUID());
        for (int i = 0; i < counters.length; i += 2) {
            tally.add((String) counters[i], (Integer) counters[i + 1]);
        }
        return tally;
    }

    private CareerProgress.CareerGameResult careerGame(boolean won, CareerTally tally) {
        UUID table = UUID.randomUUID();
        service.register(table, "Ana", "wren", 1);
        assertNotNull(service.matchEnded(table, null, won, 9, tally));
        try {
            return service.gameResult("Ana", table.toString());
        } catch (Exception e) {
            throw new IllegalStateException(e);
        }
    }

    @Test
    public void theContentIsComplete() {
        assertTrue(progress.questTemplates().size() >= 90, "about ninety quest templates");
        assertTrue(progress.questTemplates().values().stream().filter(quest -> quest.weekly).count() >= 10, "weekly quests");
        assertTrue(progress.achievementTemplates().size() >= 55, "about sixty achievements");
        for (int level = 2; level <= CareerRules.MAX_LEVEL; level++) {
            CareerProgress.CareerLevelReward reward = progress.levelRewards().get(level);
            assertNotNull(reward, "level " + level + " gives something");
            assertTrue(reward.coins > 0 || reward.packs > 0 || reward.wildcard != null || reward.cosmetic != null, "level " + level + " gives something");
        }
        for (CareerProgress.AchievementTemplate achievement : progress.achievementTemplates().values()) {
            assertTrue("game".equals(achievement.scope) || "total".equals(achievement.scope), achievement.id);
            assertTrue(achievement.target > 0 || (achievement.tiers != null && achievement.tiers.get(0).target > 0), achievement.id);
        }
    }

    @Test
    public void aMigratedDatabaseKeepsItsCareers() throws Exception {
        // a database made by the first schema, before quests existed
        Path old = Files.createTempFile("career-v1", ".db");
        try {
            try (java.sql.Connection connection = DriverManager.getConnection("jdbc:sqlite:" + old);
                 Statement statement = connection.createStatement()) {
                statement.execute("CREATE TABLE schema_version (version INTEGER NOT NULL)");
                statement.execute("INSERT INTO schema_version VALUES (1)");
                statement.execute("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)");
                statement.execute("CREATE TABLE profile (user TEXT PRIMARY KEY, created INTEGER NOT NULL, starter TEXT NOT NULL,"
                        + " coins INTEGER NOT NULL DEFAULT 0, xp INTEGER NOT NULL DEFAULT 0, wins INTEGER NOT NULL DEFAULT 0,"
                        + " losses INTEGER NOT NULL DEFAULT 0, wc_common INTEGER NOT NULL DEFAULT 0, wc_uncommon INTEGER NOT NULL DEFAULT 0,"
                        + " wc_rare INTEGER NOT NULL DEFAULT 0, wc_mythic INTEGER NOT NULL DEFAULT 0)");
                statement.execute("CREATE TABLE cards (user TEXT NOT NULL, set_code TEXT NOT NULL, card_number TEXT NOT NULL,"
                        + " name TEXT NOT NULL, rarity TEXT NOT NULL, count INTEGER NOT NULL, PRIMARY KEY (user, set_code, card_number))");
                statement.execute("CREATE TABLE payouts (match_key TEXT PRIMARY KEY, user TEXT NOT NULL, at INTEGER NOT NULL,"
                        + " opponent TEXT NOT NULL, won INTEGER NOT NULL, coins INTEGER NOT NULL, xp INTEGER NOT NULL, note TEXT)");
                statement.execute("INSERT INTO profile (user, created, starter, coins, xp) VALUES ('bo', 1, 'x', 340, 600)");
            }
            try (CareerStore migrated = new CareerStore("jdbc:sqlite:" + old)) {
                assertEquals(6, migrated.schemaVersion());
                CareerProfile profile = migrated.profile("bo");
                assertEquals(340, profile.coins);
                assertEquals(600, profile.xp);
                assertEquals(0, profile.packTokens);
                assertTrue(profile.countAiGames, "every game counts now");
            }
        } finally {
            Files.deleteIfExists(old);
        }
    }

    private long dailyQuests() throws Exception {
        return progress.quests("Ana").quests.stream().filter(quest -> !quest.weekly).count();
    }

    @Test
    public void threeQuestsToStartThenOneADay() throws Exception {
        assertEquals(3, dailyQuests());
        store.deleteQuest("Ana", 0);
        store.deleteQuest("Ana", 1);
        assertEquals(1, dailyQuests(), "the same day brings no new quest");
        day.set(day.get().plusDays(1));
        assertEquals(2, dailyQuests(), "a new day brings one");
        day.set(day.get().plusDays(5));
        assertEquals(3, dailyQuests(), "days away add one, not one per day");
        day.set(day.get().plusDays(1));
        assertEquals(3, dailyQuests(), "never more than three wait");
    }

    @Test
    public void aWeeklyQuestWaitsInTheFourthSlotAndChangesEachWeek() throws Exception {
        CareerProgress.CareerQuest weekly = progress.quests("Ana").quests.stream().filter(quest -> quest.weekly).findFirst().orElse(null);
        assertNotNull(weekly);
        assertEquals(CareerProgress.WEEKLY_SLOT, weekly.slot);
        assertThrows(CareerService.CareerException.class, () -> progress.reroll("Ana", CareerProgress.WEEKLY_SLOT), "not swappable");
        day.set(day.get().plusDays(7));
        assertEquals(1, progress.quests("Ana").quests.stream().filter(quest -> quest.weekly).count(), "one weekly quest at a time");
    }

    @Test
    public void aQuestCanBeSwappedOnceADay() throws Exception {
        CareerProgress.CareerQuests quests = progress.quests("Ana");
        assertTrue(quests.canReroll);
        String before = quests.quests.get(0).id;
        CareerProgress.CareerQuests after = progress.reroll("Ana", quests.quests.get(0).slot);
        assertFalse(after.canReroll);
        assertFalse(after.quests.stream().anyMatch(quest -> quest.slot == 0 && quest.id.equals(before)), "a different quest");
        assertThrows(CareerService.CareerException.class, () -> progress.reroll("Ana", 1));
        day.set(day.get().plusDays(1));
        assertTrue(progress.quests("Ana").canReroll);
    }

    @Test
    public void aQuestCompletesAndPaysOnce() throws Exception {
        progress.quests("Ana");
        store.putQuest("Ana", 0, "cast-red", 15, day.get().toString());
        CareerProfile before = store.profile("Ana");

        CareerProgress.CareerGameResult result = careerGame(false, tally("spells_R", 7, "spells", 7));
        CareerProgress.CareerQuestProgress red = result.quests.stream().filter(quest -> quest.id.equals("cast-red")).findFirst().orElse(null);
        assertNotNull(red);
        assertEquals(15, red.before);
        assertEquals(20, red.after, "progress stops at the target");
        assertTrue(red.completed);
        CareerProgress.QuestTemplate template = progress.questTemplates().get("cast-red");
        CareerProfile after = store.profile("Ana");
        int lossCoins = CareerRules.reward(1, false, 9).coins;
        assertEquals(before.coins + lossCoins + template.coins + achievementCoins(result) + levelCoins(result), after.coins);
        assertFalse(progress.quests("Ana").quests.stream().anyMatch(quest -> quest.id.equals("cast-red")), "a finished quest leaves");
        assertEquals(1, (int) store.stats("Ana").get("quests_completed"));
    }

    private static int achievementCoins(CareerProgress.CareerGameResult result) {
        return result.achievements.stream().mapToInt(achievement -> achievement.coins).sum();
    }

    private static int levelCoins(CareerProgress.CareerGameResult result) {
        return result.levelRewards.stream().mapToInt(reward -> reward.coins).sum();
    }

    @Test
    public void aMatchCountsOnceAndOnlyOnce() throws Exception {
        UUID table = UUID.randomUUID();
        service.register(table, "Ana", "wren", 1);
        CareerTally tally = tally("spells", 12, "damage_to_opponents", 20);
        assertNotNull(service.matchEnded(table, null, true, 9, tally));
        service.register(table, "Ana", "wren", 1);
        assertNull(service.matchEnded(table, null, true, 9, tally), "a second end of the same match pays nothing");
        Map<String, Integer> stats = store.stats("Ana");
        assertEquals(12, (int) stats.get("spells"));
        assertEquals(20, (int) stats.get("damage_to_opponents"));
        assertEquals(1, (int) stats.get("wins"));
        assertEquals(1, (int) stats.get("games"));
    }

    @Test
    public void everyLevelReachedPaysItsRewardOnce() throws Exception {
        CareerProgress.CareerLevelReward two = progress.levelRewards().get(2);
        store.addProfileInt("Ana", "xp", CareerRules.xpForLevel(1) - 10);
        CareerProgress.CareerGameResult result = careerGame(false, tally());
        assertEquals(1, result.levelBefore);
        assertEquals(2, result.levelAfter);
        assertEquals(1, result.levelRewards.size());
        assertEquals(2, result.levelRewards.get(0).level);
        assertEquals(two.coins, result.levelRewards.get(0).coins);
        assertEquals(2, store.profileInt("Ana", "level_paid"));

        CareerProgress.CareerGameResult next = careerGame(false, tally());
        assertTrue(next.levelRewards.isEmpty(), "level 2 isn't paid again");
    }

    @Test
    public void achievementsAreEarnedOnceAndSecretsStayHidden() throws Exception {
        CareerProgress.CareerAchievement oneLife = progress.achievements("Ana").stream()
                .filter(achievement -> achievement.id.equals("one-life")).findFirst().orElse(null);
        assertNotNull(oneLife);
        assertTrue(oneLife.secret);
        assertNull(oneLife.name, "a secret has no name until earned");

        CareerProgress.CareerGameResult first = careerGame(true, tally());
        assertTrue(first.achievements.stream().anyMatch(achievement -> achievement.id.equals("first-win")));
        CareerProgress.CareerGameResult second = careerGame(true, tally());
        assertFalse(second.achievements.stream().anyMatch(achievement -> achievement.id.equals("first-win")), "earned once");

        CareerTally storm = tally("most_spells_in_a_turn", 6);
        CareerProgress.CareerGameResult stormy = careerGame(false, storm);
        assertTrue(stormy.achievements.stream().anyMatch(achievement -> achievement.id.equals("storm")));
    }

    @Test
    public void theWeeklyGoalGivesFreePacksThatTheShopSpendsFirst() throws Exception {
        CareerProgress.CareerGameResult last = null;
        for (int i = 0; i < 5; i++) {
            last = careerGame(true, tally());
        }
        assertEquals(5, last.weekly.wins);
        assertEquals(10, last.weekly.nextGoal);
        assertTrue(last.packs >= 1, "the fifth win gives a pack");
        int tokens = store.profile("Ana").packTokens;
        assertTrue(tokens >= 1);

        int coins = store.profile("Ana").coins;
        List<CareerCard> pack = new ArrayList<>();
        pack.add(new CareerCard("Shock", "TST", "1", CareerRules.COMMON));
        service.open("Ana", "TST", pack);
        assertEquals(coins, store.profile("Ana").coins, "a free pack costs no coins");
        assertEquals(tokens - 1, store.profile("Ana").packTokens);
        assertEquals(1, (int) store.stats("Ana").get("packs_opened"));

        day.set(day.get().plusDays(7));
        assertEquals(0, progress.weekly(store, "Ana").wins, "a new week starts from zero");
    }

    @Test
    public void regularAiGamesAlwaysCountButNeverWhenTainted() throws Exception {
        UUID game = UUID.randomUUID();
        CareerTally practiced = tally("spells", 5);
        CareerTally.Recorder recorder = (CareerTally.Recorder) CareerTally.open(game, practiced.playerId);
        assertNotNull(recorder);
        CareerTally.taint(game, "practice tools were used");
        CareerTally taken = CareerTally.take(game);
        assertNotNull(taken.taint());
        assertNull(service.aiGameEnded(game, "Ana", null, true, 9, taken), "practice never counts");

        int xp = store.profile("Ana").xp;
        UUID counted = UUID.randomUUID();
        CareerProgress.CareerGameResult result = service.aiGameEnded(counted, "Ana", null, true, 9, tally("spells", 5));
        assertNotNull(result);
        assertFalse(result.career);
        assertEquals(CareerRewards.AI, result.kind);
        assertEquals(1, result.weekly.wins, "the weekly goal counts every win");
        assertEquals(5, (int) store.stats("Ana").get("spells"));
        assertTrue(result.xp > 0, "a counted game pays XP");
        assertEquals(xp + result.xp, store.profile("Ana").xp);
        assertNull(service.aiGameEnded(counted, "Ana", null, true, 9, tally("spells", 5)), "once per game");
    }

    @Test
    public void exportCarriesProgress() throws Exception {
        careerGame(true, tally("spells", 3));
        int achievements = store.achievements("Ana").size();
        assertTrue(achievements > 0);
        String exported = service.export("Ana");
        store.deleteProfile("Ana");
        assertTrue(store.stats("Ana").isEmpty());
        service.importFile("Ana", exported);
        assertEquals(achievements, store.achievements("Ana").size());
        assertEquals(3, (int) store.stats("Ana").get("spells"));
    }
}
