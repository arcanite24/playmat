package mage.server.career;

import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.server.ladder.BotRoster;
import mage.server.ladder.DeckLegality;
import mage.server.ladder.LadderRatings;
import mage.server.ladder.LadderStore;
import mage.server.ladder.RankedService;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.UUID;
import java.util.stream.Collectors;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

/** The Career League (#433): starting a format, its packs, League decks, and what climbing its queue pays. */
public class CareerLeagueTest {

    private static final long NOW = 1_800_000_000_000L;

    private Path file;
    private CareerStore store;
    private LadderStore ladder;
    private RankedService ranked;
    private final CareerLeague league = CareerLeague.get();

    @BeforeAll
    static void deckTypes() {
        // the server registers these from config.xml at start
        mage.cards.decks.DeckValidatorFactory.instance.addDeckType("Constructed - Standard", mage.deck.Standard.class);
        mage.cards.decks.DeckValidatorFactory.instance.addDeckType("Constructed - Pioneer", mage.deck.Pioneer.class);
    }

    @BeforeEach
    void open() throws Exception {
        file = Files.createTempFile("league-test", ".db");
        store = new CareerStore("jdbc:sqlite:" + file);
        CareerStore.use(store);
        ladder = new LadderStore("jdbc:sqlite::memory:");
        ranked = RankedService.forTests(ladder, BotRoster.get());
        RankedService.use(ranked);
    }

    @AfterEach
    void close() throws Exception {
        RankedService.use(null);
        CareerStore.use(null);
        store.close();
        ladder.close();
        Files.deleteIfExists(file);
    }

    private List<String> starterIds(String format) {
        return league.starters(format).stream().map(s -> s.id).collect(Collectors.toList());
    }

    @Test
    void everyStarterIsLegalInItsFormat() throws Exception {
        for (String format : league.formats()) {
            List<CareerLeague.LeagueStarter> starters = league.starters(format);
            assertTrue(starters.size() >= CareerLeague.STARTERS, format + " has starters to pick two of");
            for (CareerLeague.LeagueStarter starter : starters) {
                DeckCardLists deck = CareerCampaigns.get().deck(starter.id);
                assertEquals(Collections.emptyList(), DeckLegality.problems(RankedService.deckType(format), deck),
                        starter.id + " must be legal in " + format);
                assertTrue(starter.cards >= 60, starter.id + " is a whole deck");
            }
        }
    }

    @Test
    void startingAddsTheStartersPacksAndFirstLeagueWildcards() throws Exception {
        store.createProfile("Ana", "test", CareerRules.START_COINS, 1L);
        List<String> ids = starterIds("standard").subList(0, 2);
        CareerLeague.LeagueStart start = league.start("Ana", "standard", ids);
        assertEquals(2, start.decks.size());
        for (CareerLeague.LeagueDeck deck : start.decks) {
            assertEquals(Collections.emptyList(), deck.missing, deck.name + " is playable as given");
            assertEquals(Collections.emptyList(), deck.illegal, deck.name + " is legal");
            assertNotNull(league.playable("ana", "standard", deck.id));
        }
        assertEquals(6, store.leagueTokens("ana", "standard"));
        CareerProfile profile = store.profile("ana");
        assertEquals(4, profile.wildcards.common);
        assertEquals(3, profile.wildcards.uncommon);
        assertEquals(1, profile.wildcards.rare);
        // never past a playset, and no currency for the overlap
        for (CareerCard card : store.collection("ana")) {
            assertTrue(card.count <= CareerRules.PLAYSET, card.name + " is capped at a playset");
        }
        assertEquals(CareerRules.START_COINS, profile.coins, "the overlap turns into nothing");

        CareerService.CareerException again = assertThrows(CareerService.CareerException.class, () -> league.start("ana", "standard", ids));
        assertTrue(again.getMessage().contains("already"));

        league.start("ana", "pioneer", starterIds("pioneer").subList(0, 2));
        CareerProfile after = store.profile("ana");
        assertEquals(4, after.wildcards.common, "only the first League gives wildcards");
        assertEquals(6, store.leagueTokens("ana", "pioneer"));
        assertFalse(league.state("ana").first);
    }

    @Test
    void aStartNeedsTwoOfTheFormatsStarters() {
        List<String> ids = starterIds("standard");
        assertThrows(CareerService.CareerException.class, () -> league.start("ana", "standard", ids.subList(0, 1)));
        assertThrows(CareerService.CareerException.class, () -> league.start("ana", "standard", Arrays.asList(ids.get(0), "pioneer/starter.dck")));
        assertThrows(CareerService.CareerException.class, () -> league.start("ana", "modern", ids.subList(0, 2)));
    }

    @Test
    void packsComeFromTheFormatsSetsAndSpendTheFormatsTokensFirst() throws Exception {
        List<CareerLeague.LeagueSet> sets = league.sets("standard");
        assertFalse(sets.isEmpty());
        List<String> legal = CareerLeague.legalSets("standard");
        for (CareerLeague.LeagueSet set : sets) {
            assertTrue(legal.contains(set.setCode), set.setCode + " is a Standard set");
        }
        assertTrue(league.sets("pioneer").stream().anyMatch(s -> CareerLeague.MIXED.equals(s.setCode)), "Pioneer has the mixed pack");
        assertFalse(sets.stream().anyMatch(s -> CareerLeague.MIXED.equals(s.setCode)), "Standard picks a set");

        assertThrows(CareerService.CareerException.class, () -> league.openPack("ana", "standard", sets.get(0).setCode), "start first");
        league.start("ana", "standard", starterIds("standard").subList(0, 2));
        CareerLeague.LeaguePack pack = league.openPack("ana", "standard", sets.get(0).setCode);
        assertEquals("league", pack.paidWith);
        assertEquals(5, pack.tokens);
        assertFalse(pack.cards.isEmpty());
        assertThrows(CareerService.CareerException.class, () -> league.openPack("ana", "standard", "LEA"), "not a Standard set");

        league.start("ana", "pioneer", starterIds("pioneer").subList(0, 2));
        CareerLeague.LeaguePack mixed = league.openPack("ana", "pioneer", CareerLeague.MIXED);
        assertTrue(CareerLeague.legalSets("pioneer").contains(mixed.setCode), "the mixed pack is a Pioneer set's");
    }

    @Test
    void aDeckWithCardsTheAccountLacksCantBePlayed() throws Exception {
        CareerLeague.LeagueStart start = league.start("ana", "standard", starterIds("standard").subList(0, 2));
        DeckCardLists deck = start.decks.get(0).deck;
        DeckCardInfo extra = deck.getCards().stream().filter(c -> !isBasic(c.getCardName())).findFirst().orElseThrow(IllegalStateException::new);
        DeckCardLists bigger = new DeckCardLists();
        bigger.getCards().addAll(deck.getCards());
        bigger.getCards().add(new DeckCardInfo(extra.getCardName(), extra.getCardNumber(), extra.getSetCode(), CareerRules.PLAYSET));
        CareerLeague.LeagueDeck saved = league.saveDeck("ana", null, "  ", "standard", bigger);
        assertEquals("Standard deck", saved.name);
        assertFalse(saved.missing.isEmpty(), "more copies than owned");
        assertThrows(CareerService.CareerException.class, () -> league.playable("ana", "standard", saved.id));
        assertThrows(CareerService.CareerException.class, () -> league.playable("ana", "pioneer", start.decks.get(1).id), "a Standard deck");
        assertTrue(league.deleteDeck("ana", saved.id));
        assertThrows(CareerService.CareerException.class, () -> league.deck("ana", saved.id));
    }

    private static boolean isBasic(String name) {
        return Arrays.asList("Plains", "Island", "Swamp", "Mountain", "Forest").contains(name);
    }

    @Test
    void leagueGamesPayTiersOnceRanksInWildcardsAndNoCapsules() throws Exception {
        league.start("ana", "standard", starterIds("standard").subList(0, 2));
        assertEquals(LadderRatings.LEAGUE_START, ranked.ratings().player("ana", "standard", RankedService.LEAGUE, NOW).rating, 0.001);
        int tokens = store.leagueTokens("ana", "standard");
        int capsules = store.profile("ana").capsuleTokens;
        // win up to Bronze 3 (a tier up), lose back to Bronze 4, win up again: the tier pays once
        int packs = 0;
        int uncommon = 0;
        for (int i = 0; i < 12; i++) {
            RankedService.Result result = game(i % 6 < 4 ? 1 : 0);
            packs += result.paid == null ? 0 : result.paid.packs;
            uncommon += result.paid == null ? 0 : result.paid.wildcards.getOrDefault(CareerRules.UNCOMMON, 0);
        }
        LadderStore.Standing standing = ladder.standing("ana", "standard", RankedService.LEAGUE, RankedService.SEASON);
        int tiersReached = standing.peak / mage.server.ladder.LadderRank.PIPS;
        assertEquals(tiersReached, packs, "one pack per tier reached, never twice");
        assertEquals(tokens + packs, store.leagueTokens("ana", "standard"));
        assertEquals(standing.won / CareerLeague.WINS_PER_WILDCARD, uncommon, "an uncommon wildcard per five wins");
        assertEquals(capsules, store.profile("ana").capsuleTokens, "the League pays no capsule tokens");
    }

    @Test
    void reachingARankPaysItsWildcardMythicAtGold() throws Exception {
        RankedRewards.Paid bronze = CareerLeague.pay("ana", "standard", Collections.emptyList(), Collections.singletonList("bronze"), 1, true, NOW);
        assertTrue(bronze.wildcards.isEmpty(), "Bronze comes with the first game");
        RankedRewards.Paid silver = CareerLeague.pay("ana", "standard", Collections.emptyList(), Collections.singletonList("silver"), 2, true, NOW);
        assertEquals(1, (int) silver.wildcards.get(CareerRules.RARE));
        RankedRewards.Paid gold = CareerLeague.pay("ana", "standard", Collections.emptyList(), Collections.singletonList("gold"), 3, true, NOW);
        assertEquals(1, (int) gold.wildcards.get(CareerRules.MYTHIC));
        assertEquals(1, store.profile("ana").wildcards.mythic);
    }

    @Test
    void tiersGainedCountFromThePeak() {
        assertEquals(Collections.emptyList(), CareerLeague.tiersGained(0, 3, true));
        assertEquals(Collections.singletonList(1), CareerLeague.tiersGained(0, 4, true));
        assertEquals(Collections.emptyList(), CareerLeague.tiersGained(5, 7, false));
        assertEquals(Arrays.asList(2, 3), CareerLeague.tiersGained(7, 12, false));
    }

    @Test
    // RustHost refuses Career tables; a League table is a ranked one, started like Ranked's (not checked end to end here)
    void aLeagueTableIsRankedNotACareerTable() throws Exception {
        league.start("ana", "standard", starterIds("standard").subList(0, 2));
        BotRoster.Bot bot = ranked.pick("ana", "standard", RankedService.LEAGUE, NOW);
        UUID table = UUID.randomUUID();
        ranked.register(table, "ana", "standard", RankedService.LEAGUE, bot, "deck", NOW);
        assertTrue(RankedService.isRanked(table));
        assertFalse(CareerService.get().isCareerTable(table), "Career tables don't qualify for the Rust host; League ones must");
        ranked.forget(table, NOW);
    }

    @Test
    void craftMissingCraftsEverythingOrNothing() throws Exception {
        CareerLeague.LeagueStart start = league.start("ana", "standard", starterIds("standard").subList(0, 2));
        // a starter's list plus four copies of a Standard rare nobody owns
        String rareName = null;
        DeckCardLists deck = new DeckCardLists();
        deck.getCards().addAll(start.decks.get(0).deck.getCards());
        for (CareerLeague.LeagueSet set : league.sets("standard")) {
            mage.cards.repository.CardInfo rare = null;
            for (mage.cards.repository.CardInfo c : mage.cards.repository.CardRepository.instance.findCards(
                    new mage.cards.repository.CardCriteria().setCodes(set.setCode).rarities(mage.constants.Rarity.RARE))) {
                if (store.owned("ana", c.getName()) == 0) {
                    rare = c;
                    break;
                }
            }
            if (rare != null) {
                rareName = rare.getName();
                deck.getCards().add(new DeckCardInfo(rare.getName(), rare.getCardNumber(), rare.getSetCode(), 4));
                break;
            }
        }
        assertNotNull(rareName, "a Standard rare to craft");
        CareerLeague.LeagueDeck saved = league.saveDeck("ana", null, "Plus rares", "standard", deck);
        assertEquals(4, saved.craft.copies);
        assertEquals(4, saved.craft.rare);
        assertFalse(saved.craft.affordable, "one rare wildcard from the start");
        int rareBefore = store.profile("ana").wildcards.rare;
        assertThrows(CareerService.CareerException.class, () -> league.craftMissing("ana", saved.id));
        assertEquals(rareBefore, store.profile("ana").wildcards.rare, "nothing is spent when it can't all be crafted");
        assertEquals(0, store.owned("ana", rareName));

        store.addWildcard("ana", CareerRules.RARE, 3);
        CareerLeague.LeagueDeck crafted = league.craftMissing("ana", saved.id);
        assertTrue(crafted.missing.isEmpty());
        assertEquals(0, crafted.craft.copies);
        assertEquals(4, store.owned("ana", rareName));
        assertEquals(0, store.profile("ana").wildcards.rare);
        assertThrows(CareerService.CareerException.class, () -> league.craftMissing("ana", saved.id), "nothing left to craft");
    }

    @Test
    void commonsUseTheirWildcardsThenCoins() {
        CareerProfile profile = new CareerProfile();
        profile.coins = 1000;
        profile.wildcards = new CareerProfile.Wildcards(1, 0, 0, 0);
        java.util.Map<CareerCard, Integer> lacking = new java.util.LinkedHashMap<>();
        lacking.put(new CareerCard("A", "TST", "1", CareerRules.COMMON), 3);
        lacking.put(new CareerCard("B", "TST", "2", CareerRules.UNCOMMON), 1);
        CareerLeague.LeagueCraft plan = CareerLeague.planForTests(lacking, profile);
        assertEquals(4, plan.copies);
        assertEquals(1, plan.common);
        assertEquals(0, plan.uncommon);
        assertEquals(2 * CareerRules.craftCoins(CareerRules.COMMON) + CareerRules.craftCoins(CareerRules.UNCOMMON), plan.coins);
        assertTrue(plan.affordable);
    }

    @Test
    void deletingTheCareerDropsTheLeagues() throws Exception {
        league.start("ana", "standard", starterIds("standard").subList(0, 2));
        assertTrue(CareerService.get().delete("ana"));
        assertTrue(store.leagues("ana").isEmpty());
        assertTrue(store.leagueDecks("ana", null).isEmpty());
        assertEquals(0, store.leagueTokens("ana", "standard"));
    }

    @Test
    void theExportHasTheLeagues() throws Exception {
        league.start("ana", "standard", starterIds("standard").subList(0, 2));
        java.util.Map<String, Object> out = league.export("ana");
        assertTrue(((java.util.Map<?, ?>) out.get("formats")).containsKey("standard"));
        assertEquals(2, ((List<?>) out.get("decks")).size());
    }

    private RankedService.Result game(double score) throws Exception {
        BotRoster.Bot bot = ranked.pick("ana", "standard", RankedService.LEAGUE, NOW);
        UUID table = UUID.randomUUID();
        ranked.register(table, "ana", "standard", RankedService.LEAGUE, bot, "deck", NOW);
        return ranked.matchEnded(table, score, NOW);
    }
}
