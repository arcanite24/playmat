package mage.server.career;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import java.util.Arrays;
import java.util.Random;
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
}
