package mage.server.career;

import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.server.solo.GhostDecks;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.UUID;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Single player past the gauntlet (S029–S050): the daily puzzle, puzzles from files, a year of weekly challenges, the
 * ladder's nemesis, rival and list rotation, adaptive skill, and ghost decks with their owner's inbox.
 */
public class SoloLadderTest {

    private Path file;
    private CareerStore store;
    private Path soloFile;
    private mage.server.solo.SoloStore soloStore;
    private final CareerService service = CareerService.get();

    @BeforeEach
    public void setUp() throws Exception {
        file = Files.createTempFile("solo-ladder-test", ".db");
        store = new CareerStore("jdbc:sqlite:" + file);
        CareerStore.use(store);
        soloFile = Files.createTempFile("solo-ladder-solo", ".db");
        soloStore = new mage.server.solo.SoloStore("jdbc:sqlite:" + soloFile);
        mage.server.solo.SoloStore.use(soloStore);
        mage.server.solo.Leaderboards.get().forgetMigration();
        store.createProfile("Ana", "test", CareerRules.START_COINS, 1L);
    }

    @AfterEach
    public void tearDown() throws Exception {
        CareerPuzzles.get().useClock(() -> LocalDate.now(java.time.ZoneOffset.UTC));
        store.close();
        CareerStore.use(null);
        soloStore.close();
        mage.server.solo.SoloStore.use(null);
        Files.deleteIfExists(soloFile);
        Files.deleteIfExists(file);
    }

    @Test
    public void theDailyPuzzleIsPinnedAndGoesRoundThePool() throws Exception {
        CareerPuzzles puzzles = CareerPuzzles.get();
        LocalDate day = LocalDate.of(2026, 10, 13);
        CareerSetup.Puzzle today = puzzles.daily(day.toString());
        assertEquals(today.id, puzzles.daily(day.toString()).id, "a day keeps its puzzle");
        Set<String> seen = new HashSet<>();
        for (int i = 0; i < 10; i++) {
            seen.add(puzzles.daily(day.plusDays(i).toString()).id);
        }
        assertTrue(seen.size() >= Math.min(5, puzzles.puzzles().size()), "the days differ: " + seen);

        puzzles.useClock(() -> day);
        CareerPuzzles.CareerDailyPuzzle view = puzzles.dailyView("Ana");
        assertEquals("2026-10-13", view.date);
        assertEquals(today.id, view.puzzle.id);
        assertEquals(0, view.attempts);
        assertFalse(view.solved);
        puzzles.attemptDaily("Ana");
        assertEquals(1, puzzles.dailyView("Ana").attempts);
        assertNull(puzzles.dailyView("Ana").puzzle.hint, "no hint on the first tries");
    }

    @Test
    public void puzzlesFromFilesAreChecked() throws Exception {
        CareerSetup.Puzzle puzzle = new CareerSetup.Puzzle();
        puzzle.objective = "win";
        puzzle.you = new CareerSetup.CareerPuzzleSide();
        puzzle.you.hand = new ArrayList<>(Arrays.asList("Lightning Bolt"));
        puzzle.you.battlefield = new ArrayList<>(Arrays.asList("Mountain", "Mountain|tapped"));
        puzzle.opponent = new CareerSetup.CareerPuzzleSide();
        puzzle.opponent.life = 3;
        CareerSetup.Puzzle checked = CareerPuzzles.get().custom("Ana", puzzle);
        assertTrue(checked.id.startsWith("p-"));
        assertEquals("Your puzzle", checked.name);

        puzzle.you.hand.add("Not A Real Card Name");
        assertThrows(CareerService.CareerException.class, () -> CareerPuzzles.get().custom("Ana", puzzle));
        puzzle.you.hand.remove(1);
        puzzle.opponent.life = 0;
        assertThrows(CareerService.CareerException.class, () -> CareerPuzzles.get().custom("Ana", puzzle));
    }

    @Test
    public void aYearOfChallengesComesFromTheMetaLibrary() throws Exception {
        List<CareerChallenges.Challenge> year = CareerChallenges.generate(2027);
        assertEquals(52, year.size(), "2027 has 52 ISO weeks");
        assertEquals(53, CareerChallenges.generate(2026).size(), "2026 has 53");
        Set<String> ids = new HashSet<>();
        Set<String> rules = new HashSet<>();
        for (CareerChallenges.Challenge challenge : year) {
            assertTrue(ids.add(challenge.id));
            rules.add(challenge.rule);
            assertNotNull(MetaLibrary.get().deck(challenge.playerDeck), challenge.playerDeck);
            assertNotNull(MetaLibrary.get().deck(challenge.opponent.deck), challenge.opponent.deck);
            assertFalse(challenge.playerDeck.equals(challenge.opponent.deck), "different decks");
        }
        assertTrue(rules.containsAll(Arrays.asList("turns", "life", "no-creatures")), rules.toString());
        assertEquals(new com.google.gson.Gson().toJson(year), new com.google.gson.Gson().toJson(CareerChallenges.generate(2027)),
                "the same year everywhere");
    }

    private CareerContent.Opponent opponent(int tier) {
        return CareerContent.get().opponents().stream().filter(o -> o.tier == tier && o.rival == null && !o.raid).findFirst().get();
    }

    private void duel(CareerContent.Opponent opponent, boolean won) throws Exception {
        UUID table = UUID.randomUUID();
        service.register(table, "Ana", opponent.id, opponent.tier, CareerLadder.setup("Ana", opponent));
        service.matchEnded(table, null, won, 7, CareerTally.forTest(UUID.randomUUID()));
    }

    @Test
    public void whoBeatsYouBecomesYourNemesisUntilYouBeatThemBack() throws Exception {
        CareerContent.Opponent wren = opponent(1);
        assertNull(CareerLadder.nemesis("Ana"));
        int lifeBefore = lifeBonus(CareerLadder.setup("Ana", wren));
        duel(wren, false);
        assertEquals(wren.id, CareerLadder.nemesis("Ana").opponentId);
        assertEquals(lifeBefore + CareerLadder.NEMESIS_LIFE, lifeBonus(CareerLadder.setup("Ana", wren)), "the nemesis has an edge");
        int coins = store.profile("Ana").coins;
        duel(wren, true);
        assertNull(CareerLadder.nemesis("Ana"));
        assertTrue(store.profile("Ana").coins - coins >= CareerLadder.NEMESIS_BOUNTY, "the bounty is paid");
        assertEquals(1, (int) store.stats("Ana").get("nemeses_beaten"));
        assertEquals(2, (int) store.stats("Ana").get("met_" + wren.id), "both meetings counted");
    }

    private static int lifeBonus(CareerSetup setup) {
        CareerSetup.CareerTwists twists = setup == null ? null : setup.twists();
        return twists == null ? 0 : twists.lifeBonus;
    }

    @Test
    public void theRankBadgeAndListsFollowTheLadder() throws Exception {
        assertNull(CareerLadder.rank("Ana"));
        CareerContent content = CareerContent.get();
        for (int i = 0; i < CareerRules.WINS_TO_UNLOCK_NEXT_TIER; i++) {
            duel(opponent(1), true);
        }
        CareerLadder.CareerRank rank = CareerLadder.rank("Ana");
        assertNotNull(rank);
        assertEquals(1, rank.tier);
        assertNotNull(rank.badge);

        // the lists take turns by seed: every list of an opponent comes up
        for (CareerContent.Opponent opponent : content.opponents()) {
            if (opponent.decks == null || opponent.decks.isEmpty()) {
                continue;
            }
            Set<String> files = new HashSet<>();
            for (long seed = 0; seed < 40; seed++) {
                files.add(CareerContent.deckFor(opponent, seed));
            }
            assertTrue(files.size() >= 2, opponent.id + " rotates its lists");
        }
        // pips go 1 to 5
        for (CareerContent.Opponent opponent : content.opponents()) {
            int pips = CareerLadder.pips(opponent);
            assertTrue(pips >= 1 && pips <= 5, opponent.id);
        }
    }

    @Test
    public void theRivalArcOpensAChapterAtATime() throws Exception {
        List<CareerContent.Opponent> rivals = new ArrayList<>();
        for (CareerContent.Opponent opponent : CareerContent.get().opponents()) {
            if (opponent.rival != null) {
                rivals.add(opponent);
            }
        }
        rivals.sort(java.util.Comparator.comparingInt(o -> o.rival.chapter));
        assertTrue(rivals.size() >= 3, "a rival arc of three or four duels");
        java.util.Map<String, Integer> wins = new java.util.HashMap<>();
        assertTrue(CareerLadder.rivalOpen(rivals.get(0), wins));
        assertFalse(CareerLadder.rivalOpen(rivals.get(1), wins), "chapter two waits for chapter one");
        wins.put(rivals.get(0).id, 1);
        assertTrue(CareerLadder.rivalOpen(rivals.get(1), wins));
        for (CareerContent.Opponent rival : rivals) {
            assertNotNull(rival.rival.before);
            assertNotNull(rival.rival.after);
        }
    }

    @Test
    public void adaptiveSkillSettlesWhereThePlayerWinsJustOverHalf() throws Exception {
        assertEquals((int) AdaptiveSkill.START, AdaptiveSkill.skillFor("Ana", "career"));
        // a player who wins whenever the AI is below skill 6: the rating settles around 6
        java.util.Random random = new java.util.Random(5);
        int wins = 0;
        int games = 400;
        for (int i = 0; i < games; i++) {
            int skill = AdaptiveSkill.skillFor("Ana", "career");
            double chance = skill < 6 ? 0.8 : skill == 6 ? 0.55 : 0.25;
            boolean won = random.nextDouble() < chance;
            wins += won ? 1 : 0;
            AdaptiveSkill.record("Ana", "career", won);
        }
        double rate = wins / (double) games;
        assertTrue(rate > 0.45 && rate < 0.65, "wins about 55%: " + rate);
        int skill = AdaptiveSkill.skillFor("Ana", "career");
        assertTrue(skill >= 5 && skill <= 7, "settles near the player's level: " + skill);
        AdaptiveSkill.AdaptiveView view = AdaptiveSkill.view("Ana", "career");
        assertEquals(games, view.games);
        assertEquals(AdaptiveSkill.RECENT, view.recentGames);
        assertFalse(view.sentence.matches(".*\\d+ ?/ ?10.*"), "the rating isn't shown as a number: " + view.sentence);
        assertEquals((int) AdaptiveSkill.START, AdaptiveSkill.skillFor("Ana", "commander"), "each format has its own");

        UUID table = UUID.randomUUID();
        AdaptiveSkill.mark(table, "Ana", "commander");
        AdaptiveSkill.ended(table, "Bob", true);
        assertEquals(0, AdaptiveSkill.view("Ana", "commander").games, "only the marked player's result counts");
    }

    private static DeckCardLists deck() {
        DeckCardLists deck = new DeckCardLists();
        deck.setName("Mono Red");
        deck.getCards().add(new DeckCardInfo("Mountain", "1", "M10", 20));
        deck.getCards().add(new DeckCardInfo("Lightning Bolt", "146", "M10", 40));
        return deck;
    }

    @Test
    public void ghostDecksReportToTheirOwner() throws Exception {
        GhostDecks ghosts = GhostDecks.get();
        GhostDecks.GhostDeck shared = ghosts.share("Ana", null, deck(), 1000L);
        assertEquals(GhostDecks.CODE_LENGTH, shared.code.length());
        assertEquals("Mono Red", shared.name);
        assertEquals(1, ghosts.mine("Ana").size());

        GhostDecks.GhostOpponent ghost = ghosts.opponent(shared.code.toLowerCase());
        assertEquals("Ana", ghost.owner);
        assertEquals(2, ghost.deck.getCards().size());

        UUID table = UUID.randomUUID();
        ghosts.mark(table, "Bob", shared.code);
        assertTrue(ghosts.marked(table));
        ghosts.ended(table, "Bob", false, 6);
        assertFalse(ghosts.marked(table));
        GhostDecks.GhostDeck after = ghosts.mine("Ana").get(0);
        assertEquals(1, after.played);
        assertEquals(1, after.won, "the deck won");
        List<GhostDecks.InboxNote> inbox = ghosts.inbox("Ana", 10);
        assertEquals(1, inbox.size());
        assertTrue(inbox.get(0).text.contains("Bob") && inbox.get(0).text.contains("turn 6"), inbox.get(0).text);
        assertFalse(inbox.get(0).seen);
        soloStore.markInboxSeen("Ana");
        assertTrue(ghosts.inbox("Ana", 10).get(0).seen);

        // playing your own ghost moves its record but sends no note
        UUID own = UUID.randomUUID();
        ghosts.mark(own, "Ana", shared.code);
        ghosts.ended(own, "Ana", true, 9);
        assertEquals(1, ghosts.inbox("Ana", 10).size());

        assertThrows(IllegalArgumentException.class, () -> ghosts.opponent("ZZZZZZ"));
        assertThrows(IllegalArgumentException.class, () -> ghosts.share("Ana", "empty", new DeckCardLists(), 1L));
        assertFalse(ghosts.unshare("Bob", shared.code), "only the owner unshares");
        assertTrue(ghosts.unshare("Ana", shared.code));
        assertTrue(ghosts.mine("Ana").isEmpty());
    }

    @Test
    public void limitedShapesHaveTheirOwnRoads() {
        assertEquals(3, CareerLimited.shapes().size());
        assertTrue(CareerLimited.over(CareerLimited.ARENA, 7, 0));
        assertFalse(CareerLimited.over(CareerLimited.ARENA, 6, 2));
        assertTrue(CareerLimited.over(CareerLimited.ARENA, 2, 3));
        assertFalse(CareerLimited.over(CareerLimited.LEAGUE, 0, 4), "a league plays all five");
        assertTrue(CareerLimited.over(CareerLimited.LEAGUE, 1, 4));
        assertEquals(9, CareerLimited.maxGames(CareerLimited.ARENA));
        for (String kind : new String[]{"sealed", "draft"}) {
            for (CareerLimited.LimitedShape shape : CareerLimited.shapes()) {
                int top = shape.games > 0 ? shape.games : shape.wins;
                for (int wins = 1; wins <= top; wins++) {
                    assertTrue(CareerLimited.reward(kind, shape.id, wins) > CareerLimited.reward(kind, shape.id, wins - 1),
                            kind + " " + shape.id + " pays more for each win");
                }
            }
        }
        assertTrue(CareerLimited.reward("sealed", CareerLimited.ARENA, 7) > CareerLimited.reward("sealed", CareerLimited.CLASSIC, 3),
                "the longer road pays more at the top");
    }

    @Test
    public void aHordeIsSurvivedWithAMetaDeck() throws Exception {
        CareerHordes hordes = CareerHordes.get();
        List<CareerHordes.CareerHorde> list = hordes.hordes("Ana");
        assertTrue(list.size() >= 3);
        CareerHordes.CareerHorde first = list.get(0);
        assertFalse(first.beaten);
        MetaLibrary.MetaDeck deck = MetaLibrary.get().constructed().get(0);
        assertThrows(CareerService.CareerException.class, () -> hordes.attempt("Ana", first.id, "nope.dck"));
        assertThrows(CareerService.CareerException.class, () -> hordes.attempt("Ana", "no-such-horde", deck.file));
        CareerHordes.Plan plan = hordes.attempt("Ana", first.id, deck.file);
        assertNotNull(plan.setup.horde());
        assertEquals(first.survive, plan.setup.horde().survive);
        assertFalse(plan.hordeDeck.getCards().isEmpty());
        int coins = store.profile("Ana").coins;
        UUID table = UUID.randomUUID();
        service.register(table, "Ana", "horde", first.id, plan.setup);
        service.matchEnded(table, null, true, 9, CareerTally.forTest(UUID.randomUUID()));
        assertTrue(store.profile("Ana").coins > coins, "the first survival pays");
        CareerHordes.CareerHorde after = hordes.hordes("Ana").get(0);
        assertTrue(after.beaten);
        assertEquals(1, after.attempts);
    }
}
