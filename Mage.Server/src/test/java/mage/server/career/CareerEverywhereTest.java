package mage.server.career;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import java.util.Map;
import java.util.Random;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicReference;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Progression from every game (B273): kinds, one claim per seat, the turn floor, the pair cap, the daily extras,
 * staff revokes and Career tables that outlive a restart.
 */
public class CareerEverywhereTest {

    private Path file;
    private CareerStore store;
    private final CareerService service = CareerService.get();
    private final AtomicReference<LocalDate> day = new AtomicReference<>(LocalDate.of(2027, 5, 3));

    @BeforeEach
    public void setUp() throws Exception {
        file = Files.createTempFile("career-everywhere-test", ".db");
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

    private static CareerTally tally() {
        return CareerTally.forTest(UUID.randomUUID());
    }

    private static CareerService.FinishedGame duel(String kind, int turns, String winner, String loser) {
        CareerService.FinishedGame game = new CareerService.FinishedGame(UUID.randomUUID(), kind, 2, turns);
        game.seats.add(new CareerService.Seat(winner, true, tally()));
        if (loser != null) {
            game.seats.add(new CareerService.Seat(loser, false, tally()));
        }
        return game;
    }

    private static int bonusCoins(CareerProgress.CareerGameResult result, String text) {
        return result.bonuses.stream().filter(b -> b.text.startsWith(text)).mapToInt(b -> b.coins).sum();
    }

    private static int bonusXp(CareerProgress.CareerGameResult result, String text) {
        return result.bonuses.stream().filter(b -> b.text.startsWith(text)).mapToInt(b -> b.xp).sum();
    }

    @Test
    public void kindsComeFromTheTable() {
        assertEquals(CareerRewards.AI, CareerService.kindOf(false, 2, 1));
        assertEquals(CareerRewards.PERSON, CareerService.kindOf(false, 2, 2));
        assertEquals(CareerRewards.POD, CareerService.kindOf(false, 4, 2));
        assertEquals(CareerRewards.EVENT, CareerService.kindOf(true, 2, 2));
    }

    @Test
    public void everySeatOfAGameAgainstAPersonGetsProgressOnce() throws Exception {
        CareerService.FinishedGame game = duel(CareerRewards.PERSON, 8, "Ana", "Bea");
        Map<String, CareerProgress.CareerGameResult> results = service.gameEnded(game);
        assertEquals(2, results.size());
        CareerProfile ana = store.profile("Ana");
        assertNotNull(ana, "progression starts with the first counted game");
        assertFalse(ana.collection, "without a Career collection");
        assertTrue(results.get("Ana").xp > results.get("Bea").xp, "a win pays more than a loss");
        assertTrue(results.get("Bea").xp > 0, "a loss still pays");
        assertEquals(results.get("Ana").xp, ana.xp);
        assertEquals(results.get("Ana").gameKey, CareerService.gameKey(game.gameId, "Ana"));
        assertNotNull(service.gameResult("Bea", results.get("Bea").gameKey));

        assertTrue(service.gameEnded(game).isEmpty(), "a game pays each seat once");
        assertEquals(ana.xp, store.profile("Ana").xp);
    }

    @Test
    public void taintedSeatsGetNothingButTheOthersStillCount() {
        UUID ana = UUID.randomUUID();
        UUID bea = UUID.randomUUID();
        UUID gameId = UUID.randomUUID();
        CareerTally.open(gameId, java.util.Arrays.asList(ana, bea));
        CareerTally.taint(gameId, bea, "practice tools were used");
        Map<UUID, CareerTally> tallies = CareerTally.takeAll(gameId);
        CareerService.FinishedGame game = new CareerService.FinishedGame(gameId, CareerRewards.PERSON, 2, 8);
        game.seats.add(new CareerService.Seat("Ana", true, tallies.get(ana)));
        game.seats.add(new CareerService.Seat("Bea", false, tallies.get(bea)));
        Map<String, CareerProgress.CareerGameResult> results = service.gameEnded(game);
        assertTrue(results.containsKey("Ana"));
        assertFalse(results.containsKey("Bea"));
    }

    @Test
    public void aGameThatEndsBeforeTheTurnFloorPaysNothing() throws Exception {
        CareerProgress.CareerGameResult result = service.gameEnded(duel(CareerRewards.AI, 2, "Ana", null)).get("Ana");
        assertNotNull(result);
        assertEquals(0, result.xp);
        assertEquals(0, result.coins);
        assertNotNull(result.note);
        assertEquals(0, store.profile("Ana").xp);
        assertEquals(0, store.daily("Ana", "2027-05-03")[1], "it doesn't count for the day either");
    }

    @Test
    public void theDayPaysTheWinsLadderAndTheFirstBonusesOnce() throws Exception {
        CareerRewards rewards = CareerRewards.get();
        CareerProgress.CareerGameResult first = service.gameEnded(duel(CareerRewards.PERSON, 8, "Ana", "Bea")).get("Ana");
        assertEquals(rewards.dailyWinCoins(1), bonusCoins(first, "Daily win 1"));
        assertEquals(rewards.firstWinXp(), bonusXp(first, "First win"));
        assertEquals(rewards.firstPersonGameXp(), bonusXp(first, "First game against a person"));
        CareerProgress.CareerGameResult second = service.gameEnded(duel(CareerRewards.AI, 8, "Ana", null)).get("Ana");
        assertEquals(rewards.dailyWinCoins(2), bonusCoins(second, "Daily win 2"));
        assertEquals(0, bonusXp(second, "First win"), "once a day");

        CareerProgress.CareerDaily today = CareerProgress.get().daily("Ana");
        assertEquals(2, today.wins);
        assertEquals(3, today.nextWin);
        assertTrue(today.firstWinPaid);
        assertTrue(today.firstPersonGamePaid);

        day.set(day.get().plusDays(1));
        CareerProgress.CareerGameResult tomorrow = service.gameEnded(duel(CareerRewards.AI, 8, "Ana", null)).get("Ana");
        assertEquals(rewards.dailyWinCoins(1), bonusCoins(tomorrow, "Daily win 1"), "the ladder starts again each day");
    }

    @Test
    public void thePairCapStopsTwoPeopleFarmingEachOther() {
        int cap = CareerRewards.get().pairCapPerDay();
        for (int i = 0; i < cap; i++) {
            assertTrue(service.gameEnded(duel(CareerRewards.PERSON, 8, "Ana", "Bea")).get("Ana").xp > 0);
        }
        CareerProgress.CareerGameResult capped = service.gameEnded(duel(CareerRewards.PERSON, 8, "Ana", "Bea")).get("Ana");
        assertEquals(0, capped.xp);
        assertNotNull(capped.note);
        assertTrue(service.gameEnded(duel(CareerRewards.PERSON, 8, "Ana", "Cal")).get("Ana").xp > 0, "someone else still counts");
        assertTrue(service.gameEnded(duel(CareerRewards.EVENT, 8, "Ana", "Bea")).get("Ana").xp > 0, "events aren't capped");
    }

    @Test
    public void podsPayMoreForMorePlayersAndTheAiPaysByStrength() {
        CareerRewards rewards = CareerRewards.get();
        assertTrue(rewards.gameXp(CareerRewards.POD, true, 4, 0) > rewards.gameXp(CareerRewards.POD, true, 3, 0));
        assertTrue(rewards.gameXp(CareerRewards.AI, true, 2, 10) > rewards.gameXp(CareerRewards.AI, true, 2, 1));
        assertTrue(rewards.gameXp(CareerRewards.PERSON, true, 2, 0) > rewards.gameXp(CareerRewards.AI, true, 2, 1));
    }

    @Test
    public void staffCanTakeBackWhatAGamePaid() throws Exception {
        CareerProgress.CareerGameResult result = service.gameEnded(duel(CareerRewards.PERSON, 8, "Ana", "Bea")).get("Ana");
        CareerService.CareerAdminView view = service.adminView("Ana");
        assertEquals(1, view.games.size());
        assertEquals(result.gameKey, view.games.get(0).gameKey);
        int xp = store.profile("Ana").xp;
        assertTrue(service.revoke("Ana", result.gameKey));
        assertEquals(xp - view.games.get(0).xp, store.profile("Ana").xp);
        assertFalse(service.revoke("Ana", result.gameKey), "once");
        assertFalse(service.revoke("Bea", result.gameKey), "only the account's own game");
    }

    @Test
    public void showProgressIsKeptWithTheAccount() throws Exception {
        assertTrue(service.profileOrNew("Ana").showProgress);
        assertFalse(service.setShowProgress("Ana", false).showProgress);
        assertFalse(store.profile("Ana").showProgress);
    }

    @Test
    public void careerTablesOutliveARestart() throws Exception {
        store.createProfile("Ana", "test", CareerRules.START_COINS, 1L);
        UUID table = UUID.randomUUID();
        service.register(table, "Ana", "wren", 1);
        service.dropTablesInMemory();
        assertNull(service.matchEnded(table, true, 7), "gone from memory");
        assertEquals(1, service.restoreTables());
        assertNotNull(service.matchEnded(table, true, 7), "and back after the restore");
        assertEquals(0, service.restoreTables(), "a paid table is dropped from the store");
    }

    @Test
    public void formatsAndColorsComeFromTheTableAndTheDeck() {
        assertEquals("commander", CareerService.formatOf("Variant Magic - Commander", "Commander Free For All", false));
        assertEquals("brawl", CareerService.formatOf("Variant Magic - Brawl", "Two Player Duel", false));
        assertEquals("modern", CareerService.formatOf("Constructed - Modern", "Two Player Duel", false));
        assertEquals("limited", CareerService.formatOf("Limited", "Two Player Duel", true));
        assertEquals("other", CareerService.formatOf("Constructed - Freeform", "Two Player Duel", false));
    }

    @Test
    public void masteriesRankUpFromGamesAndPayOnce() throws Exception {
        int first = CareerRewards.get().masteryRankGames().get(0);
        CareerProgress.CareerGameResult last = null;
        for (int i = 0; i < first; i++) {
            CareerService.FinishedGame game = duel(CareerRewards.AI, 8, "Ana", null);
            game.format = "pauper";
            game.seats.get(0).colors.add("R");
            last = service.gameEnded(game).get("Ana");
        }
        assertEquals(2, last.masteries.size(), "red and Pauper reach rank 1 together");
        java.util.Map<String, CareerProgress.CareerMastery> tracks = new java.util.HashMap<>();
        for (CareerProgress.CareerMastery mastery : CareerProgress.get().masteries("Ana")) {
            tracks.put(mastery.track, mastery);
        }
        assertEquals(1, tracks.get("color_R").rank);
        assertEquals(1, tracks.get("format_pauper").rank);
        assertEquals(0, tracks.get("color_U").rank);
        CareerService.FinishedGame more = duel(CareerRewards.AI, 8, "Ana", null);
        more.format = "pauper";
        assertTrue(service.gameEnded(more).get("Ana").masteries.isEmpty(), "a rank pays once");
    }

    @Test
    public void eventsPayPacksByRecordOnlyWithPeople() throws Exception {
        java.util.Map<String, Integer> wins = new java.util.LinkedHashMap<>();
        wins.put("Ana", 3);
        wins.put("Bea", 0);
        UUID event = UUID.randomUUID();
        java.util.Map<String, Integer> packs = service.eventEnded(event, true, wins);
        assertEquals(CareerRewards.get().eventPacks(true, 3, true), (int) packs.get("Ana"));
        assertEquals(CareerRewards.get().eventPacks(true, 0, true), (int) packs.get("Bea"));
        assertTrue(store.profile("Ana").packTokens >= packs.get("Ana"));
        assertTrue(service.eventEnded(event, true, wins).isEmpty(), "once per event");

        java.util.Map<String, Integer> alone = new java.util.LinkedHashMap<>();
        alone.put("Cal", 3);
        assertEquals(0, (int) service.eventEnded(UUID.randomUUID(), true, alone).get("Cal"), "against the AI only: XP, no packs");
        assertTrue(store.profile("Cal").xp > 0);
    }

    @Test
    public void renownStarsPastTheTopLevel() {
        int top = CareerRules.xpToReach(CareerRules.MAX_LEVEL);
        int renownXp = CareerRewards.get().renownXp();
        assertEquals(CareerRules.MAX_LEVEL, CareerRules.levelFor(top));
        assertEquals(0, CareerRules.renownFor(top, renownXp));
        assertEquals(2, CareerRules.renownFor(top + 2 * renownXp + 1, renownXp));
    }
}
