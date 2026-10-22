package mage.server.career;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.LocalDate;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicReference;
import java.util.stream.Collectors;
import java.util.stream.Stream;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Capsule machines (M29): the content, tokens, the draw, card styles.
 */
public class CareerCapsulesTest {

    private static final Path CLIENT_ART = Paths.get("../Mage.Web.Client/public/career-art");

    private Path file;
    private CareerStore store;
    private final CareerCapsules capsules = CareerCapsules.get();
    private final AtomicReference<LocalDate> day = new AtomicReference<>(LocalDate.of(2027, 5, 3));

    @BeforeEach
    public void setUp() throws Exception {
        file = Files.createTempFile("career-capsules-test", ".db");
        store = new CareerStore("jdbc:sqlite:" + file);
        CareerStore.use(store);
        CareerProgress.get().useClock(day::get, new Random(7));
        capsules.random(new Random(11));
    }

    @AfterEach
    public void tearDown() throws Exception {
        store.close();
        CareerStore.use(null);
        CareerProgress.get().useClock(() -> LocalDate.now(java.time.ZoneOffset.UTC), new Random());
        capsules.random(new java.security.SecureRandom());
        Files.deleteIfExists(file);
    }

    private void giveTokens(String user, int tokens) throws Exception {
        CareerService.ensureProfile(store, user, 0);
        CareerCapsules.addTokens(store, user, tokens, "test", 0);
    }

    /** every card class in Mage.Sets, by file name, with whether it's a single-faced card */
    private static Map<String, Boolean> cardClasses() throws IOException {
        Map<String, Boolean> classes = new HashMap<>();
        try (Stream<Path> files = Files.walk(Paths.get("../Mage.Sets/src/mage/cards"))) {
            for (Path path : files.filter(one -> one.toString().endsWith(".java")).collect(Collectors.toList())) {
                String name = path.getFileName().toString().replace(".java", "");
                if (!classes.containsKey(name)) {
                    String text = new String(Files.readAllBytes(path), java.nio.charset.StandardCharsets.UTF_8);
                    classes.put(name, !text.matches("(?s).*extends\\s+(\\w*DoubleFaced\\w*|SplitCard|AdventureCard|\\w*Omen\\w*|\\w*Modal\\w*)\\b.*"));
                }
            }
        }
        return classes;
    }

    /** a card name as its class is named: "Commander's Sphere" is CommandersSphere */
    private static String className(String card) {
        StringBuilder name = new StringBuilder();
        for (String word : card.replace("'", "").split("[^A-Za-z0-9]+")) {
            if (!word.isEmpty()) {
                name.append(Character.toUpperCase(word.charAt(0))).append(word.substring(1));
            }
        }
        return name.toString();
    }

    @Test
    public void everyMachineIsTenSetsOfFiveRealCardsWithPicturesAndRewards() throws Exception {
        Map<String, Boolean> classes = cardClasses();
        CareerCosmetics catalog = CareerCosmetics.get();
        Set<String> ids = new HashSet<>();
        assertFalse(capsules.machines().isEmpty());
        for (CareerCapsules.CareerMachine machine : capsules.machines()) {
            assertTrue(machine.id.matches("[a-z0-9-]+"), machine.id);
            assertNotNull(machine.name, machine.id);
            assertNotNull(machine.es, machine.id);
            assertNotNull(machine.cabinet, machine.id);
            assertTrue(CareerCosmetics.KINDS.contains(machine.kind), machine.id);
            assertNotNull(catalog.find(machine.reward.kind, machine.reward.id), machine.id + "'s reward is in the catalog");
            assertNotNull(catalog.find(machine.title.kind, machine.title.id), machine.id + "'s title is in the catalog");
            assertEquals(10, machine.sets.size(), machine.id + " has ten sets");
            Set<String> cards = new HashSet<>();
            for (CareerCapsules.CareerMachineSet set : machine.sets) {
                assertTrue(ids.add(set.id), set.id + " is unique");
                assertTrue(set.capsule.matches("#[0-9a-f]{6}"), set.id);
                assertNotNull(set.style, set.id);
                assertEquals(5, set.items.size(), set.id + " has five capsules");
                assertNotNull(catalog.find("sleeve", set.reward.id), set.id + "'s sleeve is in the catalog");
                assertTrue(Files.exists(CLIENT_ART.resolve("sleeve/" + set.reward.id + ".webp")), set.id + "'s sleeve is painted");
                for (CareerCapsules.CareerMachineItem item : set.items) {
                    assertTrue(ids.add(item.id), item.id + " is unique");
                    assertTrue(item.id.startsWith(set.id + "-"), item.id);
                    assertTrue(item.id.matches("[a-z0-9-]+"), item.id);
                    assertTrue(cards.add(item.card), item.card + " is in " + machine.id + " once");
                    Boolean single = classes.get(className(item.card));
                    assertNotNull(single, item.card + " is a card");
                    assertTrue(single, item.card + " has one face");
                    assertNotNull(catalog.find(machine.kind, item.id), item.id + " is in the catalog");
                    String slug = item.id.substring(set.id.length() + 1);
                    assertTrue(Files.exists(CLIENT_ART.resolve(machine.kind + "/" + set.id + "/" + slug + ".webp")), item.id + " is painted");
                }
            }
        }
    }

    @Test
    public void tokensAreBoughtWithCoinsInBundles() throws Exception {
        CareerService.ensureProfile(store, "ann", 0);
        store.addCoins("ann", 700);
        int price = CareerRewards.get().capsules().tokenPrice;
        CareerProfile after = capsules.buyTokens("ann", 5);
        assertEquals(5, after.capsuleTokens);
        assertEquals(700 - 5 * price, after.coins);
        assertThrows(CareerService.CareerException.class, () -> capsules.buyTokens("ann", 3));
        assertThrows(CareerService.CareerException.class, () -> capsules.buyTokens("ann", 10));
        assertEquals(5, store.profile("ann").capsuleTokens, "a refused purchase changes nothing");
    }

    @Test
    public void everyPullIsNewAndAMachineEndsInExactlyItsSizeOfPulls() throws Exception {
        CareerCapsules.CareerMachine machine = capsules.machines().get(capsules.machines().size() - 1);
        int size = machine.sets.size() * 5;
        giveTokens("bea", size + 20);
        Set<String> got = new HashSet<>();
        int sets = 0;
        boolean done = false;
        while (got.size() < size) {
            CareerCapsules.CareerPullResult result = capsules.pull("bea", machine.id, 10);
            for (CareerCapsules.CareerPull pull : result.pulls) {
                assertTrue(got.add(pull.item), pull.item + " came out once");
                if (pull.setDone) {
                    sets++;
                    assertTrue(store.unlocksOf("bea").stream().anyMatch(u -> u[1].equals(pull.setReward.id)), "the set's sleeve");
                }
                done |= pull.machineDone;
            }
            assertEquals(size - got.size(), result.left);
        }
        assertEquals(10, sets);
        assertTrue(done);
        assertEquals(20, store.profile("bea").capsuleTokens, "the last pull of 10 charged only what was left");
        Set<String> unlocked = store.unlocksOf("bea").stream().map(u -> u[0] + ":" + u[1]).collect(Collectors.toSet());
        assertTrue(unlocked.contains(machine.reward.kind + ":" + machine.reward.id));
        assertTrue(unlocked.contains(machine.title.kind + ":" + machine.title.id));
        assertThrows(CareerService.CareerException.class, () -> capsules.pull("bea", machine.id, 1), "nothing left");
        assertEquals(size, capsules.shop("bea").owned.size());
    }

    @Test
    public void aPullNeedsATokenForEachCapsule() throws Exception {
        String machine = capsules.machines().get(0).id;
        giveTokens("cal", 3);
        assertThrows(CareerService.CareerException.class, () -> capsules.pull("cal", machine, 10));
        assertEquals(3, store.profile("cal").capsuleTokens);
        assertTrue(capsules.shop("cal").owned.isEmpty(), "a refused pull gives nothing");
        assertEquals(3, capsules.pull("cal", machine, 3).pulls.size());
        assertEquals(0, store.profile("cal").capsuleTokens);
        assertThrows(CareerService.CareerException.class, () -> capsules.pull("cal", "no-such-machine", 1));
    }

    @Test
    public void aStyleIsWornOnlyOnItsCardAndOnlyOnceOwned() throws Exception {
        CareerCapsules.CareerMachineItem item = capsules.machines().get(0).sets.get(0).items.get(0);
        CareerService.ensureProfile(store, "dee", 0);
        assertThrows(CareerService.CareerException.class, () -> capsules.equipStyle("dee", item.card, item.id), "not owned");
        store.addUnlock("dee", CareerCapsules.CARD_STYLE, item.id, 0);
        assertThrows(CareerService.CareerException.class, () -> capsules.equipStyle("dee", "Island", item.id), "another card");
        assertEquals(item.id, capsules.equipStyle("dee", item.card, item.id).get(item.card));
        assertEquals(item.id, capsules.worn("dee").get(item.card));
        assertTrue(capsules.equipStyle("dee", item.card, null).isEmpty());
    }

    @Test
    public void theFirstWinOfTheDayPaysAToken() throws Exception {
        CareerService.FinishedGame game = new CareerService.FinishedGame(UUID.randomUUID(), CareerRewards.AI, 2, 9);
        game.seats.add(new CareerService.Seat("eve", true, CareerTally.forTest(UUID.randomUUID())));
        CareerProgress.CareerGameResult result = CareerService.get().gameEnded(game).get("eve");
        int firstWin = CareerRewards.get().capsules().firstWinTokens;
        assertTrue(result.tokens >= firstWin);
        assertTrue(result.bonuses.stream().anyMatch(bonus -> bonus.tokens == firstWin));
        assertEquals(result.tokens, store.profile("eve").capsuleTokens);
    }

    @Test
    public void everyFifthLevelAndTheHigherAchievementTiersPayTokens() {
        CareerRewards.Capsules rules = CareerRewards.get().capsules();
        for (CareerProgress.CareerLevelReward level : CareerProgress.get().levelRewards().values()) {
            assertEquals(level.level % rules.levelTokensEvery == 0 ? rules.levelTokens : 0, level.tokens, "level " + level.level);
        }
        CareerProgress.get().achievementTemplates().values().stream().filter(a -> a.tiers != null && a.tiers.size() == 4)
                .forEach(a -> assertEquals(rules.tierTokens.get(3).intValue(), a.tiers.get(3).tokens, a.id));
    }

    @Test
    public void cardStylesTravelWithAnExportedCareer() throws Exception {
        CareerCapsules.CareerMachineItem item = capsules.machines().get(0).sets.get(0).items.get(0);
        CareerService.ensureProfile(store, "fay", 0);
        store.addUnlock("fay", CareerCapsules.CARD_STYLE, item.id, 0);
        capsules.equipStyle("fay", item.card, item.id);
        CareerCapsules.addTokens(store, "fay", 4, "test", 0);
        String exported = CareerService.get().export("fay");
        CareerService.get().delete("fay");
        CareerService.get().importFile("fay", exported);
        assertEquals(4, store.profile("fay").capsuleTokens);
        assertEquals(item.id, capsules.worn("fay").get(item.card));
        List<String> owned = capsules.shop("fay").owned;
        assertTrue(owned.contains(item.id));
    }
}
