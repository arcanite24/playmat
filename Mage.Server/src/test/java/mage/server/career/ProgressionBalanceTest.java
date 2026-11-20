package mage.server.career;

import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.server.ladder.BotRoster;
import mage.server.ladder.Glicko2;
import mage.server.ladder.LadderRank;
import mage.server.ladder.LadderStore;
import mage.server.ladder.RankedService;
import mage.util.RandomUtil;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import java.time.ZoneOffset;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicReference;

import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * The balancing sheet (B292): a casual (2 games a day), a regular (6) and a heavy player (20) play 90 days of mixed
 * games through the real progression code (rewards.json, quests, achievements, masteries, the weekly goal), and the
 * level they reach and the coins they earn must stay inside the targets below. Measured 2026-10-21: level after 90
 * days 22 (casual), 45 (regular), 67 (heavy). Capsule tokens (M29, before any bought with coins) after 90 days: 78, 142
 * and 168, so a regular player empties a 50-capsule machine in about four and a half weeks. Run it with
 * -Dxmage.progressionReport=true to print the sheet.
 */
public class ProgressionBalanceTest {

    private static final int DAYS = 90;

    @BeforeAll
    static void deckTypes() {
        // the server registers these from config.xml at start
        mage.cards.decks.DeckValidatorFactory.instance.addDeckType("Constructed - Standard", mage.deck.Standard.class);
        mage.cards.decks.DeckValidatorFactory.instance.addDeckType("Constructed - Pioneer", mage.deck.Pioneer.class);
    }

    private Path file;
    private CareerStore store;
    private final AtomicReference<LocalDate> day = new AtomicReference<>(LocalDate.of(2027, 5, 3));

    @BeforeEach
    public void setUp() throws Exception {
        file = Files.createTempFile("progression-balance", ".db");
        store = new CareerStore("jdbc:sqlite:" + file);
        CareerStore.use(store);
        CareerProgress.get().useClock(day::get, new Random(11));
    }

    @AfterEach
    public void tearDown() throws Exception {
        store.close();
        CareerStore.use(null);
        CareerProgress.get().useClock(() -> LocalDate.now(java.time.ZoneOffset.UTC), new Random());
        Files.deleteIfExists(file);
    }

    /** what one simulated player has after each checkpoint */
    private static final class Sheet {
        final int[] level = new int[3];
        final int[] coins = new int[3];
        final int[] tokens = new int[3];
        int packs;
    }

    private Sheet play(String user, int gamesPerDay, Random random) throws Exception {
        String[] kinds = {CareerRewards.AI, CareerRewards.AI, CareerRewards.PERSON, CareerRewards.POD, CareerRewards.EVENT};
        String[] formats = {"standard", "commander", "limited", "modern", "pauper"};
        String[] colors = {"W", "U", "B", "R", "G"};
        Sheet sheet = new Sheet();
        LocalDate start = day.get();
        int checkpoint = 0;
        int[] checkpoints = {7, 30, DAYS};
        for (int d = 1; d <= DAYS; d++) {
            day.set(start.plusDays(d));
            for (int g = 0; g < gamesPerDay; g++) {
                String kind = gamesPerDay <= 2 ? CareerRewards.AI : kinds[random.nextInt(kinds.length)];
                int players = CareerRewards.POD.equals(kind) ? 4 : 2;
                CareerService.FinishedGame game = new CareerService.FinishedGame(UUID.randomUUID(), kind, players, 6 + random.nextInt(6));
                game.format = formats[random.nextInt(formats.length)];
                game.aiSkill = CareerRewards.AI.equals(kind) ? 5 : 0;
                CareerTally tally = CareerTally.forTest(UUID.randomUUID());
                tally.add("spells", 6 + random.nextInt(8));
                tally.add("creature_spells", 3 + random.nextInt(4));
                tally.add("attacks", 4 + random.nextInt(8));
                tally.add("damage_to_opponents", 10 + random.nextInt(15));
                tally.add("lands_played", 4 + random.nextInt(4));
                tally.add("cards_drawn", 6 + random.nextInt(6));
                CareerService.Seat seat = new CareerService.Seat(user, random.nextDouble() < 0.5, tally);
                seat.colors.add(colors[random.nextInt(colors.length)]);
                game.seats.add(seat);
                // people are a different opponent each game, so the pair cap never binds here
                if (players > 1 && !CareerRewards.AI.equals(kind)) {
                    game.seats.add(new CareerService.Seat("opp" + random.nextInt(1_000_000), !seat.won, CareerTally.forTest(UUID.randomUUID())));
                }
                CareerService.get().gameEnded(game);
            }
            if (d == checkpoints[checkpoint]) {
                CareerProfile profile = store.profile(user);
                sheet.level[checkpoint] = profile.level;
                sheet.coins[checkpoint] = profile.coins;
                sheet.tokens[checkpoint] = profile.capsuleTokens;
                sheet.packs = profile.packTokens;
                checkpoint++;
            }
        }
        return sheet;
    }

    @Test
    public void casualRegularAndHeavyPlayersProgressAtTheirPace() throws Exception {
        Random random = new Random(5);
        Sheet casual = play("casual", 2, random);
        Sheet regular = play("regular", 6, random);
        Sheet heavy = play("heavy", 20, random);
        if (Boolean.getBoolean("xmage.progressionReport")) {
            StringBuilder report = new StringBuilder("player   games/day  level d7  d30  d90  coins d7  d30  d90  packs d90  tokens d7  d30  d90\n");
            for (Object[] row : new Object[][]{{"casual", 2, casual}, {"regular", 6, regular}, {"heavy", 20, heavy}}) {
                Sheet sheet = (Sheet) row[2];
                report.append(String.format("%-8s %9d  %8d %4d %4d  %8d %4d %4d  %9d  %9d %4d %4d%n", row[0], row[1],
                        sheet.level[0], sheet.level[1], sheet.level[2], sheet.coins[0], sheet.coins[1], sheet.coins[2], sheet.packs,
                        sheet.tokens[0], sheet.tokens[1], sheet.tokens[2]));
            }
            System.out.println(report);
        }
        // a casual player sees a level most weeks early on, and is still climbing after three months
        assertTrue(casual.level[0] >= 3, "casual level after a week: " + Arrays.toString(casual.level));
        assertTrue(casual.level[2] >= 15 && casual.level[2] <= 30, "casual level after 90 days: " + Arrays.toString(casual.level));
        // a regular player is well into the track after three months, far from its top
        assertTrue(regular.level[2] >= 35 && regular.level[2] <= 55, "regular level after 90 days: " + Arrays.toString(regular.level));
        // a heavy player doesn't finish the track in a season
        assertTrue(heavy.level[2] >= 50 && heavy.level[2] <= 85, "heavy level after 90 days: " + Arrays.toString(heavy.level));
        // a machine is 50 capsules: a regular player fills one in a month or so without spending coins, a casual player
        // within the season, and heavy play doesn't turn every wave into a weekend
        assertTrue(regular.tokens[1] >= 30 && regular.tokens[1] <= 70, "regular tokens after 30 days: " + Arrays.toString(regular.tokens));
        assertTrue(casual.tokens[2] >= 50, "casual tokens after 90 days: " + Arrays.toString(casual.tokens));
        assertTrue(heavy.tokens[1] < 100, "heavy tokens after 30 days: " + Arrays.toString(heavy.tokens));
        // the daily wins ladder caps coins: heavy play earns more, but not ten times more
        assertTrue(heavy.coins[2] < regular.coins[2] * 4, "coins after 90 days: regular " + regular.coins[2] + ", heavy " + heavy.coins[2]);
    }

    // ---- the League's pace (#433)

    /** the competitive Standard decks a League player works towards: the ladder's Standard meta lists */
    private static final String[] META = {"standard/meta-azorius-control.dck", "standard/meta-boros-dragons.dck",
            "standard/meta-dimir-midrange.dck", "standard/meta-izzet-spellementals.dck", "standard/meta-jund-sacrifice.dck",
            "standard/meta-mono-green-landfall.dck", "standard/meta-rakdos-lizards.dck"};
    /** a League run gives up here */
    private static final int LEAGUE_DAYS = 90;
    /** runs per deck, each with its own draws */
    private static final int SEEDS = 3;

    /** how one League run went */
    private static final class Run {
        /** the day the deck was crafted whole; -1 when it never was */
        int finished = -1;
        int games;
        int won;
        int packs;
        int peak;
        int coinPacks;
        /** rare and mythic copies the deck needs, at the start */
        int rares;
    }

    /**
     * One player starts the Standard League with two starters and plays `gamesPerDay` League games a day against the
     * bots the real matchmaking picks, winning as a player of rating `strength` would (Glicko's expected score against
     * the bot's League rating). Every game goes through the real code: the ladder's result and League rewards
     * (RankedService.matchEnded), then the Career's (CareerService.gameEnded). Each evening the player opens every
     * pack they can: League tokens, then Career pack tokens, then coins, keeping the coins the deck's commons and
     * uncommons will cost to craft. Each pack is of the Standard set holding the most rare and mythic copies the deck
     * still lacks. Then they try Craft missing, which crafts all of it or nothing.
     */
    private Run league(String user, String target, int gamesPerDay, double strength, long seed) throws Exception {
        LadderStore ladder = new LadderStore("jdbc:sqlite::memory:");
        RankedService ranked = RankedService.forTests(ladder, BotRoster.get());
        RankedService.use(ranked);
        Random before = RandomUtil.install(new Random(seed));
        Random random = new Random(seed);
        try {
            CareerLeague league = CareerLeague.get();
            LocalDate start = day.get();
            store.createProfile(user, "test", CareerRules.START_COINS, start.atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli());
            List<String> starters = new ArrayList<>();
            for (CareerLeague.LeagueStarter starter : league.starters("standard")) {
                starters.add(starter.id);
            }
            league.start(user, "standard", starters.subList(0, CareerLeague.STARTERS));
            DeckCardLists deck = CareerCampaigns.get().deck(target);
            String deckId = league.saveDeck(user, null, "Target", "standard", deck).id;
            Map<String, Set<String>> setsOf = setsOf(deck, league.sets("standard"));
            Run run = new Run();
            CareerLeague.LeagueCraft first = league.deck(user, deckId).craft;
            run.rares = first.rare + first.mythic;
            for (int d = 1; d <= LEAGUE_DAYS && run.finished < 0; d++) {
                day.set(start.plusDays(d));
                long now = start.plusDays(d).atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli() + 3_600_000L;
                for (int g = 0; g < gamesPerDay; g++) {
                    long at = now + g * 600_000L;
                    BotRoster.Bot bot = ranked.pick(user, "standard", RankedService.LEAGUE, at);
                    double botRating = ranked.ratings().bot(bot, RankedService.LEAGUE).rating;
                    boolean won = random.nextDouble() < 1 / (1 + Math.pow(10, (botRating - strength) / 400));
                    UUID table = UUID.randomUUID();
                    ranked.register(table, user, "standard", RankedService.LEAGUE, bot, "Target", at);
                    ranked.matchEnded(table, won ? 1 : 0, at);
                    CareerService.FinishedGame game = new CareerService.FinishedGame(table, CareerRewards.AI, 2, 7 + random.nextInt(5));
                    game.format = "standard";
                    game.aiSkill = bot.skill;
                    CareerService.Seat seat = new CareerService.Seat(user, won, CareerTally.forTest(table));
                    seat.matchWon = won;
                    game.seats.add(seat);
                    CareerService.get().gameEnded(game);
                    run.games++;
                    if (won) {
                        run.won++;
                    }
                }
                openPacks(user, deckId, setsOf, run);
                try {
                    league.craftMissing(user, deckId);
                    run.finished = d;
                } catch (CareerService.CareerException notYet) {
                    // not affordable yet
                }
            }
            LadderStore.Standing standing = ladder.standing(user, "standard", RankedService.LEAGUE, RankedService.SEASON);
            run.peak = standing == null ? 0 : standing.peak;
            return run;
        } finally {
            RandomUtil.install(before);
            RankedService.use(null);
            ladder.close();
        }
    }

    private void openPacks(String user, String deckId, Map<String, Set<String>> setsOf, Run run) throws Exception {
        CareerLeague league = CareerLeague.get();
        while (true) {
            CareerProfile profile = store.profile(user);
            CareerLeague.LeagueDeck view = league.deck(user, deckId);
            boolean token = store.leagueTokens(user, "standard") > 0 || profile.packTokens > 0;
            boolean spare = profile.coins >= CareerRules.PACK_PRICE + view.craft.coins;
            if (!token && !spare) {
                return;
            }
            // the set with the most rare and mythic copies still missing, else the most copies of any rarity
            Map<String, Integer> want = new HashMap<>();
            Map<String, Integer> owned = new HashMap<>();
            for (CareerCard card : store.collection(user)) {
                owned.merge(card.name.toLowerCase(java.util.Locale.ROOT), card.count, Integer::sum);
            }
            for (Map.Entry<String, int[]> need : needs(view.deck).entrySet()) {
                int lacking = Math.max(0, Math.min(CareerRules.PLAYSET, need.getValue()[0]) - owned.getOrDefault(need.getKey(), 0));
                if (lacking == 0) {
                    continue;
                }
                int weight = need.getValue()[1] >= 2 ? lacking * 10 : lacking;
                for (String set : setsOf.getOrDefault(need.getKey(), Collections.emptySet())) {
                    want.merge(set, weight, Integer::sum);
                }
            }
            String set = want.entrySet().stream().max(Map.Entry.<String, Integer>comparingByValue().thenComparing(Map.Entry.comparingByKey()))
                    .map(Map.Entry::getKey).orElse(league.sets("standard").get(0).setCode);
            CareerLeague.LeaguePack pack = league.openPack(user, "standard", set);
            run.packs++;
            if ("coins".equals(pack.paidWith)) {
                run.coinPacks++;
            }
        }
    }

    /** the deck's non-basic cards by lower-case name: copies, and 2 for rare or mythic, 0 otherwise */
    private static Map<String, int[]> needs(DeckCardLists deck) {
        Map<String, int[]> out = new HashMap<>();
        List<DeckCardInfo> all = new ArrayList<>(deck.getCards());
        all.addAll(deck.getSideboard());
        for (DeckCardInfo card : all) {
            if (CareerService.isBasic(card.getCardName())) {
                continue;
            }
            CardInfo info = CardRepository.instance.findCards(card.getCardName()).stream().findFirst().orElse(null);
            int rare = info == null ? 0 : (info.getRarity() == mage.constants.Rarity.RARE || info.getRarity() == mage.constants.Rarity.MYTHIC ? 2 : 0);
            out.computeIfAbsent(card.getCardName().toLowerCase(java.util.Locale.ROOT), k -> new int[]{0, rare})[0] += card.getAmount();
        }
        return out;
    }

    /** for each card of the deck, the League's Standard sets it is printed in */
    private static Map<String, Set<String>> setsOf(DeckCardLists deck, List<CareerLeague.LeagueSet> sets) {
        Set<String> codes = new HashSet<>();
        for (CareerLeague.LeagueSet set : sets) {
            codes.add(set.setCode);
        }
        Map<String, Set<String>> out = new HashMap<>();
        for (String name : needs(deck).keySet()) {
            Set<String> in = new HashSet<>();
            for (CardInfo info : CardRepository.instance.findCards(name)) {
                if (codes.contains(info.getSetCode())) {
                    in.add(info.getSetCode());
                }
            }
            out.put(name, in);
        }
        return out;
    }

    /**
     * The League's target (#433): a daily player (6 League games a day, at the ladder's middle) finishes a competitive
     * Standard deck in about four weeks. Measured 2026-11-19 with the wildcard track at every 4th pack a rare, every 12th a
     * mythic, on the calibrated roster with budget bots, three runs per deck: median 27 days (20 to 50; Jund Sacrifice
     * and Mono-Green Landfall, 35 to 50, are the slow ones). At strength 1200 the median is 37, at 1500 30; at 2 games
     * a day about 88, with many runs unfinished after 90 days. Without the track it was 75 days
     * to more than 120. Run with -Dxmage.progressionReport=true to print the sheet, and add
     * -Dxmage.leagueSheet=true for it at three strengths and two paces.
     */
    @Test
    public void aDailyLeaguePlayerFinishesAMetaDeckInAboutFourWeeks() throws Exception {
        boolean report = Boolean.getBoolean("xmage.progressionReport");
        StringBuilder sheet = new StringBuilder("games/day strength deck                              days  games  won%  packs coinPacks rares  peak\n");
        List<Integer> days = new ArrayList<>();
        int n = 0;
        boolean full = Boolean.getBoolean("xmage.leagueSheet");
        double[] strengths = full ? new double[]{1200, 1350, 1500} : new double[]{1350};
        int[] paces = full ? new int[]{2, 6} : new int[]{6};
        for (int pace : paces) {
            for (double strength : strengths) {
                for (String target : META) {
                  for (int seed = 0; seed < SEEDS; seed++) {
                    // the seed is the deck's and the try's, so a row draws the same whatever else the sheet runs
                    Run run = league("p" + (n++), target, pace, strength, target.hashCode() * 31L + seed);
                    if (pace == 6 && strength == 1350) {
                        days.add(run.finished < 0 ? Integer.MAX_VALUE : run.finished);
                    }
                    sheet.append(String.format("%9d %8.0f %-34s %4s %6d %5.0f %6d %9d %5d  %s%n", pace, strength, target,
                            run.finished < 0 ? ">" + LEAGUE_DAYS : String.valueOf(run.finished), run.games,
                            100.0 * run.won / Math.max(1, run.games), run.packs, run.coinPacks, run.rares, LadderRank.label(run.peak)));
                  }
                }
            }
        }
        if (report) {
            System.out.println(sheet);
        }
        Collections.sort(days);
        int median = days.get(days.size() / 2);
        assertTrue(median >= 21 && median <= 42, "a daily player's median days to a whole meta deck: " + days + "\n" + sheet);
    }
}
