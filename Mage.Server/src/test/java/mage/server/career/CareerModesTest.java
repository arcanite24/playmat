package mage.server.career;

import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import java.util.Arrays;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Career modes: campaigns, puzzles, the weekly challenge and the gauntlet. The content is validated here (every node,
 * deck and reward is well formed; Mage.Tests checks the card names), and each mode's
 * progress and pay rules are pinned.
 */
public class CareerModesTest {

    private Path file;
    private CareerStore store;
    private Path soloFile;
    private mage.server.solo.SoloStore soloStore;
    private final CareerService service = CareerService.get();

    @BeforeEach
    public void setUp() throws Exception {
        file = Files.createTempFile("career-modes-test", ".db");
        store = new CareerStore("jdbc:sqlite:" + file);
        CareerStore.use(store);
        soloFile = Files.createTempFile("career-modes-solo", ".db");
        soloStore = new mage.server.solo.SoloStore("jdbc:sqlite:" + soloFile);
        mage.server.solo.SoloStore.use(soloStore);
        mage.server.solo.Leaderboards.get().forgetMigration();
        store.createProfile("Ana", "test", CareerRules.START_COINS, 1L);
    }

    @AfterEach
    public void tearDown() throws Exception {
        store.close();
        CareerStore.use(null);
        soloStore.close();
        mage.server.solo.SoloStore.use(null);
        Files.deleteIfExists(soloFile);
        CareerChallenges.get().useClock(() -> LocalDate.now(java.time.ZoneOffset.UTC));
        Files.deleteIfExists(file);
    }

    private CareerProgress.CareerGameResult play(String kind, String ref, CareerSetup setup, boolean won) throws Exception {
        UUID table = UUID.randomUUID();
        service.register(table, "Ana", kind, ref, setup);
        assertEquals(kind, service.kindOf(table));
        service.matchEnded(table, null, won, 7, CareerTally.forTest(UUID.randomUUID()));
        return service.gameResult("Ana", table.toString());
    }

    // ---- content

    /** every card name in the content, to check against the card database */
    private final Set<String> names = new HashSet<>();

    private void deck(String file) throws Exception {
        DeckCardLists deck = CareerCampaigns.get().deck(file);
        int count = 0;
        for (DeckCardInfo card : CareerContent.allCards(deck)) {
            count += card.getAmount();
            names.add(card.getCardName());
        }
        assertTrue(count >= 40, file + " has at least 40 cards, has " + count);
    }

    private void twists(String where, CareerSetup.CareerTwists twists) {
        if (twists == null) {
            return;
        }
        assertNotNull(CareerSetup.ofTwists(twists), where + " has twists that change something");
        if (twists.permanents != null) {
            twists.permanents.forEach(entry -> names.add(CareerSetup.name(entry)));
        }
        if (twists.emblem != null) {
            names.add(twists.emblem);
        }
    }

    private void side(String where, CareerSetup.CareerPuzzleSide side) {
        assertNotNull(side, where);
        for (List<String> zone : Arrays.asList(side.hand, side.battlefield, side.graveyard, side.library)) {
            if (zone != null) {
                zone.forEach(entry -> names.add(CareerSetup.name(entry)));
            }
        }
    }

    /** a format school's shape (CURRICULUM.md); Mage.Tests' CareerSchoolsTest checks its decks are legal and its cards real */
    private void school(Map<String, CareerCampaigns.Campaign> campaigns, CareerCampaigns.Campaign school) throws Exception {
        assertNotNull(school.format, school.id);
        assertNotNull(school.deckType, school.id);
        assertNotNull(school.gameType, school.id);
        assertEquals(4, school.chapters.size(), school.id + " has four acts");
        assertNotNull(school.requires, school.id + " opens by its requirements");
        for (String requirement : school.requires) {
            for (String alternative : requirement.split("\\|")) {
                String id = alternative.contains("@") ? alternative.substring(0, alternative.indexOf('@')) : alternative;
                assertTrue(campaigns.containsKey(id), school.id + " requires a campaign, not " + alternative);
            }
        }
        Set<String> ids = new HashSet<>();
        CareerCampaigns.Node last = null;
        for (CareerCampaigns.Chapter chapter : school.chapters) {
            for (CareerCampaigns.Node node : chapter.nodes) {
                assertTrue(ids.add(node.id), "node ids are unique: " + node.id);
                if ("duel".equals(node.type)) {
                    assertNotNull(node.deck != null ? node.deck : chapter.deck, node.id + " has a deck to play");
                    deck(node.deck != null ? node.deck : chapter.deck);
                    assertNotNull(node.opponent, node.id);
                    deck(node.opponent.deck);
                    twists(node.id, node.opponent.twists);
                    if (node.opponents != null) {
                        for (CareerCampaigns.NodeOpponent other : node.opponents) {
                            deck(other.deck);
                        }
                    }
                    assertNotNull(node.lesson, node.id + " teaches something");
                } else if ("trial".equals(node.type)) {
                    assertNotNull(node.trial, node.id);
                    side(node.id + " you", node.trial.you);
                    side(node.id + " opponent", node.trial.opponent);
                } else {
                    assertEquals("choice", node.type, node.id);
                }
                last = node;
            }
        }
        assertNotNull(last);
        assertEquals(2, last.winsNeeded, school.id + " ends in a best of three");
        assertTrue(last.reward != null && last.reward.cosmetics != null && last.reward.cosmetics.size() >= 2,
                school.id + " graduates with a title and a sleeve");
    }

    @Test
    public void theContentIsWellFormed() throws Exception {
        Map<String, CareerCampaigns.Campaign> campaigns = CareerCampaigns.get().campaigns();
        assertFalse(campaigns.isEmpty(), "a campaign");
        for (CareerCampaigns.Campaign campaign : campaigns.values()) {
            if (campaign.school) {
                school(campaigns, campaign);
                continue;
            }
            assertEquals(5, campaign.chapters.size(), campaign.id + " has five chapters");
            Set<String> ids = new HashSet<>();
            for (CareerCampaigns.Chapter chapter : campaign.chapters) {
                deck(chapter.deck);
                assertNotNull(chapter.deckName, chapter.id);
                int duels = 0;
                boolean boss = false;
                for (CareerCampaigns.Node node : chapter.nodes) {
                    assertTrue(ids.add(node.id), "node ids are unique: " + node.id);
                    if (node.requires != null) {
                        for (String requirement : node.requires) {
                            String id = requirement.contains(":") ? requirement.substring(0, requirement.indexOf(':')) : requirement;
                            assertTrue(ids.contains(id), node.id + " requires an earlier node, not " + requirement);
                        }
                    }
                    if ("duel".equals(node.type)) {
                        duels++;
                        boss |= node.boss;
                        assertNotNull(node.opponent, node.id);
                        assertTrue(node.opponent.skill >= 1 && node.opponent.skill <= 10, node.id + " skill");
                        assertNotNull(node.reward, node.id + " pays something");
                        deck(node.opponent.deck);
                        twists(node.id, node.opponent.twists);
                    } else {
                        assertEquals("choice", node.type, node.id);
                        assertTrue(node.options != null && node.options.size() >= 2, node.id + " offers a choice");
                        node.options.forEach(option -> assertNotNull(option.reward, node.id + "/" + option.id));
                    }
                }
                assertEquals(5, duels, chapter.id + " has five duels");
                assertTrue(boss, chapter.id + " ends with a boss");
            }
            assertTrue(campaign.chapters.stream().map(chapter -> chapter.unlockSet).allMatch(set -> set != null),
                    "each chapter opens a set");
        }

        assertEquals(25, CareerPuzzles.get().puzzles().size(), "twenty puzzles and the five stack drills");
        for (CareerSetup.Puzzle puzzle : CareerPuzzles.get().puzzles().values()) {
            assertNotNull(puzzle.name, puzzle.id);
            assertNull(puzzle.problem(), puzzle.id + ": " + puzzle.problem());
            assertNotNull(puzzle.hint, puzzle.id + " has a hint");
            side(puzzle.id + " you", puzzle.you);
            side(puzzle.id + " opponent", puzzle.opponent);
        }

        assertTrue(CareerChallenges.get().challenges().size() >= 4, "a few weeks of challenges");
        for (CareerChallenges.Challenge challenge : CareerChallenges.get().challenges()) {
            deck(challenge.playerDeck);
            deck(challenge.opponent.deck);
            twists(challenge.id, challenge.opponent.twists);
        }

        CareerGauntlet.Rules gauntlet = CareerGauntlet.get().rules();
        assertEquals(gauntlet.maxWins, gauntlet.bosses.size(), "a boss for each win");
        assertEquals(gauntlet.maxWins + 1, gauntlet.rewards.size(), "a reward for every result");
        for (CareerCampaigns.NodeOpponent boss : gauntlet.bosses) {
            deck(boss.deck);
            twists(boss.name, boss.twists);
        }
        assertTrue(CareerJumpstart.get().packs().size() >= gauntlet.startPacks, "enough packs to offer");
        CareerJumpstart.get().packs().forEach(pack -> pack.cards.forEach(card -> names.add(card.getCardName())));

        // card names are checked against every card in Mage.Tests: CareerContentCardsTest
    }

    // ---- campaigns

    @Test
    public void aCampaignOpensInOrderAndPaysTheFirstWinOnly() throws Exception {
        CareerCampaigns campaigns = CareerCampaigns.get();
        CareerCampaigns.CareerChapter white = campaigns.campaigns("Ana").get(0).chapters.get(0);
        assertEquals("open", white.nodes.get(0).state);
        assertEquals("locked", white.nodes.get(1).state);
        assertThrows(CareerService.CareerException.class, () -> campaigns.plan("Ana", "five-paths", "white-2"));

        CareerCampaigns.DuelPlan plan = campaigns.plan("Ana", "five-paths", "white-1");
        assertEquals("five-paths/white-1", plan.key);
        assertNull(plan.setup, "no twists for the first duel");

        int coins = store.profile("Ana").coins;
        CareerProgress.CareerGameResult lost = play("campaign", plan.key, null, false);
        assertEquals("campaign", lost.mode.kind);
        assertEquals("locked", campaigns.campaigns("Ana").get(0).chapters.get(0).nodes.get(1).state, "a loss opens nothing");

        CareerProgress.CareerGameResult won = play("campaign", plan.key, null, true);
        assertEquals(25, won.mode.coins, "the node's coins");
        assertEquals(coins + lost.coins + won.coins, store.profile("Ana").coins);
        assertEquals("done", campaigns.campaigns("Ana").get(0).chapters.get(0).nodes.get(0).state);
        assertEquals("open", campaigns.campaigns("Ana").get(0).chapters.get(0).nodes.get(1).state);

        CareerProgress.CareerGameResult again = play("campaign", plan.key, null, true);
        assertEquals(0, again.mode.coins, "a node pays once");
    }

    @Test
    public void aChoicePaysOnceAndTheBossHasItsTwists() throws Exception {
        CareerCampaigns campaigns = CareerCampaigns.get();
        assertThrows(CareerService.CareerException.class, () -> campaigns.choose("Ana", "five-paths", "white-choice", "coins"));
        for (String node : Arrays.asList("white-1", "white-2", "white-3")) {
            play("campaign", "five-paths/" + node, null, true);
        }
        int coins = store.profile("Ana").coins;
        CareerProgress.CareerGameResult chose = campaigns.choose("Ana", "five-paths", "white-choice", "coins");
        assertEquals(60, chose.coins);
        assertEquals(coins + 60, store.profile("Ana").coins);
        assertThrows(CareerService.CareerException.class, () -> campaigns.choose("Ana", "five-paths", "white-choice", "xp"));
        assertEquals("coins", campaigns.campaigns("Ana").get(0).chapters.get(0).nodes.get(3).choice);

        play("campaign", "five-paths/white-4", null, true);
        CareerCampaigns.DuelPlan boss = campaigns.plan("Ana", "five-paths", "white-5");
        assertNotNull(boss.setup, "the boss has an edge");
        assertEquals(Integer.valueOf(25), boss.setup.twists().startingLife);

        assertFalse(store.setUnlocks("Ana").contains("M21"));
        CareerProgress.CareerGameResult beat = play("campaign", "five-paths/white-5", boss.setup, true);
        assertEquals("M21", beat.mode.unlockedSet, "the chapter opens its set");
        assertTrue(store.setUnlocks("Ana").contains("M21"));
        assertTrue(campaigns.campaigns("Ana").get(0).chapters.get(0).done);
    }

    // ---- schools

    private CareerCampaigns.CareerCampaign view(String id) throws Exception {
        return CareerCampaigns.get().campaigns("Ana").stream().filter(campaign -> campaign.id.equals(id)).findFirst().orElse(null);
    }

    private CareerCampaigns.Node node(String campaign, String id) {
        return CareerCampaigns.get().campaigns().get(campaign).chapters.stream().flatMap(chapter -> chapter.nodes.stream())
                .filter(node -> node.id.equals(id)).findFirst().orElse(null);
    }

    @Test
    public void aSchoolOpensByItsRequirements() throws Exception {
        CareerCampaigns campaigns = CareerCampaigns.get();
        CareerCampaigns.CareerCampaign pauper = view("pauper");
        assertTrue(pauper.school);
        assertFalse(pauper.open, "closed until a Five Paths boss falls");
        assertEquals(Arrays.asList("five-paths@1"), pauper.missing);
        assertEquals("locked", pauper.chapters.get(0).nodes.get(0).state, "a closed school's first duel is locked");
        assertThrows(CareerService.CareerException.class, () -> campaigns.plan("Ana", "pauper", "pauper-1a"));

        store.finishCampaignNode("Ana", "five-paths", "white-5", null, 1L);
        pauper = view("pauper");
        assertTrue(pauper.open);
        assertEquals("open", pauper.chapters.get(0).nodes.get(0).state);
        assertEquals(1, view("five-paths").bosses);
        assertFalse(view("pioneer").open, "Pioneer wants two Pauper acts or three Five Paths chapters");

        store.finishCampaignNode("Ana", "five-paths", "blue-5", null, 1L);
        store.finishCampaignNode("Ana", "five-paths", "black-5", null, 1L);
        assertTrue(view("pioneer").open, "either alternative opens it");
    }

    @Test
    public void aSchoolDuelIsPlayedInItsFormat() throws Exception {
        CareerCampaigns campaigns = CareerCampaigns.get();
        store.finishCampaignNode("Ana", "five-paths", "white-5", null, 1L);
        CareerCampaigns.DuelPlan first = campaigns.plan("Ana", "pauper", "pauper-1a");
        assertEquals("campaign", first.kind);
        assertEquals("Constructed - Pauper", first.deckType);
        assertEquals("Two Player Duel", first.gameType);
        assertEquals(1, first.winsNeeded);
        assertEquals(1, first.seats.size());
        assertEquals(first.opponentName, first.seats.get(0).name);
        assertTrue(first.setup == null || !first.setup.endsWhenHumansAreOut());

        // a pod: the same table seats every AI, as a free-for-all
        CareerCampaigns.Node pod = node("commander", "commander-4a");
        before:
        for (CareerCampaigns.Chapter chapter : campaigns.campaigns().get("commander").chapters) {
            for (CareerCampaigns.Node node : chapter.nodes) {
                if (node == pod) {
                    break before;
                }
                store.finishCampaignNode("Ana", "commander", node.id, node.options == null ? null : node.options.get(0).id, 1L);
            }
        }
        CareerCampaigns.DuelPlan podPlan = campaigns.plan("Ana", "commander", "commander-4a");
        assertEquals("Commander Free For All", podPlan.gameType);
        assertEquals("Variant Magic - Commander", podPlan.deckType);
        assertEquals(1 + pod.opponents.size(), podPlan.seats.size());
        assertTrue(podPlan.seats.size() >= 3, "a four-player pod");
        assertTrue(podPlan.setup.endsWhenHumansAreOut(), "a pod ends once the player is out");
    }

    @Test
    public void aTrialIsATurnThatFinishesItsNode() throws Exception {
        CareerCampaigns campaigns = CareerCampaigns.get();
        store.finishCampaignNode("Ana", "five-paths", "white-5", null, 1L);
        CareerCampaigns.Node trial = node("pauper", "pauper-1t");
        assertEquals("trial", trial.type);
        for (String id : trial.requires) {
            store.finishCampaignNode("Ana", "pauper", id, null, 1L);
        }
        CareerCampaigns.DuelPlan plan = campaigns.plan("Ana", "pauper", "pauper-1t");
        assertEquals("trial", plan.kind);
        assertTrue(plan.setup.replacesOpeningHands(), "a set position");
        assertNotNull(plan.setup.puzzle().you);

        CareerProgress.CareerGameResult failed = play("trial", plan.key, plan.setup, false);
        assertEquals(0, failed.coins, "a failed trial pays nothing");
        assertTrue(failed.mode.trial);
        CareerProgress.CareerGameResult passed = play("trial", plan.key, plan.setup, true);
        assertEquals(trial.reward.coins, passed.coins, "only the node's reward: a trial isn't a game");
        assertEquals("done", view("pauper").chapters.get(0).nodes.stream().filter(node -> node.id.equals("pauper-1t"))
                .findFirst().get().state);
        assertEquals(0, store.weeklyWins("Ana", CareerProgress.get().week()), "a trial isn't a weekly win");
    }

    @Test
    public void graduationPaysTheSchoolsTitleAndSleeve() throws Exception {
        store.finishCampaignNode("Ana", "five-paths", "white-5", null, 1L);
        CareerCampaigns.Campaign pauper = CareerCampaigns.get().campaigns().get("pauper");
        CareerCampaigns.Node last = null;
        for (CareerCampaigns.Chapter chapter : pauper.chapters) {
            for (CareerCampaigns.Node node : chapter.nodes) {
                if (last != null) {
                    store.finishCampaignNode("Ana", "pauper", last.id, last.options == null ? null : last.options.get(0).id, 1L);
                }
                last = node;
            }
        }
        CareerCampaigns.DuelPlan finalPlan = CareerCampaigns.get().plan("Ana", "pauper", last.id);
        assertEquals(2, finalPlan.winsNeeded, "the final is a best of three");
        CareerProgress.CareerGameResult graduated = play("campaign", finalPlan.key, finalPlan.setup, true);
        assertEquals(2, graduated.mode.cosmetics.size(), "a title and a sleeve");
        assertTrue(graduated.mode.lines.stream().anyMatch(line -> line.startsWith("You graduated")));
        assertEquals(Integer.valueOf(1), store.stats("Ana").get("schools_graduated"));
        assertEquals(pauper.chapters.size(), view("pauper").bosses);
    }

    // ---- puzzles

    @Test
    public void puzzlesOpenInOrderAndStarsGoByAttempts() throws Exception {
        CareerPuzzles puzzles = CareerPuzzles.get();
        List<CareerPuzzles.CareerPuzzle> book = puzzles.puzzles("Ana");
        assertTrue(book.get(2).open, "the first three are open");
        assertFalse(book.get(3).open);
        String first = book.get(0).id;
        assertThrows(CareerService.CareerException.class, () -> puzzles.attempt("Ana", book.get(3).id));

        CareerSetup.Puzzle puzzle = puzzles.attempt("Ana", first);
        CareerSetup setup = CareerSetup.ofPuzzle(puzzle);
        assertTrue(setup.replacesOpeningHands());
        CareerProgress.CareerGameResult failed = play("puzzle", first, setup, false);
        assertEquals(0, failed.coins, "a puzzle pays nothing for a loss");
        assertEquals(0, failed.xp, "and nothing for the game itself");

        puzzles.attempt("Ana", first);
        int coins = store.profile("Ana").coins;
        CareerProgress.CareerGameResult solved = play("puzzle", first, setup, true);
        assertEquals(2, solved.mode.stars, "solved on the second try");
        assertEquals(CareerPuzzles.FIRST_SOLVE_COINS, solved.mode.coins);
        assertEquals(coins + solved.coins, store.profile("Ana").coins);

        puzzles.attempt("Ana", first);
        CareerProgress.CareerGameResult again = play("puzzle", first, setup, true);
        assertEquals(0, again.coins, "a puzzle pays once");
        assertEquals(2, puzzles.puzzles("Ana").get(0).stars, "the best stars stay");
        assertTrue(puzzles.puzzles("Ana").get(1).open);
        assertEquals(3, CareerPuzzles.stars(1));
        assertEquals(2, CareerPuzzles.stars(3));
        assertEquals(1, CareerPuzzles.stars(4));
    }

    // ---- the weekly challenge

    @Test
    public void theChallengeRotatesWeeklyAndRanksByTurns() throws Exception {
        CareerChallenges challenges = CareerChallenges.get();
        challenges.useClock(() -> LocalDate.of(2027, 5, 3));
        String week = challenges.view("Ana").week;
        String id = challenges.view("Ana").id;
        challenges.useClock(() -> LocalDate.of(2027, 5, 10));
        assertFalse(id.equals(challenges.view("Ana").id) && week.equals(challenges.view("Ana").week), "a new week");
        challenges.useClock(() -> LocalDate.of(2027, 5, 3));

        store.createProfile("Bo", "test", CareerRules.START_COINS, 1L);
        CareerChallenges.Plan plan = challenges.attempt("Ana");
        assertEquals(week, plan.week);
        int coins = store.profile("Ana").coins;
        CareerProgress.CareerGameResult won = play("challenge", plan.week, plan.setup, true);
        assertEquals(CareerChallenges.FIRST_WIN_COINS, won.mode.coins);
        assertEquals(coins + won.coins, store.profile("Ana").coins);
        assertEquals(0, play("challenge", plan.week, plan.setup, true).mode.coins, "the first win of the week pays");
        assertNotNull(won.mode);

        CareerChallenges.CareerChallenge view = challenges.view("Ana");
        assertTrue(view.won);
        assertEquals(1, view.board.size());
        assertTrue(view.board.get(0).you);
    }

    // ---- the gauntlet

    @Test
    public void aGauntletRunPicksPlaysAndPaysByWins() throws Exception {
        CareerGauntlet gauntlet = CareerGauntlet.get();
        int coins = store.profile("Ana").coins;
        CareerGauntlet.CareerRun run = gauntlet.start("Ana");
        assertEquals(coins - gauntlet.rules().entryCoins, store.profile("Ana").coins);
        assertThrows(CareerService.CareerException.class, () -> gauntlet.start("Ana"), "one run at a time");
        assertThrows(CareerService.CareerException.class, () -> gauntlet.plan("Ana"), "pick first");
        assertEquals(3, run.offer.options.size());
        assertThrows(CareerService.CareerException.class,
                () -> gauntlet.pick("Ana", Arrays.asList(run.offer.options.get(0).id)), "pick two");

        CareerGauntlet.CareerRun picked = gauntlet.pick("Ana", Arrays.asList(run.offer.options.get(0).id, run.offer.options.get(1).id));
        assertNull(picked.offer);
        CareerGauntlet.Plan plan = gauntlet.plan("Ana");
        int cards = plan.playerDeck.getCards().stream().mapToInt(DeckCardInfo::getAmount).sum();
        assertTrue(cards >= 40, "two packs make a deck, has " + cards);

        play("gauntlet", plan.runId, plan.setup, true);
        CareerGauntlet.CareerRun afterWin = gauntlet.state("Ana").run;
        assertEquals(1, afterWin.wins);
        assertNotNull(afterWin.offer, "a win offers a bundle");
        gauntlet.pick("Ana", Arrays.asList(afterWin.offer.options.get(0).id));

        int before = store.profile("Ana").coins;
        CareerProgress.CareerGameResult lost = play("gauntlet", plan.runId, null, false);
        assertNull(gauntlet.state("Ana").run, "a loss ends the run");
        assertEquals(30, lost.mode.coins, "one win pays 30");
        assertEquals(before + lost.coins, store.profile("Ana").coins);
        assertNull(play("gauntlet", plan.runId, null, true).mode, "a finished run counts no more games");
    }
}
