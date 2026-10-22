package mage.server.career;

import com.google.gson.Gson;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashSet;
import java.util.List;
import java.util.Random;
import java.util.Set;
import java.util.UUID;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * The gauntlet's map, seeds, ascension and line modes (S022–S028): maps are well formed and come from the seed, a
 * shared seed replays the same run, a map can be walked from the first fight to the last boss, and boss rush and
 * endless run down the roster.
 */
public class CareerGauntletMapTest {

    private Path file;
    private CareerStore store;
    private Path soloFile;
    private mage.server.solo.SoloStore soloStore;
    private final CareerService service = CareerService.get();
    private final CareerGauntlet gauntlet = CareerGauntlet.get();

    @BeforeEach
    public void setUp() throws Exception {
        file = Files.createTempFile("career-gauntlet-test", ".db");
        store = new CareerStore("jdbc:sqlite:" + file);
        CareerStore.use(store);
        soloFile = Files.createTempFile("career-gauntlet-solo", ".db");
        soloStore = new mage.server.solo.SoloStore("jdbc:sqlite:" + soloFile);
        mage.server.solo.SoloStore.use(soloStore);
        mage.server.solo.Leaderboards.get().forgetMigration();
        store.createProfile("Ana", "test", CareerRules.START_COINS, 1L);
        store.addCoins("Ana", 100_000);
    }

    @AfterEach
    public void tearDown() throws Exception {
        store.close();
        CareerStore.use(null);
        soloStore.close();
        mage.server.solo.SoloStore.use(null);
        Files.deleteIfExists(soloFile);
        Files.deleteIfExists(file);
    }

    private void play(String runId, boolean won) throws Exception {
        UUID table = UUID.randomUUID();
        CareerGauntlet.Plan plan = gauntlet.plan("Ana");
        assertEquals(runId, plan.runId);
        service.register(table, "Ana", "gauntlet", plan.runId, plan.setup);
        service.matchEnded(table, null, won, 7, CareerTally.forTest(UUID.randomUUID()));
    }

    private CareerGauntlet.CareerRun takeFirst(CareerGauntlet.CareerRun run) throws Exception {
        List<String> ids = new ArrayList<>();
        for (int i = 0; i < run.offer.pick; i++) {
            ids.add(run.offer.options.get(i).id);
        }
        return gauntlet.pick("Ana", ids);
    }

    private static CareerGauntlet.StartOptions options(String mode, String seed, int ascension) {
        CareerGauntlet.StartOptions options = new CareerGauntlet.StartOptions();
        options.mode = mode;
        options.seed = seed;
        options.ascension = ascension;
        return options;
    }

    @Test
    public void seedsReadBackAndRefuseNonsense() {
        Random random = new Random(7);
        for (int i = 0; i < 500; i++) {
            long seed = random.nextLong() & 0xFFFFFFFFFFL;
            String code = CareerGauntlet.seedCode(seed);
            assertTrue(code.matches("[0-9A-Z]{4}-[0-9A-Z]{4}"), code);
            assertEquals(seed, (long) CareerGauntlet.parseSeed(code));
            assertEquals(seed, (long) CareerGauntlet.parseSeed(" " + code.toLowerCase() + " "), "case and spaces don't matter");
        }
        assertNull(CareerGauntlet.parseSeed("nope"));
        assertNull(CareerGauntlet.parseSeed(""));
    }

    @Test
    public void mapsAreWellFormedAndComeFromTheSeed() {
        CareerGauntlet.MapRules rules = gauntlet.rules().map;
        int bosses = gauntlet.rules().bosses.size();
        Gson gson = new Gson();
        for (long seed = 0; seed < 300; seed++) {
            List<List<CareerGauntlet.Node>> map = CareerGauntlet.generateMap(rules, bosses, new Random(seed));
            assertEquals(gson.toJson(map), gson.toJson(CareerGauntlet.generateMap(rules, bosses, new Random(seed))), "same seed, same map");
            assertEquals(rules.rows, map.size());
            List<CareerGauntlet.Node> top = map.get(map.size() - 1);
            assertEquals(1, top.size());
            assertEquals("boss", top.get(0).type);
            assertEquals(bosses - 1, top.get(0).opponent);
            assertTrue(map.get(map.size() - 2).stream().anyMatch(node -> "rest".equals(node.type)), "a rest before the boss");
            assertTrue(map.get(0).stream().allMatch(node -> "fight".equals(node.type)), "the first row is fights");
            Set<String> reachable = new HashSet<>();
            map.get(0).forEach(node -> reachable.add(node.id));
            for (int r = 0; r < map.size() - 1; r++) {
                for (CareerGauntlet.Node node : map.get(r)) {
                    assertFalse(node.links.isEmpty(), "every node leads on: " + node.id);
                    if (reachable.contains(node.id)) {
                        reachable.addAll(node.links);
                    }
                    if (r < 2) {
                        assertFalse("elite".equals(node.type) || "shop".equals(node.type), "no elites or shops early");
                    }
                }
            }
            for (List<CareerGauntlet.Node> row : map) {
                for (CareerGauntlet.Node node : row) {
                    assertTrue(reachable.contains(node.id), "seed " + seed + ": " + node.id + " can be reached");
                    if (CareerGauntlet.isFight(node)) {
                        assertTrue(node.opponent >= 0 && node.opponent < bosses);
                    }
                }
            }
        }
    }

    @Test
    public void aSharedSeedReplaysTheSameRun() throws Exception {
        String seed = CareerGauntlet.seedCode(123456789L);
        CareerGauntlet.CareerRun first = gauntlet.start("Ana", options("map", seed, 0));
        assertEquals(seed, first.seed);
        gauntlet.abandon("Ana");
        CareerGauntlet.CareerRun second = gauntlet.start("Ana", options("map", seed, 0));
        Gson gson = new Gson();
        assertEquals(gson.toJson(first.map), gson.toJson(second.map));
        assertEquals(gson.toJson(first.offer.options), gson.toJson(second.offer.options), "the same starting packs");
        gauntlet.abandon("Ana");
        CareerGauntlet.CareerRun other = gauntlet.start("Ana", options("map", null, 0));
        assertNotNull(other.seed);
        assertThrows(CareerService.CareerException.class, () -> gauntlet.start("Ana", options("map", null, 0)), "one run at a time");
        gauntlet.abandon("Ana");
        assertThrows(CareerService.CareerException.class, () -> gauntlet.start("Ana", options("map", "not a seed!", 0)));
        assertThrows(CareerService.CareerException.class, () -> gauntlet.start("Ana", options("sideways", null, 0)));
        assertThrows(CareerService.CareerException.class, () -> gauntlet.start("Ana", options("map", null, 1)), "ascension opens by beating the map");
    }

    @Test
    public void aMapCanBeWalkedToTheLastBossAndOpensAscension() throws Exception {
        CareerGauntlet.CareerRun run = takeFirst(gauntlet.start("Ana", options("map", CareerGauntlet.seedCode(42L), 0)));
        assertNull(run.offer);
        assertEquals(run.map.get(0).size(), run.choices.size(), "the first row is open");
        assertThrows(CareerService.CareerException.class, () -> gauntlet.plan("Ana"), "choose a node first");
        String runId = run.id;
        Set<String> seen = new HashSet<>();
        for (int step = 0; step < 100 && run != null && "active".equals(run.state); step++) {
            final CareerGauntlet.CareerRun now = run;
            if (now.offer != null) {
                seen.add(now.offer.kind);
                if ("shop".equals(now.offer.kind)) {
                    CareerGauntlet.CareerOfferOption buy = now.offer.options.stream()
                            .filter(option -> option.price != null && option.price <= now.runCoins).findFirst().orElse(null);
                    if (buy != null) {
                        run = gauntlet.pick("Ana", Collections.singletonList(buy.id));
                        assertEquals(now.runCoins - buy.price, run.runCoins, "the shop takes run coins");
                        assertNotNull(run.offer, "the shop stays open until you leave");
                    }
                    run = gauntlet.pick("Ana", Collections.singletonList("leave"));
                } else if (now.offer.pick > 1) {
                    run = takeFirst(now);
                } else {
                    run = gauntlet.pick("Ana", Collections.singletonList(now.offer.options.get(0).id));
                }
                continue;
            }
            if (now.next != null && now.at != null && now.choices.isEmpty()) {
                play(runId, true);
                run = gauntlet.state("Ana").run;
                continue;
            }
            assertFalse(now.choices.isEmpty(), "somewhere to go at step " + step);
            run = gauntlet.move("Ana", now.choices.get(0));
            final String at = run.at;
            seen.add(run.map.stream().flatMap(List::stream).filter(node -> node.id.equals(at)).findFirst().get().type);
        }
        assertNull(gauntlet.state("Ana").run, "the run is over");
        assertTrue(seen.contains("boss"));
        assertEquals("won", gauntlet.state("Ana").recent.get(0).state);
        assertEquals(1, gauntlet.state("Ana").ascensionOpen);
        CareerGauntlet.CareerRun ascended = gauntlet.start("Ana", options("map", null, 1));
        assertEquals(1, ascended.ascension);
        assertNotNull(ascended.ascensionTitle);
    }

    @Test
    public void bossRushRunsTheRosterAndEndlessKeepsScaling() throws Exception {
        List<CareerContent.Opponent> roster = CareerGauntlet.rosterOrder();
        assertFalse(roster.isEmpty());
        for (int i = 1; i < roster.size(); i++) {
            assertTrue(roster.get(i - 1).tier <= roster.get(i).tier, "the roster goes by tier");
        }
        CareerGauntlet.CareerRun rush = takeFirst(gauntlet.start("Ana", options("bossrush", null, 0)));
        assertEquals(roster.get(0).name, rush.next.name);
        play(rush.id, true);
        CareerGauntlet.CareerRun afterWin = gauntlet.state("Ana").run;
        assertEquals(roster.get(1).name, afterWin.next.name);
        assertNull(afterWin.offer, "boss rush keeps one deck");
        play(rush.id, false);
        assertNull(gauntlet.state("Ana").run);

        CareerGauntlet.CareerRun endless = takeFirst(gauntlet.start("Ana", options("endless", null, 0)));
        int firstSkill = gauntlet.plan("Ana").skill;
        for (int i = 0; i < 6; i++) {
            play(endless.id, true);
            CareerGauntlet.CareerRun now = gauntlet.state("Ana").run;
            assertNotNull(now.offer, "endless offers a bundle after each win");
            takeFirst(now);
        }
        assertTrue(gauntlet.plan("Ana").skill >= firstSkill, "endless never gets easier");
        assertEquals(6, gauntlet.state("Ana").run.wins);
    }
}
